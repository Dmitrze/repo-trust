//! Security & Readiness scorer.
//!
//! Federation policy per `docs/methodology.md` §Module 5:
//! - Scorecard ≤30 days old: contributes weight 0.40 of module score, High confidence.
//! - Scorecard 30-90 days: weight 0.30, Medium confidence.
//! - Scorecard >90 days or absent: contributes 0, Low confidence; module
//!   relies on doc-presence + CI + semver only.

use std::collections::BTreeMap;

use serde_json::json;

use crate::features::security::SecurityFeatures;
use crate::models::{Confidence, EvidenceItem, ModuleResult, Verdict};

const MODULE_NAME: &str = "security";

/// Threshold-free scorer; the federation policy is hard-coded per
/// `methodology.md` §Module 5 and versioned with `SCORING_VERSION`.
#[must_use]
pub fn score(features: &SecurityFeatures) -> (ModuleResult, Vec<EvidenceItem>) {
    let mut sub_scores: BTreeMap<String, u8> = BTreeMap::new();
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // ─── 1. Documentation presence ──────────────────────────────────────
    let doc_score = doc_score_from(features);
    sub_scores.insert("documentation_presence".into(), doc_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "documentation_presence".into(),
        label: "Presence of governance + community documents".into(),
        value: json!({
            "has_security_md": features.has_security_md,
            "has_contributing_md": features.has_contributing_md,
            "has_code_of_conduct": features.has_code_of_conduct,
            "has_license": features.has_license,
            "has_codeowners": features.has_codeowners,
        }),
        threshold: None,
        verdict: verdict_from_score(doc_score),
        rationale: format!(
            "{}/5 expected docs present (SECURITY, CONTRIBUTING, CODE_OF_CONDUCT, LICENSE, CODEOWNERS).",
            count_present_docs(features)
        ),
    });

    // Surface missing-LICENSE as its own Concerning evidence per scenario S-103.
    if !features.has_license {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "has_license".into(),
            label: "Repository has no detected LICENSE".into(),
            value: json!(false),
            threshold: None,
            verdict: Verdict::Concerning,
            rationale: "No LICENSE / LICENSE.md / LICENSE.txt / COPYING found at the repo root."
                .into(),
        });
    }

    // ─── 2. CI workflow presence ───────────────────────────────────────
    let ci_score: u8 = if features.has_ci_workflow { 100 } else { 0 };
    sub_scores.insert("ci_workflow".into(), ci_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "ci_workflow_present".into(),
        label: "CI workflow files present".into(),
        value: json!(features.has_ci_workflow),
        threshold: None,
        verdict: if features.has_ci_workflow {
            Verdict::Positive
        } else {
            Verdict::Concerning
        },
        rationale: if features.has_ci_workflow {
            "Found at least one CI workflow file (e.g. .github/workflows/*.yml).".into()
        } else {
            "No CI workflow files detected at the conventional paths.".into()
        },
    });

    // ─── 3. Semver consistency ─────────────────────────────────────────
    let semver_score: u8 = if features.semver_consistent { 100 } else { 50 };
    sub_scores.insert("semver_consistency".into(), semver_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "semver_consistency".into(),
        label: "Release tags follow SemVer".into(),
        value: json!(features.semver_consistent),
        threshold: None,
        verdict: if features.semver_consistent {
            Verdict::Positive
        } else {
            Verdict::Neutral
        },
        rationale: if features.semver_consistent {
            "All release tags match `vX.Y.Z` or `X.Y.Z`.".into()
        } else {
            "Some release tags do not follow SemVer (or no tagged releases yet).".into()
        },
    });

    // ─── 4. OSV advisories (Day-2 placeholder) ─────────────────────────
    let osv_score: u8 = match features.osv_open_advisories {
        0 => 100,
        1..=2 => 60,
        _ => 20,
    };
    sub_scores.insert("osv_advisories".into(), osv_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "osv_open_advisories".into(),
        label: "Open OSV advisories on published packages".into(),
        value: json!(features.osv_open_advisories),
        threshold: None,
        verdict: verdict_from_score(osv_score),
        rationale: "Day 2: per-package OSV mapping is deferred to Day 3 (Adoption + deps.dev). Counted as 0 advisories until then.".into(),
    });
    missing.push("osv_deferred_to_phase_3".into());

    // ─── 5. Scorecard federation ───────────────────────────────────────
    let (scorecard_subscore, scorecard_weight) = match (
        features.scorecard_score,
        features.scorecard_age_days,
    ) {
        (Some(score), Some(age)) if age <= 30 => {
            let s = score_to_u8(score);
            sub_scores.insert("scorecard_score".into(), s);
            evidence.push(EvidenceItem {
                module: MODULE_NAME.into(),
                code: "scorecard_score".into(),
                label: "OpenSSF Scorecard score".into(),
                value: json!(crate::utils::time::round6(score)),
                threshold: Some(json!({"max": 10.0})),
                verdict: verdict_from_score(s),
                rationale: format!(
                    "Scorecard score {score:.1}/10 (run {age}d ago). Federated weight 0.40."
                ),
            });
            if !features.scorecard_checks_failed.is_empty() {
                evidence.push(EvidenceItem {
                    module: MODULE_NAME.into(),
                    code: "scorecard_failed_checks".into(),
                    label: "Scorecard checks scoring < 5".into(),
                    value: json!(features.scorecard_checks_failed),
                    threshold: None,
                    verdict: Verdict::Concerning,
                    rationale: format!(
                        "{} Scorecard check(s) scored below 5.",
                        features.scorecard_checks_failed.len()
                    ),
                });
            }
            (Some(s), 4.0)
        },
        (Some(score), Some(age)) if age <= 90 => {
            let s = score_to_u8(score);
            sub_scores.insert("scorecard_score".into(), s);
            evidence.push(EvidenceItem {
                module: MODULE_NAME.into(),
                code: "scorecard_score".into(),
                label: "OpenSSF Scorecard score (stale)".into(),
                value: json!(crate::utils::time::round6(score)),
                threshold: Some(json!({"max": 10.0, "stale_after_days": 30})),
                verdict: verdict_from_score(s),
                rationale: format!(
                    "Scorecard score {score:.1}/10, last run {age}d ago. Federated weight 0.30 (stale window)."
                ),
            });
            (Some(s), 3.0)
        },
        _ => {
            missing.push("scorecard".into());
            evidence.push(EvidenceItem {
                module: MODULE_NAME.into(),
                code: "scorecard_unavailable".into(),
                label: "OpenSSF Scorecard report unavailable".into(),
                value: json!(null),
                threshold: None,
                verdict: Verdict::Neutral,
                rationale: "Scorecard has not yet scored this repository. Falling back to doc + CI + semver signals only.".into(),
            });
            (None, 0.0)
        },
    };

    // ─── Archived ──────────────────────────────────────────────────────
    if features.archived {
        missing.push("archived".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "archived".into(),
            label: "Repository is archived".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Owner has archived this repository; security signals are frozen.".into(),
        });
    }

    // ─── Final score ───────────────────────────────────────────────────
    // Fixed weights for non-Scorecard signals: docs=2.0, ci=1.0, semver=0.5, osv=0.5.
    // Scorecard adds its own weight when present.
    let mut total_weight = 2.0 + 1.0 + 0.5 + 0.5;
    let mut weighted_sum = 2.0 * doc_score as f64
        + 1.0 * ci_score as f64
        + 0.5 * semver_score as f64
        + 0.5 * osv_score as f64;
    if let Some(s) = scorecard_subscore {
        weighted_sum += scorecard_weight * s as f64;
        total_weight += scorecard_weight;
    }
    let final_score = (weighted_sum / total_weight).round().clamp(0.0, 100.0) as u8;

    // ─── Confidence ────────────────────────────────────────────────────
    let confidence = compute_confidence(features);

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

/// Map Scorecard's 0.0-10.0 to a 0-100 sub-score.
fn score_to_u8(score: f64) -> u8 {
    (score * 10.0).round().clamp(0.0, 100.0) as u8
}

fn count_present_docs(f: &SecurityFeatures) -> u8 {
    [
        f.has_security_md,
        f.has_contributing_md,
        f.has_code_of_conduct,
        f.has_license,
        f.has_codeowners,
    ]
    .into_iter()
    .filter(|b| *b)
    .count() as u8
}

fn doc_score_from(f: &SecurityFeatures) -> u8 {
    // Each present doc contributes 20.
    let n = count_present_docs(f) as u32;
    (n * 20).min(100) as u8
}

fn verdict_from_score(s: u8) -> Verdict {
    match s {
        80..=100 => Verdict::Positive,
        50..=79 => Verdict::Neutral,
        20..=49 => Verdict::Concerning,
        _ => Verdict::HighRisk,
    }
}

fn compute_confidence(features: &SecurityFeatures) -> Confidence {
    if features.archived {
        return Confidence::Low;
    }
    match features.scorecard_age_days {
        Some(d) if d <= 30 => Confidence::High,
        Some(d) if d <= 90 => Confidence::Medium,
        _ => Confidence::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::security::SecurityFeatures;

    fn baseline() -> SecurityFeatures {
        SecurityFeatures::default()
    }

    #[test]
    fn fresh_scorecard_drives_high_confidence_and_high_score() {
        let mut f = baseline();
        f.scorecard_score = Some(8.7);
        f.scorecard_age_days = Some(12);
        f.has_security_md = true;
        f.has_contributing_md = true;
        f.has_code_of_conduct = true;
        f.has_license = true;
        f.has_codeowners = true;
        f.has_ci_workflow = true;
        f.semver_consistent = true;
        let (r, ev) = score(&f);
        assert_eq!(r.confidence, Confidence::High);
        assert!(r.score >= 75, "expected ≥75, got {}", r.score);
        assert!(ev.len() >= 4);
        assert!(r.sub_scores.contains_key("scorecard_score"));
    }

    #[test]
    fn missing_scorecard_falls_back_to_low_confidence() {
        let mut f = baseline();
        f.has_license = true;
        let (r, ev) = score(&f);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "scorecard"));
        assert!(ev.iter().any(|e| e.code == "scorecard_unavailable"));
        // No scorecard sub-score.
        assert!(!r.sub_scores.contains_key("scorecard_score"));
    }

    #[test]
    fn stale_scorecard_30_to_90_days_is_medium_confidence_with_lower_weight() {
        let mut f = baseline();
        f.scorecard_score = Some(7.5);
        f.scorecard_age_days = Some(60);
        f.has_license = true;
        let (r, _) = score(&f);
        assert_eq!(r.confidence, Confidence::Medium);
    }

    #[test]
    fn very_stale_scorecard_over_90_days_is_low_confidence() {
        let mut f = baseline();
        f.scorecard_score = Some(7.5);
        f.scorecard_age_days = Some(120);
        let (r, _) = score(&f);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "scorecard"));
    }

    #[test]
    fn missing_license_emits_concerning_evidence() {
        let mut f = baseline();
        f.has_license = false;
        let (_, ev) = score(&f);
        let item = ev
            .iter()
            .find(|e| e.code == "has_license")
            .expect("has_license evidence");
        assert!(matches!(item.verdict, Verdict::Concerning));
    }

    #[test]
    fn ci_workflow_present_scores_full() {
        let mut f = baseline();
        f.has_ci_workflow = true;
        let (r, _) = score(&f);
        assert_eq!(r.sub_scores.get("ci_workflow").copied(), Some(100));
    }

    #[test]
    fn semver_inconsistent_drops_to_neutral() {
        let mut f = baseline();
        f.semver_consistent = false;
        let (r, _) = score(&f);
        assert_eq!(r.sub_scores.get("semver_consistency").copied(), Some(50));
    }

    #[test]
    fn osv_zero_advisories_scores_full() {
        let mut f = baseline();
        f.osv_open_advisories = 0;
        let (r, _) = score(&f);
        assert_eq!(r.sub_scores.get("osv_advisories").copied(), Some(100));
    }

    #[test]
    fn osv_many_advisories_drops_score() {
        let mut f = baseline();
        f.osv_open_advisories = 5;
        let (r, _) = score(&f);
        assert_eq!(r.sub_scores.get("osv_advisories").copied(), Some(20));
    }

    #[test]
    fn archived_demotes_to_low_confidence() {
        let mut f = baseline();
        f.scorecard_score = Some(9.0);
        f.scorecard_age_days = Some(5);
        f.archived = true;
        let (r, _) = score(&f);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "archived"));
    }

    #[test]
    fn evidence_codes_are_unique() {
        let mut f = baseline();
        f.scorecard_score = Some(8.0);
        f.scorecard_age_days = Some(20);
        f.scorecard_checks_failed = vec!["Pinned-Dependencies".into()];
        f.has_security_md = true;
        f.has_license = true;
        f.has_ci_workflow = true;
        f.semver_consistent = true;
        let (_, ev) = score(&f);
        let mut codes: Vec<&str> = ev.iter().map(|e| e.code.as_str()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), ev.len(), "codes must be unique");
    }

    #[test]
    fn module_result_carries_module_name() {
        let f = baseline();
        let (r, _) = score(&f);
        assert_eq!(r.module, "security");
    }

    #[test]
    fn failed_scorecard_checks_emit_concerning_evidence() {
        let mut f = baseline();
        f.scorecard_score = Some(6.0);
        f.scorecard_age_days = Some(10);
        f.scorecard_checks_failed = vec!["Branch-Protection".into(), "Pinned-Dependencies".into()];
        let (_, ev) = score(&f);
        let item = ev
            .iter()
            .find(|e| e.code == "scorecard_failed_checks")
            .expect("failed-checks evidence");
        assert!(matches!(item.verdict, Verdict::Concerning));
    }
}
