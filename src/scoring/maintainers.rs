//! Maintainer Health scorer — pure function from features → `(ModuleResult, evidence)`.
//!
//! Per `docs/methodology.md` §Module 3, the final score is the arithmetic
//! mean of the per-sub-signal scores. Sub-scores: bus-factor proxy,
//! commit concentration (Gini), retention rate, governance documents.
//!
//! Solo-maintainer is surfaced as `Concerning` evidence — never `HighRisk`
//! by itself per `module-specs.md` §Maintainer Health: "many excellent OSS
//! projects are solo-maintained".

use std::collections::BTreeMap;

use serde_json::json;

use super::thresholds::{linear_higher_better, linear_lower_better, MaintainerThresholds};
use crate::features::maintainers::MaintainerFeatures;
use crate::models::{Confidence, EvidenceItem, ModuleResult, Verdict};

const MODULE_NAME: &str = "maintainers";

/// Score the maintainer module.
///
/// `repo_age_days` demotes confidence to `Low` for repos younger than the
/// threshold (`MaintainerThresholds.min_repo_age_for_high_confidence_days`).
#[must_use]
pub fn score(
    features: &MaintainerFeatures,
    thresholds: &MaintainerThresholds,
    repo_age_days: u64,
) -> (ModuleResult, Vec<EvidenceItem>) {
    let mut sub_scores: BTreeMap<String, u8> = BTreeMap::new();
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // ─── 1. Bus-factor proxy ─────────────────────────────────────────────
    let bus_score = bus_factor_to_score(features.bus_factor_proxy, thresholds);
    sub_scores.insert("bus_factor_proxy".into(), bus_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "bus_factor_proxy".into(),
        label: "Bus-factor proxy (authors covering 50% of commits)".into(),
        value: json!(features.bus_factor_proxy),
        threshold: Some(json!({"healthy": thresholds.bus_factor_full_credit})),
        verdict: verdict_from_score(bus_score),
        rationale: format!(
            "{} author(s) needed to cover 50% of last-365d commits. Full credit at ≥{}.",
            features.bus_factor_proxy, thresholds.bus_factor_full_credit
        ),
    });

    // ─── 2. Commit concentration (Gini, lower better) ───────────────────
    let gini_score = linear_lower_better(
        features.commit_gini,
        thresholds.gini_full_credit,
        thresholds.gini_zero,
    );
    sub_scores.insert("commit_concentration".into(), gini_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "commit_gini".into(),
        label: "Commit concentration (Gini coefficient)".into(),
        value: json!(crate::utils::time::round6(features.commit_gini)),
        threshold: Some(json!({
            "full_credit": thresholds.gini_full_credit,
            "zero": thresholds.gini_zero,
        })),
        verdict: verdict_from_score(gini_score),
        rationale: format!(
            "Gini = {:.3}. ≤{:.2} is a balanced multi-maintainer distribution; ≥{:.2} indicates concentration.",
            features.commit_gini, thresholds.gini_full_credit, thresholds.gini_zero,
        ),
    });

    // ─── 3. Contributor retention rate ──────────────────────────────────
    let retention_score = linear_higher_better(
        features.contributor_retention_rate,
        thresholds.retention_full_credit,
        thresholds.retention_zero,
    );
    sub_scores.insert("contributor_retention".into(), retention_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "contributor_retention".into(),
        label: "Contributor retention rate (cross-180d-window overlap)".into(),
        value: json!(crate::utils::time::round6(
            features.contributor_retention_rate
        )),
        threshold: Some(json!({
            "full_credit": thresholds.retention_full_credit,
            "zero": thresholds.retention_zero,
        })),
        verdict: verdict_from_score(retention_score),
        rationale: format!(
            "Retention = {:.0}%. ≥{:.0}% is healthy; ≤{:.0}% indicates one-off contributors.",
            features.contributor_retention_rate * 100.0,
            thresholds.retention_full_credit * 100.0,
            thresholds.retention_zero * 100.0,
        ),
    });

    // ─── 4. Governance documents ────────────────────────────────────────
    let governance_score = governance_score_from(
        features.has_codeowners,
        features.has_maintainers_md || features.has_governance_doc,
    );
    sub_scores.insert("governance_docs".into(), governance_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "governance_docs".into(),
        label: "Presence of governance documents".into(),
        value: json!({
            "has_codeowners": features.has_codeowners,
            "has_maintainers_md": features.has_maintainers_md,
            "has_governance_doc": features.has_governance_doc,
        }),
        threshold: None,
        verdict: verdict_from_score(governance_score),
        rationale: "Looking for CODEOWNERS plus either MAINTAINERS.md or GOVERNANCE.md.".into(),
    });

    // ─── Solo-maintainer evidence (Concerning, NOT HighRisk standalone) ─
    if features.active_maintainers_last_year == 1 {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "solo_maintainer".into(),
            label: "Solo maintainer (1 active human author in last 365d)".into(),
            value: json!(features.active_maintainers_last_year),
            threshold: None,
            verdict: Verdict::Concerning,
            rationale: "Only one human author committed in the last year. Many excellent OSS projects are solo-maintained; this is a sustainability flag, not a disqualifier.".into(),
        });
    }

    // ─── Top authors evidence ───────────────────────────────────────────
    if !features.top_authors.is_empty() {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "top_authors".into(),
            label: "Top 5 authors by commit count (last 365d, bots excluded)".into(),
            value: json!(features.top_authors),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: format!(
                "{} active human author(s) over the last 365 days.",
                features.active_maintainers_last_year
            ),
        });
    }

    // ─── Archived ────────────────────────────────────────────────────────
    if features.archived {
        missing.push("archived".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "archived".into(),
            label: "Repository is archived".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Owner has archived this repository; maintainer signals are frozen.".into(),
        });
    }

    // ─── Final score = arithmetic mean of sub-scores ────────────────────
    let final_score = if sub_scores.is_empty() {
        0
    } else {
        let sum: u32 = sub_scores.values().map(|s| u32::from(*s)).sum();
        let n = sub_scores.len() as u32;
        ((sum + n / 2) / n) as u8
    };

    let confidence = compute_confidence(features, repo_age_days, thresholds);

    if repo_age_days < thresholds.min_repo_age_for_high_confidence_days {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "repo_too_young".into(),
            label: "Repository is younger than the stable-baseline window".into(),
            value: json!(repo_age_days),
            threshold: Some(json!(thresholds.min_repo_age_for_high_confidence_days)),
            verdict: Verdict::Neutral,
            rationale: format!(
                "Repo created {repo_age_days} day(s) ago; maintainer baselines are lower confidence under {} days of history.",
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

fn bus_factor_to_score(value: u64, thresholds: &MaintainerThresholds) -> u8 {
    // Discrete table from methodology.md §Module 3.
    // ≥ full_credit → 100; below floors per integer step.
    let v = value as f64;
    linear_higher_better(v, thresholds.bus_factor_full_credit as f64, 0.0)
}

fn governance_score_from(has_codeowners: bool, has_maintainers_or_governance: bool) -> u8 {
    let mut s = 50u8; // neutral baseline — absence is not concerning per spec
    if has_codeowners {
        s = s.saturating_add(30);
    }
    if has_maintainers_or_governance {
        s = s.saturating_add(20);
    }
    s.min(100)
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
    features: &MaintainerFeatures,
    repo_age_days: u64,
    thresholds: &MaintainerThresholds,
) -> Confidence {
    if features.archived {
        return Confidence::Low;
    }
    if repo_age_days < thresholds.min_repo_age_for_high_confidence_days {
        return Confidence::Low;
    }
    // If we have very few commits to compute over, drop one band.
    if features.active_maintainers_last_year == 0 {
        return Confidence::Low;
    }
    if features.active_maintainers_last_year < 3 {
        return Confidence::Medium;
    }
    Confidence::High
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::maintainers::{AuthorCount, MaintainerFeatures};

    fn baseline() -> MaintainerFeatures {
        MaintainerFeatures::default()
    }

    #[test]
    fn solo_maintainer_emits_concerning_not_highrisk() {
        let mut f = baseline();
        f.active_maintainers_last_year = 1;
        f.bus_factor_proxy = 1;
        f.commit_gini = 0.0;
        f.top_authors = vec![AuthorCount {
            login: "alice".into(),
            commits: 100,
        }];
        let (_, ev) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        let solo = ev
            .iter()
            .find(|e| e.code == "solo_maintainer")
            .expect("solo evidence");
        assert!(matches!(solo.verdict, Verdict::Concerning));
        assert!(!matches!(solo.verdict, Verdict::HighRisk));
    }

    #[test]
    fn healthy_multi_maintainer_scores_high() {
        let mut f = baseline();
        f.active_maintainers_last_year = 8;
        f.bus_factor_proxy = 5;
        f.commit_gini = 0.30;
        f.contributor_retention_rate = 0.7;
        f.has_codeowners = true;
        f.has_governance_doc = true;
        let (r, _ev) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        assert!(r.score >= 80, "expected ≥80, got {}", r.score);
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn high_concentration_lowers_score() {
        let mut f = baseline();
        f.active_maintainers_last_year = 3;
        f.bus_factor_proxy = 1;
        f.commit_gini = 0.85;
        f.contributor_retention_rate = 0.1;
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        assert!(r.score < 50, "expected <50, got {}", r.score);
    }

    #[test]
    fn archived_demotes_to_low_confidence() {
        let mut f = baseline();
        f.active_maintainers_last_year = 4;
        f.archived = true;
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "archived"));
    }

    #[test]
    fn young_repo_gets_low_confidence() {
        let mut f = baseline();
        f.active_maintainers_last_year = 4;
        f.bus_factor_proxy = 3;
        let (r, ev) = score(&f, &MaintainerThresholds::v1(), 30);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(ev.iter().any(|e| e.code == "repo_too_young"));
    }

    #[test]
    fn zero_active_maintainers_is_low_confidence() {
        let f = baseline();
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        assert_eq!(r.confidence, Confidence::Low);
    }

    #[test]
    fn governance_docs_boost_subscore() {
        let mut f = baseline();
        f.active_maintainers_last_year = 4;
        f.bus_factor_proxy = 4;
        f.has_codeowners = true;
        f.has_governance_doc = true;
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        let g = r.sub_scores.get("governance_docs").copied().unwrap_or(0);
        assert!(g >= 90, "expected governance_docs ≥ 90, got {g}");
    }

    #[test]
    fn no_governance_docs_falls_to_neutral() {
        let mut f = baseline();
        f.active_maintainers_last_year = 4;
        f.bus_factor_proxy = 3;
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        let g = r.sub_scores.get("governance_docs").copied().unwrap_or(0);
        assert!((40..=60).contains(&g), "expected ~50, got {g}");
    }

    #[test]
    fn evidence_codes_are_unique() {
        let mut f = baseline();
        f.active_maintainers_last_year = 3;
        f.bus_factor_proxy = 2;
        f.commit_gini = 0.5;
        f.contributor_retention_rate = 0.4;
        f.has_codeowners = true;
        f.top_authors = vec![AuthorCount {
            login: "alice".into(),
            commits: 50,
        }];
        let (_, ev) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        let mut codes: Vec<&str> = ev.iter().map(|e| e.code.as_str()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), ev.len(), "codes should be unique");
    }

    #[test]
    fn module_result_carries_module_name_and_emits_at_least_three_evidence() {
        let mut f = baseline();
        f.active_maintainers_last_year = 2;
        f.bus_factor_proxy = 2;
        let (r, ev) = score(&f, &MaintainerThresholds::v1(), 365);
        assert_eq!(r.module, "maintainers");
        assert!(ev.len() >= 3);
    }

    #[test]
    fn bus_factor_zero_is_zero_score() {
        let mut f = baseline();
        f.bus_factor_proxy = 0;
        let (r, _) = score(&f, &MaintainerThresholds::v1(), 365 * 3);
        let bf = r.sub_scores.get("bus_factor_proxy").copied().unwrap_or(0);
        assert_eq!(bf, 0);
    }
}
