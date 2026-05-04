//! Security & Readiness features.
//!
//! Pure functions — no I/O. Computed from
//! [`SecurityRawData`](crate::collectors::security::SecurityRawData) by [`compute`].

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::collectors::security::SecurityRawData;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SecurityFeatures {
    /// `None` when Scorecard returned 404.
    pub scorecard_score: Option<f64>,
    /// Days since the Scorecard report was generated (`None` when no Scorecard).
    pub scorecard_age_days: Option<u64>,
    /// Names of Scorecard checks that scored < 5 (or had no data with score == -1).
    pub scorecard_checks_failed: Vec<String>,
    /// OSV count — always 0 in Day 2; Day 3 plugs in real data.
    pub osv_open_advisories: u64,
    pub has_security_md: bool,
    pub has_contributing_md: bool,
    pub has_code_of_conduct: bool,
    pub has_license: bool,
    pub has_codeowners: bool,
    pub has_ci_workflow: bool,
    /// `true` when every release tag matches `vX.Y.Z` or `X.Y.Z`.
    pub semver_consistent: bool,
    pub archived: bool,
}

#[must_use]
pub fn compute(raw: &SecurityRawData, now: OffsetDateTime) -> SecurityFeatures {
    let scorecard_score = raw.scorecard.as_ref().map(|r| r.score);
    let scorecard_age_days = raw
        .scorecard
        .as_ref()
        .map(|r| ((now - r.date).whole_days().max(0)) as u64);
    let scorecard_checks_failed = raw
        .scorecard
        .as_ref()
        .map(|r| {
            r.checks
                .iter()
                .filter(|c| c.score < 5)
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    SecurityFeatures {
        scorecard_score,
        scorecard_age_days,
        scorecard_checks_failed,
        osv_open_advisories: raw.osv_advisories.len() as u64,
        has_security_md: raw.has_security_md,
        has_contributing_md: raw.has_contributing_md,
        has_code_of_conduct: raw.has_code_of_conduct,
        has_license: raw.has_license,
        has_codeowners: raw.has_codeowners,
        has_ci_workflow: raw.has_ci_workflow,
        semver_consistent: semver_consistent(&raw.releases),
        archived: raw.archived,
    }
}

/// True when every non-draft release tag matches `vX.Y.Z` or `X.Y.Z`.
/// Returns **false** for repos with zero releases — no track record of
/// semver discipline yet (per `docs/day-5-polish.md` §2 decision A).
/// Scorer assigns the Neutral 50/100 sub-score in that case.
fn semver_consistent(releases: &[crate::api::github::ReleaseMeta]) -> bool {
    let mut any = false;
    for r in releases.iter().filter(|r| !r.draft) {
        any = true;
        if !is_semver_tag(&r.tag_name) {
            return false;
        }
    }
    any
}

fn is_semver_tag(tag: &str) -> bool {
    let trimmed = tag.strip_prefix('v').unwrap_or(tag);
    semver::Version::parse(trimmed).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_with_v_prefix_accepted() {
        assert!(is_semver_tag("v1.2.3"));
    }

    #[test]
    fn semver_without_prefix_accepted() {
        assert!(is_semver_tag("1.2.3"));
    }

    #[test]
    fn semver_prerelease_accepted() {
        assert!(is_semver_tag("1.2.3-rc.1"));
    }

    #[test]
    fn non_semver_rejected() {
        assert!(!is_semver_tag("release-2026-01"));
        assert!(!is_semver_tag("foo"));
        assert!(!is_semver_tag("1.2"));
    }
}
