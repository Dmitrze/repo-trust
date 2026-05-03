//! Star Authenticity scorer (Day 3 — shallow).
//!
//! Implements Heuristic 1 (low-activity profile share, 6-band table) and
//! Heuristic 3 (fork/watcher ratios with ecosystem multipliers) per
//! `docs/methodology.md` §Module 1. Final formula for Day 3:
//! `0.55 × H1 + 0.45 × H3` (the lockstep H2 weight of 0.30 is redistributed
//! to H3 until H2 ships Day 4 and weights revert to 0.55 / 0.30 / 0.15).
//!
//! Critical posture: rationale uses **only probabilistic phrasing** — the
//! words "fake", "fraud", "bot" do not appear in any code under this file
//! per `CLAUDE.md` §14 (Glossary).
//! Verdict ceiling for Heuristic 1 is `Concerning` — never `HighRisk`
//! standalone.

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

    // ─── Day-3 weighted formula: 0.55 × H1 + 0.45 × H3 ──────────────────
    let final_score: u8 = match h1_score {
        Some(h1) => {
            let raw = 0.55 * f64::from(h1) + 0.45 * f64::from(h3_score);
            raw.round().clamp(0.0, 100.0) as u8
        },
        None => h3_score, // Sample empty — ratios alone.
    };

    // Lockstep deferred caveat — explicit so reports don't silently miss H2.
    evidence.push(EvidenceItem {
        module: MODULE_NAME.into(),
        code: "lockstep_deferred_to_day_4".into(),
        label: "Heuristic 2 (lockstep timing) deferred".into(),
        value: json!(null),
        threshold: None,
        verdict: Verdict::Neutral,
        rationale: "Day 3 ships Heuristics 1 + 3 only. The lockstep z-score lands Day 4; weights revert to 0.55/0.30/0.15 then.".into(),
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
        assert!(ev.iter().any(|e| e.code == "lockstep_deferred_to_day_4"));
    }

    #[test]
    fn suspicious_profile_lowers_score_to_concerning_not_highrisk() {
        let mut f = baseline();
        f.low_activity_share = Some(0.38);
        f.fork_to_star_ratio = 0.005;
        f.watcher_to_star_ratio = 0.0005;
        let (r, ev) = score(&f, &StarsThresholds::v1());
        assert!(r.score <= 30, "expected ≤30, got {}", r.score);
        let h1 = ev
            .iter()
            .find(|e| e.code == "low_activity_stargazer_share")
            .unwrap();
        // Verdict ceiling: Concerning, never HighRisk standalone.
        assert!(matches!(h1.verdict, Verdict::Concerning));
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
