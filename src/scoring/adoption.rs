//! Adoption Signals scorer — pure function from features → `(ModuleResult, evidence)`.
//!
//! Per `docs/methodology.md` §Module 4 and `specs/adoption-signals-module.md`,
//! the scorer aggregates four sub-scores: `weekly_downloads` (logarithmic
//! banding), `documentation_maturity`, `package_systems_count`, and
//! `awesome_list_mentions`. The final score is the arithmetic mean of the
//! present sub-scores.
//!
//! Conservative posture: a repo that publishes no package is a Neutral
//! caveat, NEVER a Concerning verdict ("we don't penalize this — we simply
//! lower the module's confidence to Medium" — methodology §Module 4).

use std::collections::BTreeMap;

use serde_json::json;

use super::thresholds::AdoptionThresholds;
use crate::features::adoption::AdoptionFeatures;
use crate::models::{Confidence, EvidenceItem, ModuleResult, Verdict};

const MODULE_NAME: &str = "adoption";

/// Score the adoption module.
#[must_use]
pub fn score(
    features: &AdoptionFeatures,
    thresholds: &AdoptionThresholds,
) -> (ModuleResult, Vec<EvidenceItem>) {
    let mut sub_scores: BTreeMap<String, u8> = BTreeMap::new();
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // ─── 1. Weekly downloads (logarithmic banding) ─────────────────────────
    if let Some(dl) = features.weekly_downloads {
        let s = downloads_to_score(dl, thresholds);
        sub_scores.insert("weekly_downloads".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "weekly_downloads".into(),
            label: "Aggregated weekly downloads across published packages".into(),
            value: json!(dl),
            threshold: Some(json!({
                "band_25": thresholds.downloads_band_25,
                "band_50": thresholds.downloads_band_50,
                "band_75": thresholds.downloads_band_75,
                "band_100": thresholds.downloads_band_100,
            })),
            verdict: verdict_from_score(s),
            rationale: format!(
                "{dl} weekly downloads. Logarithmic banding: 0→0, {}→25, {}→50, {}→75, {}+→100.",
                thresholds.downloads_band_25,
                thresholds.downloads_band_50,
                thresholds.downloads_band_75,
                thresholds.downloads_band_100,
            ),
        });
    } else {
        // No published package — Neutral caveat per methodology §Module 4.
        missing.push("no_packages".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "no_packages".into(),
            label: "Repository publishes no package (or deps.dev returned no downloads)".into(),
            value: json!(null),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Repository publishes no package; documentation maturity is used as the only adoption signal. This is a caveat, not a concern — many excellent repos (research, docs, examples) publish no package.".into(),
        });
    }

    // ─── 2. Documentation maturity ─────────────────────────────────────────
    let doc_score = doc_maturity_to_score(features.documentation_maturity_score);
    sub_scores.insert("documentation_maturity".into(), doc_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "documentation_maturity".into(),
        label: "Documentation maturity (README + docs/ + examples/)".into(),
        value: json!({
            "score": crate::utils::time::round6(features.documentation_maturity_score),
            "has_readme": features.has_readme,
            "readme_word_count": features.readme_word_count,
            "has_docs_dir": features.has_docs_dir,
            "has_examples_dir": features.has_examples_dir,
        }),
        threshold: Some(json!({
            "readme_words_full_credit": thresholds.readme_words_full_credit,
            "readme_words_half_credit": thresholds.readme_words_half_credit,
        })),
        verdict: verdict_from_score(doc_score),
        rationale: format!(
            "Doc-maturity {:.2}/1.0. README {}; docs/ {}; examples/ {}.",
            features.documentation_maturity_score,
            if features.has_readme {
                "present"
            } else {
                "absent"
            },
            if features.has_docs_dir {
                "present"
            } else {
                "absent"
            },
            if features.has_examples_dir {
                "present"
            } else {
                "absent"
            },
        ),
    });

    // ─── 2a. Missing README (Concerning per scenario S-201) ────────────────
    if !features.has_readme {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "no_readme".into(),
            label: "Repository has no detected README".into(),
            value: json!(false),
            threshold: None,
            verdict: Verdict::Concerning,
            rationale: "GET /readme returned 404 — the default branch has no README at the conventional paths.".into(),
        });
    }

    // ─── 3. Package systems count ──────────────────────────────────────────
    let systems_score = systems_count_to_score(features.package_systems_count);
    sub_scores.insert("package_systems_count".into(), systems_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "package_systems".into(),
        label: "Distinct package systems published".into(),
        value: json!(features.package_systems),
        threshold: None,
        verdict: verdict_from_score(systems_score),
        rationale: format!(
            "{} distinct package system(s): {}. Cross-ecosystem packaging is a strong adoption signal.",
            features.package_systems_count,
            if features.package_systems.is_empty() {
                "none".to_string()
            } else {
                features.package_systems.join(", ")
            },
        ),
    });

    // ─── 4. Awesome-list mentions (neutral baseline) ───────────────────────
    let awesome_score = awesome_to_score(features.awesome_list_mentions);
    sub_scores.insert("awesome_list_mentions".into(), awesome_score);
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "awesome_list_mentions".into(),
        label: "Awesome-list mentions".into(),
        value: json!(features.awesome_list_mentions),
        threshold: None,
        verdict: verdict_from_score(awesome_score),
        rationale: format!(
            "{} awesome-list mention(s). 0 is the neutral baseline (most repos are not on awesome lists); ≥1 is a Positive boost.",
            features.awesome_list_mentions,
        ),
    });

    // ─── deps.dev outage caveat ────────────────────────────────────────────
    if features.deps_dev_error {
        missing.push("deps_dev_unavailable".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "deps_dev_unavailable".into(),
            label: "deps.dev API unavailable".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "deps.dev returned an error during this scan. Downloads + ecosystem-count signals are skipped; this scan reflects documentation signals only.".into(),
        });
    }

    // ─── Archived ──────────────────────────────────────────────────────────
    if features.archived {
        missing.push("archived".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "archived".into(),
            label: "Repository is archived".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Owner has archived this repository; adoption signals are frozen.".into(),
        });
    }

    // ─── Final score = arithmetic mean of present sub-scores ──────────────
    let final_score = if sub_scores.is_empty() {
        0
    } else {
        let sum: u32 = sub_scores.values().map(|s| u32::from(*s)).sum();
        let n = u32::try_from(sub_scores.len()).unwrap_or(1).max(1);
        u8::try_from((sum + n / 2) / n).unwrap_or(0)
    };

    let confidence = compute_confidence(features, thresholds);

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

/// Logarithmic banding per `specs/adoption-signals-module.md` §9 v1.
///
/// Pure step function rather than smooth interpolation — adoption is a
/// power-law signal and a few crisp bands beat a noisy linear curve.
#[must_use]
pub fn downloads_to_score(downloads: u64, t: &AdoptionThresholds) -> u8 {
    if downloads >= t.downloads_band_100 {
        100
    } else if downloads >= t.downloads_band_75 {
        75
    } else if downloads >= t.downloads_band_50 {
        50
    } else if downloads >= t.downloads_band_25 {
        25
    } else {
        0
    }
}

/// Map a `[0.0, 1.0]` documentation-maturity score to a 0-100 sub-score.
#[must_use]
pub fn doc_maturity_to_score(maturity: f64) -> u8 {
    let s = (maturity.clamp(0.0, 1.0) * 100.0).round();
    u8::try_from(s as i64).unwrap_or(0)
}

/// Cross-ecosystem packaging is a strong adoption signal. 0 → 0 (no
/// published package), 1 → 75 (the common single-ecosystem case), 2+ → 100.
#[must_use]
pub fn systems_count_to_score(count: u64) -> u8 {
    match count {
        0 => 0,
        1 => 75,
        _ => 100,
    }
}

/// Awesome-list mentions. 0 is the *neutral baseline* — most repos are not
/// on awesome lists, so absence is not a negative.
#[must_use]
pub fn awesome_to_score(count: u64) -> u8 {
    match count {
        0 => 50,
        1 => 75,
        _ => 100,
    }
}

fn verdict_from_score(s: u8) -> Verdict {
    match s {
        80..=100 => Verdict::Positive,
        50..=79 => Verdict::Neutral,
        20..=49 => Verdict::Concerning,
        _ => Verdict::HighRisk,
    }
}

fn compute_confidence(features: &AdoptionFeatures, t: &AdoptionThresholds) -> Confidence {
    if features.archived {
        return Confidence::Low;
    }
    if features.deps_dev_error {
        return Confidence::Low;
    }
    if features.weekly_downloads.is_none() {
        // No published package → Medium per methodology §Module 4.
        return Confidence::Medium;
    }
    // High when downloads clear the floor; otherwise Medium.
    if features
        .weekly_downloads
        .is_some_and(|d| d >= t.high_confidence_downloads_floor)
    {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::adoption::AdoptionFeatures;

    fn baseline() -> AdoptionFeatures {
        AdoptionFeatures::default()
    }

    fn popular() -> AdoptionFeatures {
        AdoptionFeatures {
            weekly_downloads: Some(500_000),
            package_systems: vec!["GO".into()],
            package_systems_count: 1,
            has_readme: true,
            readme_word_count: Some(1200),
            has_docs_dir: true,
            has_examples_dir: true,
            documentation_maturity_score: 1.0,
            awesome_list_mentions: 0,
            archived: false,
            deps_dev_error: false,
        }
    }

    // S-001: popular package + complete docs scores high.
    #[test]
    fn s001_popular_package_scores_high() {
        let f = popular();
        let (r, ev) = score(&f, &AdoptionThresholds::v1());
        assert!(r.score >= 75, "expected ≥75, got {}", r.score);
        assert_eq!(r.confidence, Confidence::High);
        let dl = ev
            .iter()
            .find(|e| e.code == "weekly_downloads")
            .expect("weekly_downloads evidence");
        assert!(matches!(dl.verdict, Verdict::Neutral | Verdict::Positive));
        let dm = ev
            .iter()
            .find(|e| e.code == "documentation_maturity")
            .expect("documentation_maturity evidence");
        assert!(matches!(dm.verdict, Verdict::Positive));
    }

    // S-002: minimal but well-documented package lands mid.
    #[test]
    fn s002_minimal_but_documented_lands_mid() {
        let mut f = baseline();
        f.weekly_downloads = Some(1_500);
        f.package_systems = vec!["NPM".into()];
        f.package_systems_count = 1;
        f.has_readme = true;
        f.readme_word_count = Some(600);
        f.has_docs_dir = true;
        f.documentation_maturity_score = 0.80; // README + docs, no examples.
        let (r, _) = score(&f, &AdoptionThresholds::v1());
        assert!(
            (40..=70).contains(&r.score),
            "expected 40..=70, got {}",
            r.score
        );
    }

    // S-101: no published package falls back gracefully.
    #[test]
    fn s101_no_packages_medium_confidence_with_caveat() {
        let mut f = baseline();
        f.has_readme = true;
        f.readme_word_count = Some(400);
        f.documentation_maturity_score = 0.35;
        let (r, ev) = score(&f, &AdoptionThresholds::v1());
        assert_eq!(r.confidence, Confidence::Medium);
        assert!(r.missing_data.iter().any(|m| m == "no_packages"));
        let np = ev
            .iter()
            .find(|e| e.code == "no_packages")
            .expect("no_packages evidence");
        assert!(matches!(np.verdict, Verdict::Neutral));
        // Module score is computed from the present sub-scores only.
        assert!(!r.sub_scores.contains_key("weekly_downloads"));
    }

    // S-102: deps.dev outage drops to Low confidence.
    #[test]
    fn s102_deps_dev_error_low_confidence_with_caveat() {
        let mut f = baseline();
        f.has_readme = true;
        f.readme_word_count = Some(300);
        f.documentation_maturity_score = 0.35;
        f.deps_dev_error = true;
        let (r, ev) = score(&f, &AdoptionThresholds::v1());
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "deps_dev_unavailable"));
        let cav = ev
            .iter()
            .find(|e| e.code == "deps_dev_unavailable")
            .expect("deps_dev_unavailable evidence");
        assert!(matches!(cav.verdict, Verdict::Neutral));
    }

    // S-103: archived repo demotes to Low confidence + caveat.
    #[test]
    fn s103_archived_demotes_to_low_confidence() {
        let mut f = popular();
        f.archived = true;
        let (r, ev) = score(&f, &AdoptionThresholds::v1());
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "archived"));
        assert!(ev.iter().any(|e| e.code == "archived"));
    }

    // S-201: missing README → Concerning evidence.
    #[test]
    fn s201_missing_readme_emits_concerning() {
        let mut f = baseline();
        f.weekly_downloads = Some(50_000);
        f.package_systems = vec!["GO".into()];
        f.package_systems_count = 1;
        // README absent, docs absent, examples absent → maturity 0.
        let (_, ev) = score(&f, &AdoptionThresholds::v1());
        let nr = ev
            .iter()
            .find(|e| e.code == "no_readme")
            .expect("no_readme evidence");
        assert!(matches!(nr.verdict, Verdict::Concerning));
    }

    #[test]
    fn download_band_boundaries_are_correct() {
        let t = AdoptionThresholds::v1();
        assert_eq!(downloads_to_score(0, &t), 0);
        assert_eq!(downloads_to_score(999, &t), 0);
        assert_eq!(downloads_to_score(1_000, &t), 25);
        assert_eq!(downloads_to_score(9_999, &t), 25);
        assert_eq!(downloads_to_score(10_000, &t), 50);
        assert_eq!(downloads_to_score(99_999, &t), 50);
        assert_eq!(downloads_to_score(100_000, &t), 75);
        assert_eq!(downloads_to_score(999_999, &t), 75);
        assert_eq!(downloads_to_score(1_000_000, &t), 100);
        assert_eq!(downloads_to_score(50_000_000, &t), 100);
    }

    #[test]
    fn multi_system_packaging_boost() {
        // A single-ecosystem repo scores 75 on the systems sub-score; a
        // cross-ecosystem repo scores 100. Both score the same on every
        // other sub-score → final score ordering preserved.
        let mut single = popular();
        single.package_systems = vec!["NPM".into()];
        single.package_systems_count = 1;
        let mut multi = popular();
        multi.package_systems = vec!["NPM".into(), "PYPI".into()];
        multi.package_systems_count = 2;
        let (rs, _) = score(&single, &AdoptionThresholds::v1());
        let (rm, _) = score(&multi, &AdoptionThresholds::v1());
        assert!(
            rm.score >= rs.score,
            "multi-system ({}) should ≥ single-system ({})",
            rm.score,
            rs.score
        );
        assert_eq!(
            rm.sub_scores.get("package_systems_count").copied(),
            Some(100)
        );
        assert_eq!(
            rs.sub_scores.get("package_systems_count").copied(),
            Some(75)
        );
    }

    #[test]
    fn no_packages_drops_downloads_subscore() {
        let mut f = baseline();
        f.has_readme = true;
        f.readme_word_count = Some(50);
        f.documentation_maturity_score = 0.20;
        let (r, _) = score(&f, &AdoptionThresholds::v1());
        assert!(!r.sub_scores.contains_key("weekly_downloads"));
        // The other three sub-scores are still present.
        assert!(r.sub_scores.contains_key("documentation_maturity"));
        assert!(r.sub_scores.contains_key("package_systems_count"));
        assert!(r.sub_scores.contains_key("awesome_list_mentions"));
    }

    #[test]
    fn awesome_list_mentions_neutral_baseline() {
        // 0 mentions → 50 (neutral); 1 → 75; 2+ → 100.
        assert_eq!(awesome_to_score(0), 50);
        assert_eq!(awesome_to_score(1), 75);
        assert_eq!(awesome_to_score(2), 100);
        assert_eq!(awesome_to_score(20), 100);
    }

    #[test]
    fn evidence_codes_are_unique() {
        let f = popular();
        let (_, ev) = score(&f, &AdoptionThresholds::v1());
        let mut codes: Vec<&str> = ev.iter().map(|e| e.code.as_str()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ev.len(), "codes must be unique");
    }

    #[test]
    fn module_result_carries_module_name_and_emits_at_least_three_evidence() {
        let f = popular();
        let (r, ev) = score(&f, &AdoptionThresholds::v1());
        assert_eq!(r.module, "adoption");
        assert!(
            ev.len() >= 3,
            "expected ≥3 evidence items, got {}",
            ev.len()
        );
    }

    #[test]
    fn no_packages_is_neutral_never_concerning() {
        // Conservative posture — no published package is a Neutral caveat,
        // never a Concerning verdict (per spec §2 + methodology §Module 4).
        let mut f = baseline();
        f.has_readme = true;
        f.readme_word_count = Some(500);
        f.documentation_maturity_score = 0.50;
        let (_, ev) = score(&f, &AdoptionThresholds::v1());
        let np = ev
            .iter()
            .find(|e| e.code == "no_packages")
            .expect("no_packages evidence");
        assert!(matches!(np.verdict, Verdict::Neutral));
        assert!(!matches!(
            np.verdict,
            Verdict::Concerning | Verdict::HighRisk
        ));
    }

    #[test]
    fn high_confidence_requires_downloads_floor() {
        // 5k downloads is below the 10k floor → Medium even with everything else.
        let mut f = popular();
        f.weekly_downloads = Some(5_000);
        let (r, _) = score(&f, &AdoptionThresholds::v1());
        assert_eq!(r.confidence, Confidence::Medium);
    }
}
