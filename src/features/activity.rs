//! Activity Health features — normalized inputs to the activity scorer.
//!
//! Computed from [`ActivityRawData`](crate::collectors::activity::ActivityRawData)
//! by [`compute`]. Pure function — no I/O, deterministic given inputs.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::{CommitMeta, IssueMeta, PullMeta};
use crate::collectors::activity::ActivityRawData;

/// Per-module features produced from raw collected data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ActivityFeatures {
    pub commits_last_30d: u64,
    pub commits_last_90d: u64,
    pub commits_last_365d: u64,
    /// `None` when the repo has zero commits in the 18-month window.
    pub days_since_last_commit: Option<u64>,
    /// `None` when the repo has no published releases.
    pub days_since_last_release: Option<u64>,
    pub release_count_last_year: u64,
    /// `None` when issues are disabled or no issues in the window.
    pub median_issue_first_response_hours: Option<f64>,
    /// `None` when no PRs in the window.
    pub median_pr_review_hours: Option<f64>,
    pub active_contributors_last_90d: u64,
    /// Population variance (not sample) of monthly commit counts over the
    /// 18-month window. Reported as evidence; not currently in the score
    /// per `specs/activity-health-module.md` §Out of scope.
    pub commit_count_variance_18m: f64,
    /// `true` when the repo's metadata had `archived: true` — modules
    /// surface this as a top-level caveat.
    pub archived: bool,
    /// `true` when issues are disabled on the repo (drops the issue
    /// sub-score from the scorer per scenarios S-103).
    pub issues_enabled: bool,
}

/// Convert raw collected data into normalized features.
///
/// `now` is parameterized for determinism (so tests can pin a fixed
/// "current time"). In production the orchestrator passes
/// `RepositoryContext::snapshot_at`.
#[must_use]
pub fn compute(raw: &ActivityRawData, now: OffsetDateTime) -> ActivityFeatures {
    let commits_30d_cutoff = now - time::Duration::days(30);
    let commits_90d_cutoff = now - time::Duration::days(90);
    let commits_365d_cutoff = now - time::Duration::days(365);

    let commits_last_30d = count_commits_after(&raw.commits_18m, commits_30d_cutoff);
    let commits_last_90d = count_commits_after(&raw.commits_18m, commits_90d_cutoff);
    let commits_last_365d = count_commits_after(&raw.commits_18m, commits_365d_cutoff);

    let days_since_last_commit = raw
        .commits_18m
        .iter()
        .map(|c| c.commit.author.date)
        .max()
        .map(|latest| ((now - latest).whole_days().max(0)) as u64);

    let last_release = raw
        .releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| r.published_at.or(Some(r.created_at)))
        .max();
    let days_since_last_release = last_release.map(|t| ((now - t).whole_days().max(0)) as u64);
    let release_count_last_year = raw
        .releases
        .iter()
        .filter(|r| !r.draft)
        .filter(|r| {
            r.published_at
                .or(Some(r.created_at))
                .is_some_and(|t| t >= commits_365d_cutoff)
        })
        .count() as u64;

    let median_issue_first_response_hours = if raw.issues_enabled {
        median_issue_first_response_hours(&raw.issues_90d)
    } else {
        None
    };

    let median_pr_review_hours = median_pr_review_hours(&raw.prs_90d);

    let active_contributors_last_90d =
        unique_contributors_after(&raw.commits_18m, commits_90d_cutoff).len() as u64;

    let commit_count_variance_18m = monthly_variance(&raw.commits_18m, now);

    ActivityFeatures {
        commits_last_30d,
        commits_last_90d,
        commits_last_365d,
        days_since_last_commit,
        days_since_last_release,
        release_count_last_year,
        median_issue_first_response_hours,
        median_pr_review_hours,
        active_contributors_last_90d,
        commit_count_variance_18m,
        archived: raw.archived,
        issues_enabled: raw.issues_enabled,
    }
}

fn count_commits_after(commits: &[CommitMeta], cutoff: OffsetDateTime) -> u64 {
    commits
        .iter()
        .filter(|c| c.commit.author.date >= cutoff)
        .count() as u64
}

fn unique_contributors_after(commits: &[CommitMeta], cutoff: OffsetDateTime) -> HashSet<&str> {
    commits
        .iter()
        .filter(|c| c.commit.author.date >= cutoff)
        .filter_map(|c| {
            c.author
                .as_ref()
                .map(|u| u.login.as_str())
                .or(Some(c.commit.author.name.as_str()))
        })
        .collect()
}

/// Median first-response time, in hours, computed from
/// `(updated_at - created_at)` where the issue has at least one comment.
/// We don't have a per-comment endpoint in Phase 1; this is a deliberate
/// approximation surfaced in the evidence rationale.
fn median_issue_first_response_hours(issues: &[IssueMeta]) -> Option<f64> {
    let mut hours: Vec<f64> = issues
        .iter()
        .filter(|i| i.comments > 0)
        .map(|i| (i.updated_at - i.created_at).as_seconds_f64() / 3600.0)
        .filter(|h| *h >= 0.0)
        .collect();
    median(&mut hours)
}

/// Median PR review latency, approximated by `(merged_at | closed_at) -
/// created_at` in hours.
fn median_pr_review_hours(prs: &[PullMeta]) -> Option<f64> {
    let mut hours: Vec<f64> = prs
        .iter()
        .filter_map(|p| p.merged_at.or(p.closed_at).map(|end| end - p.created_at))
        .map(|d| d.as_seconds_f64() / 3600.0)
        .filter(|h| *h >= 0.0)
        .collect();
    median(&mut hours)
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

/// Population variance of monthly commit counts over the 18-month window.
fn monthly_variance(commits: &[CommitMeta], now: OffsetDateTime) -> f64 {
    let mut buckets = [0u64; 18];
    for c in commits {
        let age_days = (now - c.commit.author.date).whole_days().max(0);
        let month = (age_days / 30) as usize;
        if month < buckets.len() {
            buckets[month] += 1;
        }
    }
    let n = buckets.len() as f64;
    let mean = buckets.iter().map(|x| *x as f64).sum::<f64>() / n;
    let var = buckets
        .iter()
        .map(|x| {
            let d = *x as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    crate::utils::time::round6(var)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(year: i32, month: u8, day: u8) -> OffsetDateTime {
        time::Date::from_calendar_date(year, month.try_into().unwrap(), day)
            .unwrap()
            .midnight()
            .assume_utc()
    }

    fn commit(year: i32, month: u8, day: u8, author: &str) -> CommitMeta {
        CommitMeta {
            sha: format!("sha-{author}-{year}{month:02}{day:02}"),
            commit: crate::api::github::CommitDetails {
                author: crate::api::github::AuthorTimestamp {
                    name: author.to_string(),
                    email: None,
                    date: ts(year, month, day),
                },
                message: "msg".into(),
            },
            author: Some(crate::api::github::UserStub {
                login: author.to_string(),
                user_type: Some("User".into()),
            }),
        }
    }

    #[test]
    fn empty_repo_has_no_last_commit() {
        let raw = ActivityRawData {
            commits_18m: vec![],
            issues_90d: vec![],
            prs_90d: vec![],
            releases: vec![],
            archived: false,
            issues_enabled: true,
        };
        let f = compute(&raw, ts(2026, 5, 3));
        assert_eq!(f.commits_last_30d, 0);
        assert!(f.days_since_last_commit.is_none());
        assert!(f.days_since_last_release.is_none());
        assert_eq!(f.active_contributors_last_90d, 0);
    }

    #[test]
    fn windows_count_correctly() {
        let now = ts(2026, 5, 3);
        let raw = ActivityRawData {
            commits_18m: vec![
                commit(2026, 5, 1, "alice"), // 2 days ago
                commit(2026, 4, 1, "bob"),   // 32 days
                commit(2026, 1, 1, "alice"), // 122 days
                commit(2025, 6, 1, "carol"), // 336 days
            ],
            issues_90d: vec![],
            prs_90d: vec![],
            releases: vec![],
            archived: false,
            issues_enabled: true,
        };
        let f = compute(&raw, now);
        assert_eq!(f.commits_last_30d, 1);
        assert_eq!(f.commits_last_90d, 2);
        assert_eq!(f.commits_last_365d, 4);
        assert_eq!(f.days_since_last_commit, Some(2));
        assert_eq!(f.active_contributors_last_90d, 2); // alice, bob
    }

    #[test]
    fn issues_disabled_drops_response_time() {
        let now = ts(2026, 5, 3);
        let raw = ActivityRawData {
            commits_18m: vec![],
            issues_90d: vec![],
            prs_90d: vec![],
            releases: vec![],
            archived: false,
            issues_enabled: false,
        };
        let f = compute(&raw, now);
        assert!(f.median_issue_first_response_hours.is_none());
        assert!(!f.issues_enabled);
    }
}
