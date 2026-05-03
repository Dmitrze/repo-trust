//! Star Authenticity features (Day 3 — shallow).
//!
//! Heuristic 1 (low-activity profile share, 9-signal composite) and
//! Heuristic 3 (fork/watcher ratios with ecosystem multipliers). Heuristic
//! 2 (lockstep timing z-score) lands Day 4.
//!
//! Pure functions — no I/O. Source of truth: `docs/methodology.md` §Module 1.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::{StargazerEntry, UserProfile};
use crate::collectors::stars::StarsRawData;

/// Per-module features produced from raw collected data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StarsFeatures {
    pub total_stars: u64,
    pub forks_count: u64,
    pub watchers_count: u64,
    pub fork_to_star_ratio: f64,
    pub watcher_to_star_ratio: f64,
    /// Share `[0.0, 1.0]` of sampled stargazers matching the 9-signal
    /// low-activity composite. `None` when no stargazers were sampled
    /// (Quick mode or below-floor repo).
    pub low_activity_share: Option<f64>,
    pub sample_size: usize,
    pub primary_language: Option<String>,
    pub repo_age_days: u64,
    pub archived: bool,
}

/// Convert raw collected data into normalized features.
#[must_use]
pub fn compute(raw: &StarsRawData, now: OffsetDateTime) -> StarsFeatures {
    let total = raw.repo_metadata.stargazers_count;
    let forks = raw.repo_metadata.forks_count;
    let watchers = raw.repo_metadata.watchers_count;

    let fork_to_star_ratio = if total > 0 {
        forks as f64 / total as f64
    } else {
        0.0
    };
    let watcher_to_star_ratio = if total > 0 {
        watchers as f64 / total as f64
    } else {
        0.0
    };

    let low_activity_share = if raw.sampled_profiles.is_empty() {
        None
    } else {
        let matches = raw
            .sampled_profiles
            .iter()
            .filter(|(s, p)| matches_low_activity_profile(s, p))
            .count();
        Some(matches as f64 / raw.sampled_profiles.len() as f64)
    };

    let repo_age_days = (now - raw.repo_metadata.created_at).whole_days().max(0) as u64;

    StarsFeatures {
        total_stars: total,
        forks_count: forks,
        watchers_count: watchers,
        fork_to_star_ratio: crate::utils::time::round6(fork_to_star_ratio),
        watcher_to_star_ratio: crate::utils::time::round6(watcher_to_star_ratio),
        low_activity_share: low_activity_share.map(crate::utils::time::round6),
        sample_size: raw.sampled_profiles.len(),
        primary_language: raw.repo_metadata.language.clone(),
        repo_age_days,
        archived: raw.repo_metadata.archived,
    }
}

/// 9-signal composite per `methodology.md` §Module 1, Heuristic 1.
///
/// All signals must hold for a profile to be flagged. The 9th signal
/// (`starred_at == created_at` same UTC day) only applies when the
/// stargazer entry carries a starred-at timestamp (REST `vnd.github.star+json`
/// Accept header). When unavailable we fall back to the 8-signal composite.
#[must_use]
pub fn matches_low_activity_profile(stargazer: &StargazerEntry, profile: &UserProfile) -> bool {
    // Signal 1: created after 2022-01-01 UTC.
    let cutoff = OffsetDateTime::from_unix_timestamp(1_640_995_200) // 2022-01-01T00:00:00Z
        .expect("static unix timestamp");
    if profile.created_at < cutoff {
        return false;
    }
    // Signals 2-3: ≤1 follower / following.
    if profile.followers > 1 || profile.following > 1 {
        return false;
    }
    // Signal 4: zero gists.
    if profile.public_gists > 0 {
        return false;
    }
    // Signal 5: ≤4 public repos.
    if profile.public_repos > 4 {
        return false;
    }
    // Signals 6-8: bio / blog / email empty.
    if non_empty(&profile.bio) || non_empty(&profile.blog) || non_empty(&profile.email) {
        return false;
    }
    // Signal 9 (when available): starred_at same UTC day as created_at.
    if let StargazerEntry::WithDate { starred_at, .. } = stargazer {
        let starred_day = starred_at.date();
        let created_day = profile.created_at.date();
        if starred_day != created_day {
            return false;
        }
    }
    true
}

fn non_empty(s: &Option<String>) -> bool {
    s.as_deref().map(|x| !x.trim().is_empty()).unwrap_or(false)
}

/// Ecosystem multipliers per `module-specs.md` §Star Authenticity.
/// Returns `(fork_multiplier, watcher_multiplier)` that ADJUST the healthy
/// thresholds (multiply the methodology baseline ratio).
#[must_use]
pub fn ecosystem_multipliers(language: Option<&str>) -> (f64, f64) {
    match language {
        Some("TypeScript") | Some("JavaScript") => (0.7, 0.8),
        Some("Python") => (1.0, 1.0),
        Some("Go") => (1.1, 1.0),
        Some("Rust") => (0.9, 0.9),
        Some("Java") | Some("Kotlin") => (1.0, 1.0),
        _ => (1.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::github::UserStub;

    fn ts(year: i32, month: u8, day: u8) -> OffsetDateTime {
        time::Date::from_calendar_date(year, month.try_into().unwrap(), day)
            .unwrap()
            .midnight()
            .assume_utc()
    }

    fn flagged_profile() -> UserProfile {
        UserProfile {
            login: "low_a".into(),
            created_at: ts(2024, 6, 1),
            followers: 0,
            following: 0,
            public_repos: 1,
            public_gists: 0,
            bio: None,
            blog: None,
            email: None,
            user_type: Some("User".into()),
        }
    }

    fn entry_with_date(login: &str, day: time::OffsetDateTime) -> StargazerEntry {
        StargazerEntry::WithDate {
            starred_at: day,
            user: UserStub {
                login: login.into(),
                user_type: Some("User".into()),
            },
        }
    }

    #[test]
    fn low_activity_profile_flagged_with_all_8_signals() {
        let profile = flagged_profile();
        let stargazer = StargazerEntry::Plain(UserStub {
            login: "low_a".into(),
            user_type: Some("User".into()),
        });
        assert!(matches_low_activity_profile(&stargazer, &profile));
    }

    #[test]
    fn pre_2022_account_not_flagged() {
        let mut p = flagged_profile();
        p.created_at = ts(2021, 12, 31);
        let s = StargazerEntry::Plain(UserStub {
            login: "x".into(),
            user_type: None,
        });
        assert!(!matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn high_followers_not_flagged() {
        let mut p = flagged_profile();
        p.followers = 5;
        let s = StargazerEntry::Plain(UserStub {
            login: "x".into(),
            user_type: None,
        });
        assert!(!matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn non_empty_bio_not_flagged() {
        let mut p = flagged_profile();
        p.bio = Some("hello".into());
        let s = StargazerEntry::Plain(UserStub {
            login: "x".into(),
            user_type: None,
        });
        assert!(!matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn whitespace_bio_treated_as_empty() {
        let mut p = flagged_profile();
        p.bio = Some("   ".into());
        let s = StargazerEntry::Plain(UserStub {
            login: "x".into(),
            user_type: None,
        });
        assert!(matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn starred_at_same_day_as_created_flagged() {
        let p = flagged_profile();
        let s = entry_with_date("low_a", ts(2024, 6, 1));
        assert!(matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn starred_at_different_day_not_flagged() {
        let p = flagged_profile();
        let s = entry_with_date("low_a", ts(2024, 6, 5));
        assert!(!matches_low_activity_profile(&s, &p));
    }

    #[test]
    fn ecosystem_multipliers_table() {
        assert_eq!(ecosystem_multipliers(Some("TypeScript")), (0.7, 0.8));
        assert_eq!(ecosystem_multipliers(Some("JavaScript")), (0.7, 0.8));
        assert_eq!(ecosystem_multipliers(Some("Go")), (1.1, 1.0));
        assert_eq!(ecosystem_multipliers(Some("Rust")), (0.9, 0.9));
        assert_eq!(ecosystem_multipliers(Some("Python")), (1.0, 1.0));
        assert_eq!(ecosystem_multipliers(None), (1.0, 1.0));
        assert_eq!(ecosystem_multipliers(Some("Cobol")), (1.0, 1.0));
    }
}
