//! Star Authenticity scorer (Day 4 — full).
//!
//! Implements Heuristic 1 (low-activity profile share, 6-band table),
//! Heuristic 2 (lockstep timing z-score), and Heuristic 3 (fork/watcher
//! ratios with ecosystem multipliers) per `docs/methodology.md` §Module 1.
//! Final formula: `0.55 × H1 + 0.30 × H2 + 0.15 × H3` per methodology v1.0.
//! Falls back to `0.55 × H1 + 0.45 × H3` when H2 is `None` (short series
//! or no `starred_at` timestamps).
//!
//! Critical posture: rationale uses **only probabilistic phrasing** — the
//! words "fake", "fraud", "bot" do not appear in any code under this file
//! per `CLAUDE.md` §14 (Glossary).
//! Verdict ceiling stays `Concerning` even when combined H1+H2 evidence is
//! emitted — never `HighRisk` standalone.

use std::collections::BTreeMap;

use serde_json::json;

use super::thresholds::StarsThresholds;
use crate::features::stars::{ecosystem_multipliers, StarsFeatures};
use crate::models::{Confidence, EvidenceItem, ModuleResult, Verdict};

const MODULE_NAME: &str = "stars";

/// Score the Star Authenticity module (shallow Day-3 cut).
#[must_use]
pub fn score(
    features: &StarsFeatures,
    thresholds: &StarsThresholds,
) -> (ModuleResult, Vec<EvidenceItem>) {
    let mut sub_scores: BTreeMap<String, u8> = BTreeMap::new();
    let mut evidence: Vec<EvidenceItem> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // ─── Below-floor short-circuit ───────────────────────────────────────
    if features.total_stars < thresholds.min_stars_to_sample {
        missing.push("below_sampling_floor".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "below_sampling_floor".into(),
            label: "Repository has too few stars to sample meaningfully".into(),
            value: json!(features.total_stars),
            threshold: Some(json!(thresholds.min_stars_to_sample)),
            verdict: Verdict::Neutral,
            rationale: format!(
                "Repository has {} star(s); the Star Authenticity heuristic requires at least {} for a meaningful sample.",
                features.total_stars, thresholds.min_stars_to_sample,
            ),
        });
        return (
            ModuleResult {
                module: MODULE_NAME.into(),
                score: 0,
                confidence: Confidence::Low,
                sub_scores,
                sample_size: Some(0),
                missing_data: missing,
            },
            evidence,
        );
    }

    // ─── 1. Heuristic 1 — low-activity stargazer share ──────────────────
    let h1_score: Option<u8> = if let Some(share) = features.low_activity_share {
        let leniency_applied = features.repo_age_days < thresholds.young_repo_age_days;
        let leniency = if leniency_applied {
            thresholds.young_repo_leniency_pp
        } else {
            0.0
        };
        let adjusted_share = (share - leniency).max(0.0);
        let s = bucket_low_activity(adjusted_share, &thresholds.low_activity_bands);
        sub_scores.insert("low_activity_share".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "low_activity_stargazer_share".into(),
            label: "Share of sampled stargazers matching the 9-signal low-activity profile".into(),
            value: json!(crate::utils::time::round6(share)),
            threshold: Some(json!({
                "bands": thresholds.low_activity_bands,
                "young_repo_leniency_pp": thresholds.young_repo_leniency_pp,
            })),
            // Verdict ceiling for this heuristic is Concerning (never HighRisk standalone).
            verdict: stars_verdict(s),
            rationale: format!(
                "{:.1}% of {} sampled stargazers match the 9-signal low-activity profile{}.",
                share * 100.0,
                features.sample_size,
                if leniency_applied {
                    format!(
                        " ({:.0}pp leniency applied for repo younger than {} days)",
                        leniency * 100.0,
                        thresholds.young_repo_age_days
                    )
                } else {
                    String::new()
                },
            ),
        });
        Some(s)
    } else {
        // Sample empty (Quick mode or get_user errored). Don't count this as
        // a fatal data gap; just rely on Heuristic 3 alone.
        missing.push("no_stargazer_sample".into());
        None
    };

    // ─── 2. Heuristic 3 — fork / watcher ratios with ecosystem multipliers
    let (fork_mult, watcher_mult) = ecosystem_multipliers(features.primary_language.as_deref());
    let fork_healthy = thresholds.fork_to_star_healthy * fork_mult;
    let watcher_healthy = thresholds.watcher_to_star_healthy * watcher_mult;

    let fork_score = ratio_score(features.fork_to_star_ratio, fork_healthy);
    let watcher_score = ratio_score(features.watcher_to_star_ratio, watcher_healthy);
    let h3_score: u8 = ((u32::from(fork_score) + u32::from(watcher_score)) / 2) as u8;

    sub_scores.insert("fork_to_star_ratio".into(), fork_score);
    sub_scores.insert("watcher_to_star_ratio".into(), watcher_score);

    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "fork_to_star_ratio".into(),
        label: "Forks-to-stars ratio".into(),
        value: json!(crate::utils::time::round6(features.fork_to_star_ratio)),
        threshold: Some(json!({
            "healthy": crate::utils::time::round6(fork_healthy),
            "ecosystem_multiplier": fork_mult,
        })),
        verdict: stars_verdict(fork_score),
        rationale: format!(
            "fork/star ratio = {:.4}; ecosystem-adjusted healthy threshold ≥ {:.4} (multiplier {:.2}).",
            features.fork_to_star_ratio, fork_healthy, fork_mult,
        ),
    });
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "watcher_to_star_ratio".into(),
        label: "Watchers-to-stars ratio".into(),
        value: json!(crate::utils::time::round6(features.watcher_to_star_ratio)),
        threshold: Some(json!({
            "healthy": crate::utils::time::round6(watcher_healthy),
            "ecosystem_multiplier": watcher_mult,
        })),
        verdict: stars_verdict(watcher_score),
        rationale: format!(
            "watcher/star ratio = {:.4}; ecosystem-adjusted healthy threshold ≥ {:.4} (multiplier {:.2}).",
            features.watcher_to_star_ratio, watcher_healthy, watcher_mult,
        ),
    });

    // ─── Heuristic 2 — lockstep timing z-score ──────────────────────────
    let h2_score: Option<u8> = if let Some(z) = features.lockstep_z_score {
        let s = bucket_lockstep(z, &thresholds.lockstep_score_bands);
        sub_scores.insert("lockstep_z_score".into(), s);
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "lockstep_z_score".into(),
            label: "Lockstep timing — max daily z-score over 28-day baseline".into(),
            value: json!(crate::utils::time::round6(z)),
            threshold: Some(json!({
                "bands": thresholds.lockstep_score_bands.iter()
                    .map(|(c, s)| serde_json::json!([if c.is_finite() { json!(c) } else { json!("infinity") }, s]))
                    .collect::<Vec<_>>(),
            })),
            verdict: stars_verdict(s),
            rationale: format!(
                "Max daily z-score = {:.2} over a rolling 28-day baseline lagged 7 days. ≥5 indicates a starring burst; ≥3 a notable spike.",
                z
            ),
        });
        Some(s)
    } else {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "lockstep_window_too_short".into(),
            label: "Lockstep timing window unavailable".into(),
            value: json!(null),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Sample spans fewer than 35 days (28 baseline + 7 lag) or carries no starred_at timestamps. Heuristic 2 contribution is dropped from the final formula.".into(),
        });
        None
    };

    // ─── Day-4 weighted formula: 0.55 × H1 + 0.30 × H2 + 0.15 × H3 ──────
    // When H2 is unavailable, fall back to the Day-3 redistribution
    // (0.55 × H1 + 0.45 × H3) so the module still produces a reasonable score.
    let final_score: u8 = match (h1_score, h2_score) {
        (Some(h1), Some(h2)) => {
            let raw = 0.55 * f64::from(h1) + 0.30 * f64::from(h2) + 0.15 * f64::from(h3_score);
            raw.round().clamp(0.0, 100.0) as u8
        },
        (Some(h1), None) => {
            let raw = 0.55 * f64::from(h1) + 0.45 * f64::from(h3_score);
            raw.round().clamp(0.0, 100.0) as u8
        },
        (None, Some(h2)) => {
            // Sample empty (Quick mode) but z-score available somehow:
            // weight H2 + H3 evenly.
            let raw = 0.50 * f64::from(h2) + 0.50 * f64::from(h3_score);
            raw.round().clamp(0.0, 100.0) as u8
        },
        (None, None) => h3_score, // Ratios alone.
    };

    // ─── Combined H1+H2 evidence (Concerning, never HighRisk) ───────────
    if let (Some(share), Some(z)) = (features.low_activity_share, features.lockstep_z_score) {
        if share >= thresholds.combined_low_activity_threshold
            && z >= thresholds.combined_z_threshold
        {
            evidence.push(EvidenceItem {
                module: MODULE_NAME.into(),
                code: "combined_low_activity_and_lockstep".into(),
                label: "Combined Heuristic 1 + Heuristic 2 signal".into(),
                value: json!({
                    "low_activity_share": crate::utils::time::round6(share),
                    "lockstep_z_score": crate::utils::time::round6(z),
                }),
                threshold: Some(json!({
                    "low_activity_share": thresholds.combined_low_activity_threshold,
                    "lockstep_z_score": thresholds.combined_z_threshold,
                })),
                // Methodology requires BOTH signals before lowering the
                // module score band. Verdict ceiling stays Concerning per
                // CLAUDE.md §14.
                verdict: Verdict::Concerning,
                rationale: format!(
                    "Both signals present: {:.1}% of sampled stargazers match the low-activity profile AND the daily star series shows a max z-score of {:.2}. Methodology recommends treating this combination as Concerning.",
                    share * 100.0, z,
                ),
            });
        }
    }

    // ─── Recency-bias evidence (Day-3 follow-through, Q1) ───────────────
    // Emitted on every non-below-floor run so reports are explicit about the
    // sampling bias. True uniform sampling lands in Phase 2 deep mode.
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "recency_biased_sample".into(),
        label: "Stargazer sample is recency-biased".into(),
        value: json!(features.sample_size),
        threshold: None,
        verdict: Verdict::Neutral,
        rationale: "Day 3-4 sampling is recency-biased: the most-recent N stargazers are sampled directly. True uniform random sampling over the full stargazer history is deferred to Phase 2 deep mode.".into(),
    });

    // ─── Sample-size confidence demotion ────────────────────────────────
    if features.low_activity_share.is_some()
        && features.sample_size < thresholds.min_sample_for_high_confidence
    {
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "small_sample".into(),
            label: "Stargazer sample below confidence threshold".into(),
            value: json!(features.sample_size),
            threshold: Some(json!({
                "high": thresholds.min_sample_for_high_confidence,
                "medium": thresholds.min_sample_for_medium_confidence,
            })),
            verdict: Verdict::Neutral,
            rationale: format!(
                "Sample size {} is below {} required for High confidence on Heuristic 1.",
                features.sample_size, thresholds.min_sample_for_high_confidence,
            ),
        });
    }

    // ─── Archived ───────────────────────────────────────────────────────
    if features.archived {
        missing.push("archived".into());
        evidence.push(EvidenceItem {
            module: MODULE_NAME.into(),
            code: "archived".into(),
            label: "Repository is archived".into(),
            value: json!(true),
            threshold: None,
            verdict: Verdict::Neutral,
            rationale: "Owner has archived this repository; star-authenticity signals are frozen."
                .into(),
        });
    }

    let confidence = compute_confidence(features, thresholds);

    (
        ModuleResult {
            module: MODULE_NAME.into(),
            score: final_score,
            confidence,
            sub_scores,
            sample_size: Some(features.sample_size),
            missing_data: missing,
        },
        evidence,
    )
}

/// Verdict for stars-module evidence. Clamps to `Concerning` as the maximum
/// (no `HighRisk` standalone for Heuristic 1 — methodology requires combined
/// H1 + H2 evidence to push into "High Risk" category, which can only happen
/// Day 4+).
fn stars_verdict(score: u8) -> Verdict {
    match score {
        80..=100 => Verdict::Positive,
        50..=79 => Verdict::Neutral,
        _ => Verdict::Concerning,
    }
}

fn bucket_low_activity(share: f64, bands: &[(f64, u8); 6]) -> u8 {
    for (ceiling, sub_score) in bands {
        if share <= *ceiling {
            return *sub_score;
        }
    }
    0
}

/// Bucket a lockstep z-score into a sub-score per the configured bands
/// (`<3 → 100, 3-5 → 85, 5-8 → 60, 8-12 → 30, >12 → 10` by default).
fn bucket_lockstep(z: f64, bands: &[(f64, u8); 5]) -> u8 {
    for (ceiling, sub_score) in bands {
        if z <= *ceiling {
            return *sub_score;
        }
    }
    bands.last().map(|(_, s)| *s).unwrap_or(0)
}

/// Map an observed ratio against an ecosystem-adjusted healthy threshold.
/// Linear: ≥ healthy → 100; ≤ 0 → 0; otherwise scaled by `actual / healthy`.
fn ratio_score(actual: f64, healthy: f64) -> u8 {
    if healthy <= 0.0 {
        return 0;
    }
    if actual >= healthy {
        return 100;
    }
    if actual <= 0.0 {
        return 0;
    }
    let frac = (actual / healthy).clamp(0.0, 1.0);
    (frac * 100.0).round().clamp(0.0, 100.0) as u8
}

fn compute_confidence(features: &StarsFeatures, thresholds: &StarsThresholds) -> Confidence {
    if features.archived {
        return Confidence::Low;
    }
    if features.total_stars < thresholds.min_stars_to_sample {
        return Confidence::Low;
    }
    if features.sample_size < thresholds.min_sample_for_medium_confidence {
        return Confidence::Low;
    }
    if features.sample_size < thresholds.min_sample_for_high_confidence {
        return Confidence::Medium;
    }
    if features.repo_age_days < thresholds.young_repo_age_days {
        return Confidence::Medium;
    }
    Confidence::High
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::stars::StarsFeatures;

    fn baseline() -> StarsFeatures {
        StarsFeatures {
            total_stars: 1_000,
            forks_count: 100,
            watchers_count: 10,
            fork_to_star_ratio: 0.10,
            watcher_to_star_ratio: 0.01,
            low_activity_share: Some(0.04),
            lockstep_z_score: Some(2.0), // smooth distribution → H2 = 100
            sample_size: 200,
            primary_language: Some("Rust".into()),
            repo_age_days: 365 * 3,
            archived: false,
        }
    }

    #[test]
    fn organic_profile_scores_high() {
        let f = baseline();
        let (r, ev) = score(&f, &StarsThresholds::v1());
        assert!(r.score >= 80, "expected ≥80, got {}", r.score);
        assert_eq!(r.confidence, Confidence::High);
        assert!(ev.iter().any(|e| e.code == "low_activity_stargazer_share"));
        assert!(ev.iter().any(|e| e.code == "fork_to_star_ratio"));
        assert!(ev.iter().any(|e| e.code == "watcher_to_star_ratio"));
        assert!(ev.iter().any(|e| e.code == "lockstep_z_score"));
        assert!(ev.iter().any(|e| e.code == "recency_biased_sample"));
        assert!(
            !ev.iter().any(|e| e.code == "lockstep_deferred_to_day_4"),
            "Day-3 deferred caveat should be gone now that H2 ships"
        );
    }

    #[test]
    fn suspicious_profile_lowers_score_to_concerning_not_highrisk() {
        let mut f = baseline();
        f.low_activity_share = Some(0.38);
        f.lockstep_z_score = Some(8.0); // bursty pattern
        f.fork_to_star_ratio = 0.005;
        f.watcher_to_star_ratio = 0.0005;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        // Day-4 formula: 0.55 × 20 (H1=20 for 38%) + 0.30 × 30 (H2=30 for z=8) + 0.15 × ~50 (H3 ratios)
        //              ≈ 11 + 9 + 7.5 ≈ 28-32.
        assert!(r.score <= 35, "expected ≤35, got {}", r.score);
        let h1 = ev
            .iter()
            .find(|e| e.code == "low_activity_stargazer_share")
            .unwrap();
        // Verdict ceiling: Concerning, never HighRisk standalone.
        assert!(matches!(h1.verdict, Verdict::Concerning));
        // Combined H1+H2 evidence should fire (share ≥ 0.20 AND z ≥ 5).
        let combined = ev
            .iter()
            .find(|e| e.code == "combined_low_activity_and_lockstep")
            .expect("combined evidence missing");
        assert!(matches!(combined.verdict, Verdict::Concerning));
    }

    #[test]
    fn lockstep_window_too_short_falls_back_to_h3_redistribution() {
        let mut f = baseline();
        f.lockstep_z_score = None;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        assert!(ev.iter().any(|e| e.code == "lockstep_window_too_short"));
        assert!(!r.sub_scores.contains_key("lockstep_z_score"));
    }

    #[test]
    fn lockstep_smooth_z_score_full_credit() {
        let mut f = baseline();
        f.lockstep_z_score = Some(2.5);
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.sub_scores.get("lockstep_z_score").copied(), Some(100));
    }

    #[test]
    fn lockstep_bursty_z_score_drops() {
        let mut f = baseline();
        f.lockstep_z_score = Some(10.0);
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.sub_scores.get("lockstep_z_score").copied(), Some(30));
    }

    #[test]
    fn day_4_formula_uses_methodology_weights() {
        // H1=100, H2=100, H3=100 → final = 100 (sanity check).
        let f = baseline();
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.score, 100);
    }

    #[test]
    fn combined_evidence_only_fires_when_both_thresholds_met() {
        // Single signal: only H1 high → no combined evidence.
        let mut f = baseline();
        f.low_activity_share = Some(0.30);
        f.lockstep_z_score = Some(2.0); // below combined_z_threshold = 5
        let (_, ev) = score(&f, &StarsThresholds::v1());
        assert!(
            !ev.iter()
                .any(|e| e.code == "combined_low_activity_and_lockstep"),
            "combined evidence should NOT fire when only H1 condition met"
        );
    }

    #[test]
    fn recency_biased_evidence_emitted_on_every_non_below_floor_run() {
        let f = baseline();
        let (_, ev) = score(&f, &StarsThresholds::v1());
        let item = ev
            .iter()
            .find(|e| e.code == "recency_biased_sample")
            .expect("recency_biased_sample evidence required");
        assert!(matches!(item.verdict, Verdict::Neutral));
    }

    #[test]
    fn below_floor_short_circuits_to_zero_with_low_confidence() {
        let mut f = baseline();
        f.total_stars = 25;
        f.sample_size = 0;
        f.low_activity_share = None;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.score, 0);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "below_sampling_floor"));
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].code, "below_sampling_floor");
    }

    #[test]
    fn young_repo_gets_5pp_leniency() {
        let mut f = baseline();
        f.low_activity_share = Some(0.22);
        f.repo_age_days = 60;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        // Without leniency, 0.22 falls in the 0.20-0.35 band → 40.
        // With 5pp leniency, adjusted = 0.17 → 0.10-0.20 band → 65.
        let h1 = r.sub_scores.get("low_activity_share").copied().unwrap();
        assert_eq!(h1, 65);
        assert!(ev
            .iter()
            .find(|e| e.code == "low_activity_stargazer_share")
            .unwrap()
            .rationale
            .contains("leniency"));
    }

    #[test]
    fn small_sample_demotes_confidence_to_medium() {
        let mut f = baseline();
        f.sample_size = 60;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.confidence, Confidence::Medium);
        assert!(ev.iter().any(|e| e.code == "small_sample"));
    }

    #[test]
    fn very_small_sample_demotes_to_low_confidence() {
        let mut f = baseline();
        f.sample_size = 20;
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.confidence, Confidence::Low);
    }

    #[test]
    fn ecosystem_multiplier_shifts_ratio_threshold_typescript() {
        let mut f = baseline();
        f.primary_language = Some("TypeScript".into());
        f.fork_to_star_ratio = 0.03; // below baseline 0.04 but above TS-adjusted 0.028
        let (r, _) = score(&f, &StarsThresholds::v1());
        let s = r.sub_scores.get("fork_to_star_ratio").copied().unwrap();
        assert_eq!(s, 100);
    }

    #[test]
    fn quick_mode_no_sample_falls_back_to_h3_only() {
        let mut f = baseline();
        f.sample_size = 0;
        f.low_activity_share = None;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        // Should not contain H1 sub-score; final = H3 mean.
        assert!(!r.sub_scores.contains_key("low_activity_share"));
        assert!(r.sub_scores.contains_key("fork_to_star_ratio"));
        assert!(r.missing_data.iter().any(|m| m == "no_stargazer_sample"));
        assert!(!ev.is_empty());
    }

    #[test]
    fn archived_demotes_to_low_confidence() {
        let mut f = baseline();
        f.archived = true;
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.confidence, Confidence::Low);
        assert!(r.missing_data.iter().any(|m| m == "archived"));
    }

    #[test]
    fn evidence_codes_are_unique() {
        let f = baseline();
        let (_, ev) = score(&f, &StarsThresholds::v1());
        let mut codes: Vec<&str> = ev.iter().map(|e| e.code.as_str()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), ev.len());
    }

    #[test]
    fn rationale_uses_only_probabilistic_phrasing_no_fake_fraud_bot() {
        let mut f = baseline();
        f.low_activity_share = Some(0.45);
        let (_, ev) = score(&f, &StarsThresholds::v1());
        for item in &ev {
            let lower = item.rationale.to_lowercase();
            assert!(
                !lower.contains("fake"),
                "rationale must not contain 'fake': {}",
                item.rationale
            );
            assert!(
                !lower.contains("fraud"),
                "rationale must not contain 'fraud': {}",
                item.rationale
            );
            // Match "bot" only as a word, not as part of "robot"/"both"/"bottom".
            for word in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
                assert_ne!(
                    word, "bot",
                    "rationale must not contain word 'bot': {}",
                    item.rationale
                );
            }
        }
    }

    #[test]
    fn module_result_carries_module_name() {
        let f = baseline();
        let (r, _) = score(&f, &StarsThresholds::v1());
        assert_eq!(r.module, "stars");
    }
}
