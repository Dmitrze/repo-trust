//! Maintainer Health features — Gini, bus-factor proxy, retention, bot filter.
//!
//! Pure functions — no I/O. Computed from
//! [`MaintainersRawData`](crate::collectors::maintainers::MaintainersRawData)
//! by [`compute`].

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::CommitMeta;
use crate::collectors::maintainers::MaintainersRawData;

/// Per-module features produced from raw collected data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MaintainerFeatures {
    /// Distinct human (non-bot) authors with ≥1 commit in last 365 days.
    pub active_maintainers_last_year: u64,
    /// Gini coefficient of commits-per-author (last 365d, bots excluded).
    pub commit_gini: f64,
    /// Bus-factor proxy: minimum authors needed to cover 50% of last-365d commits.
    pub bus_factor_proxy: u64,
    /// % of contributors active in BOTH 180-day windows that are active in
    /// either. Range `[0.0, 1.0]`.
    pub contributor_retention_rate: f64,
    /// Total contributors as reported by GitHub (anon-excluded). Surfaces
    /// as evidence; not directly scored.
    pub total_contributors: u64,
    pub has_codeowners: bool,
    pub has_maintainers_md: bool,
    pub has_governance_doc: bool,
    pub archived: bool,
    /// Top 5 authors by commit count for evidence display.
    pub top_authors: Vec<AuthorCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorCount {
    pub login: String,
    pub commits: u64,
}

/// Convert raw collected data into normalized features.
#[must_use]
pub fn compute(raw: &MaintainersRawData, now: OffsetDateTime) -> MaintainerFeatures {
    let cutoff_365d = now - time::Duration::days(365);
    let cutoff_180d_first = now - time::Duration::days(360);
    let cutoff_180d_second = now - time::Duration::days(180);

    // Filter bot commits + slice to 365d.
    let commits_365d: Vec<&CommitMeta> = raw
        .commits_18m
        .iter()
        .filter(|c| c.commit.author.date >= cutoff_365d)
        .filter(|c| {
            c.author
                .as_ref()
                .map_or(true, |u| !is_bot(&u.login, u.user_type.as_deref()))
        })
        .collect();

    // commits-by-author map (sorted by login for determinism).
    let mut by_author: BTreeMap<&str, u64> = BTreeMap::new();
    for c in &commits_365d {
        let login = c
            .author
            .as_ref()
            .map_or(c.commit.author.name.as_str(), |u| u.login.as_str());
        *by_author.entry(login).or_insert(0) += 1;
    }
    let counts: Vec<u64> = by_author.values().copied().collect();

    let active_maintainers_last_year = by_author.len() as u64;
    let commit_gini = gini(&counts);
    let bus_factor_proxy = bus_factor(&counts);

    // Top 5 authors by commit count, descending.
    let mut authors_sorted: Vec<(&str, u64)> = by_author.iter().map(|(k, v)| (*k, *v)).collect();
    authors_sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let top_authors: Vec<AuthorCount> = authors_sorted
        .into_iter()
        .take(5)
        .map(|(login, commits)| AuthorCount {
            login: login.to_string(),
            commits,
        })
        .collect();

    // Two non-overlapping 180d windows for retention.
    // First window: 360d → 180d ago. Second window: 180d → now.
    let first_window: HashSet<&str> = commits_365d
        .iter()
        .filter(|c| {
            let d = c.commit.author.date;
            d >= cutoff_180d_first && d < cutoff_180d_second
        })
        .filter_map(|c| c.author.as_ref().map(|u| u.login.as_str()))
        .collect();
    let second_window: HashSet<&str> = commits_365d
        .iter()
        .filter(|c| c.commit.author.date >= cutoff_180d_second)
        .filter_map(|c| c.author.as_ref().map(|u| u.login.as_str()))
        .collect();
    let contributor_retention_rate = retention(&first_window, &second_window);

    let total_contributors = raw
        .contributors
        .iter()
        .filter(|c| !is_bot(&c.login, c.user_type.as_deref()))
        .count() as u64;

    MaintainerFeatures {
        active_maintainers_last_year,
        commit_gini: crate::utils::time::round6(commit_gini),
        bus_factor_proxy,
        contributor_retention_rate: crate::utils::time::round6(contributor_retention_rate),
        total_contributors,
        has_codeowners: raw.has_codeowners,
        has_maintainers_md: raw.has_maintainers_md,
        has_governance_doc: raw.has_governance_doc,
        archived: raw.archived,
        top_authors,
    }
}

/// Bot detection. Filters by GitHub `type == "Bot"` AND username heuristic.
/// Patterns: `[bot]` suffix (the GitHub-app convention), and `*-bot` (the
/// community-bot convention).
#[must_use]
pub fn is_bot(login: &str, user_type: Option<&str>) -> bool {
    if user_type.is_some_and(|t| t.eq_ignore_ascii_case("Bot")) {
        return true;
    }
    let l = login.to_lowercase();
    if l.ends_with("[bot]") {
        return true;
    }
    if l.ends_with("-bot") || l == "github-actions" {
        return true;
    }
    matches!(l.as_str(), "dependabot" | "renovate" | "mergify")
}

/// Gini coefficient of a non-negative integer distribution. Returns 0 for
/// empty / all-zero / single-author inputs. Range `[0.0, 1.0)`.
#[must_use]
pub fn gini(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let total: u64 = values.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let mut sorted: Vec<u64> = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len() as i64;
    let mut weighted: i128 = 0;
    for (i, &v) in sorted.iter().enumerate() {
        let coef = 2 * (i as i64 + 1) - n - 1;
        weighted += i128::from(coef) * i128::from(v);
    }
    let g = weighted as f64 / (n as f64 * total as f64);
    g.clamp(0.0, 1.0)
}

/// Bus-factor proxy: minimum number of authors needed to cover ≥50% of total commits.
#[must_use]
pub fn bus_factor(values: &[u64]) -> u64 {
    let total: u64 = values.iter().sum();
    if total == 0 {
        return 0;
    }
    let half = total.div_ceil(2);
    let mut sorted: Vec<u64> = values.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a)); // descending
    let mut accumulated = 0u64;
    for (i, &v) in sorted.iter().enumerate() {
        accumulated += v;
        if accumulated >= half {
            return (i + 1) as u64;
        }
    }
    sorted.len() as u64
}

/// Contributor retention rate: |intersection| / |union|.
fn retention(first: &HashSet<&str>, second: &HashSet<&str>) -> f64 {
    let union: usize = first.union(second).count();
    if union == 0 {
        return 0.0;
    }
    let intersection: usize = first.intersection(second).count();
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_filter_excludes_bracket_bot_suffix() {
        assert!(is_bot("dependabot[bot]", None));
        assert!(is_bot("renovate[bot]", Some("Bot")));
        assert!(is_bot("github-actions[bot]", None));
    }

    #[test]
    fn bot_filter_excludes_user_type_bot() {
        assert!(is_bot("notabot", Some("Bot")));
        assert!(is_bot("notabot", Some("bot")));
    }

    #[test]
    fn bot_filter_excludes_dash_bot_suffix() {
        assert!(is_bot("ci-bot", None));
        assert!(is_bot("My-Bot", None));
    }

    #[test]
    fn bot_filter_keeps_humans() {
        assert!(!is_bot("alice", Some("User")));
        assert!(!is_bot("octocat", None));
        assert!(!is_bot("contributor42", Some("User")));
    }

    #[test]
    fn gini_zero_for_equal_distribution() {
        assert!(gini(&[5, 5, 5, 5]).abs() < 1e-9);
        assert!(gini(&[1, 1]).abs() < 1e-9);
    }

    #[test]
    fn gini_zero_for_empty_or_all_zero() {
        assert_eq!(gini(&[]), 0.0);
        assert_eq!(gini(&[0, 0, 0]), 0.0);
    }

    #[test]
    fn gini_high_for_concentrated() {
        let g = gini(&[0, 0, 0, 100]);
        assert!(g > 0.7, "expected gini > 0.7 for [0,0,0,100], got {g}");
    }

    #[test]
    fn gini_bounded_zero_to_one() {
        let g = gini(&[1, 2, 3, 4, 5, 100, 1000, 10_000]);
        assert!((0.0..=1.0).contains(&g));
    }

    #[test]
    fn bus_factor_solo_author_is_1() {
        assert_eq!(bus_factor(&[100]), 1);
    }

    #[test]
    fn bus_factor_balanced_5_authors_is_3() {
        // 5 authors with 20 each → need 3 to hit 60% (>= 50%).
        assert_eq!(bus_factor(&[20, 20, 20, 20, 20]), 3);
    }

    #[test]
    fn bus_factor_top_heavy_returns_1() {
        // 1 author 80%, 4 authors 5% each → top alone covers 80% ≥ 50%.
        assert_eq!(bus_factor(&[80, 5, 5, 5, 5]), 1);
    }

    #[test]
    fn retention_full_overlap() {
        let mut a = HashSet::new();
        a.insert("alice");
        a.insert("bob");
        let mut b = HashSet::new();
        b.insert("alice");
        b.insert("bob");
        assert!((retention(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn retention_no_overlap() {
        let mut a = HashSet::new();
        a.insert("alice");
        let mut b = HashSet::new();
        b.insert("bob");
        assert!(retention(&a, &b).abs() < 1e-9);
    }
}
