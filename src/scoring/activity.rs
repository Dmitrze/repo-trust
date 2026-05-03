//! Activity Health scorer — pure function from features + thresholds to
//! `(ModuleResult, Vec<EvidenceItem>)`.
//!
//! Per `docs/methodology.md` §Module 2, the final module score is the
//! arithmetic mean of the per-sub-signal scores. Sub-signals whose input
//! is `None` (e.g. issues disabled, no releases) are dropped from the mean
//! and recorded in `missing_data`.

use std::collections::BTreeMap;

use serde_json::json;

use super::thresholds::{linear_higher_better, linear_lower_better, ActivityThresholds};
use crate::features::activity::ActivityFeatures;
use crate::models::{Confidence, EvidenceItem, ModuleResult, Verdict};

const MODULE_NAME: &str = "activity";

/// Score the activity module.
///
/// `repo_age_days` is the number of days between the repo's `created_at`
/// and the scan's `now`. Used to demote new repos to `Low` confidence per
/// `tests/scenarios/activity-health-module.md` S-101.
#[must_use]
pub fn score(
    features: &ActivityFeatures,
    thresholds: &ActivityThresholds,
    repo_age_days: u64,
) -> (ModuleResult, Vec<EvidenceItem>) {
    let mut sub_scores: BTreeMap<String, u8> = BTreeMap::new();
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // ─── 1. Commit recency ───────────────────────────────────────────────
    if let Some(days) = features.days_since_last_commit {
        let s = linear_lower_better(
            days as f64,
            thresholds.days_since_last_commit_full_credit,
            thresholds.days_since_last_commit_zero,
        );
        sub_scores.insert("commit_recency".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "days_since_last_commit".into(),
            label: "Days since last commit".into(),
            value: json!(days),
            threshold: Some(json!({
                "full_credit": thresholds.days_since_last_commit_full_credit,
                "zero": thresholds.days_since_last_commit_zero,
            })),
            verdict: verdict_from_score(s),
            rationale: format!(
                "Last commit {days} day(s) ago. Full credit at ≤{} days; zero at ≥{} days.",
                thresholds.days_since_last_commit_full_credit as u64,
                thresholds.days_since_last_commit_zero as u64,
            ),
        });
    } else {
        // No commits in the 18-month window is a *finding*, not missing data.
        // The API succeeded; the repo simply has no recent commits. Score 0
        // and surface as Concerning evidence; don't demote confidence.
        sub_scores.insert("commit_recency".into(), 0);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "no_commits_in_window".into(),
            label: "No commits in the last 18 months".into(),
            value: json!(0u64),
            threshold: None,
            verdict: Verdict::Concerning,
            rationale: "The 18-month commit window contained zero commits.".into(),
        });
    }

    // ─── 2. Commits in the last 90 days ──────────────────────────────────
    {
        let s = linear_higher_better(
            features.commits_last_90d as f64,
            thresholds.commits_last_90d_full_credit,
            thresholds.commits_last_90d_zero,
        );
        sub_scores.insert("commits_last_90d".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "commits_last_90d".into(),
            label: "Commits in the last 90 days".into(),
            value: json!(features.commits_last_90d),
            threshold: Some(json!({
                "full_credit": thresholds.commits_last_90d_full_credit,
                "zero": thresholds.commits_last_90d_zero,
            })),
            verdict: verdict_from_score(s),
            rationale: format!(
                "{} commit(s) in the last 90 days. Full credit at ≥{}.",
                features.commits_last_90d, thresholds.commits_last_90d_full_credit as u64,
            ),
        });
    }

    // ─── 3. Active contributors in the last 90 days ──────────────────────
    {
        let s = linear_higher_better(
            features.active_contributors_last_90d as f64,
            thresholds.active_contributors_full_credit,
            thresholds.active_contributors_zero,
        );
        sub_scores.insert("active_contributors_last_90d".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "active_contributors_last_90d".into(),
            label: "Active contributors in the last 90 days".into(),
            value: json!(features.active_contributors_last_90d),
            threshold: Some(json!({
                "full_credit": thresholds.active_contributors_full_credit,
                "zero": thresholds.active_contributors_zero,
            })),
            verdict: verdict_from_score(s),
            rationale: format!(
                "{} unique contributor(s) in the last 90 days. Full credit at ≥{}.",
                features.active_contributors_last_90d,
                thresholds.active_contributors_full_credit as u64,
            ),
        });
    }

    // ─── 4. Median issue first-response time (skip when issues disabled) ─
    if features.issues_enabled {
        if let Some(hours) = features.median_issue_first_response_hours {
            let s = linear_lower_better(
                hours,
                thresholds.median_issue_response_full_credit_hours,
                thresholds.median_issue_response_zero_hours,
            );
            sub_scores.insert("median_issue_response".into(), s);
            evidence.push(EvidenceItem {
                module: MODULE_NAME.into(),
                code: "median_issue_first_response_hours".into(),
                label: "Median issue first-response time (hours)".into(),
                value: json!(crate::utils::time::round6(hours)),
                threshold: Some(json!({
                    "full_credit": thresholds.median_issue_response_full_credit_hours,
                    "zero": thresholds.median_issue_response_zero_hours,
                })),
                verdict: verdict_from_score(s),
                rationale: format!(
                    "Median first-response time across recent issues with comments is ~{:.1}h.",
                    hours
                ),
            });
        } else {
            // Issues enabled but no qualifying issues in the 90-day window.
            missing.push("issues_no_responses".into());
        }
    } else {
        missing.push("issues_disabled".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "issues_disabled".into(),
            label: "Issues are disabled on this repository".into(),
            value: json!(false),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "GitHub Issues is turned off; the issue-response sub-score is dropped from the mean.".into(),
        });
    }

    // ─── 5. Days since last release (skip when no releases) ──────────────
    if let Some(days) = features.days_since_last_release {
        let s = linear_lower_better(
            days as f64,
            thresholds.days_since_last_release_full_credit,
            thresholds.days_since_last_release_zero,
        );
        sub_scores.insert("days_since_last_release".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "days_since_last_release".into(),
            label: "Days since last release".into(),
            value: json!(days),
            threshold: Some(json!({
                "full_credit": thresholds.days_since_last_release_full_credit,
                "zero": thresholds.days_since_last_release_zero,
            })),
            verdict: verdict_from_score(s),
            rationale: format!(
                "Latest release was {days} day(s) ago. Full credit at ≤{} days.",
                thresholds.days_since_last_release_full_credit as u64,
            ),
        });
    } else {
        missing.push("no_releases".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "no_releases".into(),
            label: "No published releases".into(),
            value: json!(null),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale:
                "Repository has no published releases; this sub-score is dropped from the mean."
                    .into(),
        });
    }

    // ─── Archived repos ──────────────────────────────────────────────────
    if features.archived {
        missing.push("archived".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "archived".into(),
            label: "Repository is archived".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Owner has archived this repository; activity signals are frozen.".into(),
        });
    }

    // ─── Final score = arithmetic mean of present sub-scores ─────────────
    let final_score = if sub_scores.is_empty() {
        0
    } else {
        let sum: u32 = sub_scores.values().map(|s| u32::from(*s)).sum();
        let n = sub_scores.len() as u32;
        ((sum + n / 2) / n) as u8
    };

    // ─── Confidence ──────────────────────────────────────────────────────
    let confidence = compute_confidence(features, &missing, repo_age_days, thresholds);

    if repo_age_days < thresholds.min_repo_age_for_high_confidence_days {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "repo_too_young".into(),
            label: "Repository is younger than the stable-baseline window".into(),
            value: json!(repo_age_days),
            threshold: Some(json!(thresholds.min_repo_age_for_high_confidence_days)),
            verdict: Verdict::Neutral,
            rationale: format!(
                "Repo created {repo_age_days} day(s) ago; activity baselines are lower confidence under {} days of history.",
                thresholds.min_repo_age_for_high_confidence_days
            ),
        });
    }

    (
        ModuleResult {
            module: MODULE_NAME.into(),
            score: final_score,
            confidence,
            sub_scores,
            sample_size: None,
            missing_data: missing,
        },
        evidence,
    )
}

fn verdict_from_score(s: u8) -> Verdict {
    match s {
        80..=100 => Verdict::Positive,
        50..=79 => Verdict::Neutral,
        20..=49 => Verdict::Concerning,
        _ => Verdict::HighRisk,
    }
}

fn compute_confidence(
    features: &ActivityFeatures,
    missing: &[String],
    repo_age_days: u64,
    thresholds: &ActivityThresholds,
) -> Confidence {
    if features.archived {
        return Confidence::Low;
    }
    if repo_age_days < thresholds.min_repo_age_for_high_confidence_days {
        return Confidence::Low;
    }
    // Anything in `missing` other than the benign "no_releases" / "issues_no_responses"
    // demotes confidence one band.
    let demoting_missing = missing
        .iter()
        .filter(|m| m.as_str() != "no_releases" && m.as_str() != "issues_no_responses")
        .count();
    if demoting_missing == 0 {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::activity::ActivityFeatures;

    fn baseline_features() -> ActivityFeatures {
        ActivityFeatures {
            commits_last_30d: 0,
            commits_last_90d: 0,
            commits_last_365d: 0,
            days_since_last_commit: None,
            days_since_last_release: None,
            release_count_last_year: 0,
            median_issue_first_response_hours: None,
            median_pr_review_hours: None,
            active_contributors_last_90d: 0,
            commit_count_variance_18m: 0.0,
            archived: false,
            issues_enabled: true,
        }
    }

    #[test]
    fn inactive_repo_scores_low() {
        // octocat/Hello-World shape: ancient last commit, no recent activity.
        let mut f = baseline_features();
        f.days_since_last_commit = Some(5_000);
        f.commits_last_90d = 0;
        f.active_contributors_last_90d = 0;
        let (r, ev) = score(&f, &ActivityThresholds::v1(), 5_000);
        assert!(
            r.score <= 10,
            "score {} should be ≤10 for dead repo",
            r.score
        );
        assert_eq!(r.module, "activity");
        assert!(ev.len() >= 3, "≥3 evidence items required");
        // confidence: archived=false, age >> 180d, missing has "no_releases"
        // which is benign → still High
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn very_active_repo_scores_high() {
        let mut f = baseline_features();
        f.days_since_last_commit = Some(1);
        f.commits_last_90d = 200;
        f.active_contributors_last_90d = 30;
        f.median_issue_first_response_hours = Some(10.0);
        f.days_since_last_release = Some(7);
        let (r, ev) = score(&f, &ActivityThresholds::v1(), 365 * 5);
        assert!(r.score >= 95, "score {} should be ≥95", r.score);
        assert_eq!(r.confidence, Confidence::High);
        assert!(
            ev.iter()
                .filter(|e| matches!(e.verdict, Verdict::Positive))
                .count()
                >= 3
        );
    }

    #[test]
    fn archived_repo_demotes_to_low_confidence() {
        let mut f = baseline_features();
        f.archived = true;
        f.days_since_last_commit = Some(800);
        let (r, _ev) = score(&f, &ActivityThresholds::v1(), 365 * 3);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "archived"));
    }

    #[test]
    fn young_repo_demotes_to_low_confidence() {
        let mut f = baseline_features();
        f.days_since_last_commit = Some(2);
        f.commits_last_90d = 10;
        f.active_contributors_last_90d = 2;
        let (r, ev) = score(&f, &ActivityThresholds::v1(), 30);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(ev.iter().any(|e| e.code == "repo_too_young"));
    }

    #[test]
    fn issues_disabled_drops_subscore_and_adds_missing() {
        let mut f = baseline_features();
        f.days_since_last_commit = Some(10);
        f.commits_last_90d = 50;
        f.active_contributors_last_90d = 5;
        f.issues_enabled = false;
        let (r, _ev) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        assert!(r.missing_data.iter().any(|m| m == "issues_disabled"));
        assert!(!r.sub_scores.contains_key("median_issue_response"));
        assert_eq!(r.confidence, Confidence::Medium);
    }

    #[test]
    fn no_releases_drops_subscore_but_keeps_high_confidence() {
        let mut f = baseline_features();
        f.days_since_last_commit = Some(5);
        f.commits_last_90d = 50;
        f.active_contributors_last_90d = 5;
        f.median_issue_first_response_hours = Some(20.0);
        f.days_since_last_release = None;
        let (r, _ev) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        assert!(r.missing_data.iter().any(|m| m == "no_releases"));
        assert!(!r.sub_scores.contains_key("days_since_last_release"));
        // no_releases is benign — confidence stays High.
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn final_score_is_arithmetic_mean_of_subscores() {
        let mut f = baseline_features();
        // Pin every sub-score so we can predict the mean.
        f.days_since_last_commit = Some(14); // 100
        f.commits_last_90d = 30; // 100
        f.active_contributors_last_90d = 4; // 100
        f.median_issue_first_response_hours = Some(48.0); // 100
        f.days_since_last_release = Some(90); // 100
        let (r, _) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        assert_eq!(r.score, 100);
        assert_eq!(r.sub_scores.len(), 5);
    }

    #[test]
    fn evidence_codes_are_unique() {
        let mut f = baseline_features();
        f.days_since_last_commit = Some(50);
        f.commits_last_90d = 10;
        f.active_contributors_last_90d = 2;
        f.median_issue_first_response_hours = Some(100.0);
        f.days_since_last_release = Some(200);
        let (_, ev) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        let mut codes: Vec<&str> = ev.iter().map(|e| e.code.as_str()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), ev.len(), "evidence codes should be unique");
    }

    #[test]
    fn module_result_carries_module_name() {
        let f = baseline_features();
        let (r, _) = score(&f, &ActivityThresholds::v1(), 365);
        assert_eq!(r.module, "activity");
    }

    #[test]
    fn empty_repo_emits_concerning_evidence_for_no_commits() {
        let f = baseline_features();
        let (r, ev) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        assert!(ev
            .iter()
            .any(|e| e.code == "no_commits_in_window" && matches!(e.verdict, Verdict::Concerning)));
        // "No commits" is a finding, not missing data — confidence stays High.
        assert!(!r.missing_data.iter().any(|m| m == "commits"));
    }

    #[test]
    fn high_active_contributors_caps_at_100() {
        let mut f = baseline_features();
        f.active_contributors_last_90d = 1000;
        let (_, ev) = score(&f, &ActivityThresholds::v1(), 365 * 2);
        let item = ev
            .iter()
            .find(|e| e.code == "active_contributors_last_90d")
            .unwrap();
        assert!(matches!(item.verdict, Verdict::Positive));
    }
}
