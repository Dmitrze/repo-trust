//! Compute the overall trust score from per-module results.

use crate::models::{Confidence, ModuleResult, ModuleWeights};

/// Confidence-weighted aggregate of module scores.
///
/// ```text
/// trust_score = Σ (w_i × score_i × conf_i) / Σ (w_i × conf_i)
/// ```
///
/// where `conf_i ∈ {0.5, 0.75, 1.0}` for `{Low, Medium, High}`.
///
/// Returns 0 if no modules contributed (empty input or all weights zero).
#[must_use]
pub fn aggregate(modules: &[ModuleResult], weights: &ModuleWeights) -> u8 {
    let mut numerator = 0.0f64;
    let mut denominator = 0.0f64;

    // Iterate in a fixed order so floating-point summation is deterministic.
    let mut by_name: Vec<&ModuleResult> = modules.iter().collect();
    by_name.sort_by(|a, b| a.module.cmp(&b.module));

    for m in by_name {
        let w = weight_for(&m.module, weights);
        let conf = confidence_factor(m.confidence);
        numerator += w * f64::from(m.score) * conf;
        denominator += w * conf;
    }

    if denominator <= f64::EPSILON {
        return 0;
    }

    let raw = numerator / denominator;
    raw.round().clamp(0.0, 100.0) as u8
}

fn weight_for(module: &str, weights: &ModuleWeights) -> f64 {
    match module {
        "stars" => weights.stars,
        "activity" => weights.activity,
        "maintainers" => weights.maintainers,
        "adoption" => weights.adoption,
        "security" => weights.security,
        _ => 0.0,
    }
}

fn confidence_factor(c: Confidence) -> f64 {
    match c {
        Confidence::Low => 0.5,
        Confidence::Medium => 0.75,
        Confidence::High => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn module(name: &str, score: u8, confidence: Confidence) -> ModuleResult {
        ModuleResult {
            module: name.to_string(),
            score,
            confidence,
            sub_scores: BTreeMap::new(),
            sample_size: None,
            missing_data: vec![],
        }
    }

    #[test]
    fn empty_modules_aggregate_to_zero() {
        assert_eq!(aggregate(&[], &ModuleWeights::default()), 0);
    }

    #[test]
    fn five_perfect_high_confidence_aggregates_to_100() {
        let mods = vec![
            module("stars", 100, Confidence::High),
            module("activity", 100, Confidence::High),
            module("maintainers", 100, Confidence::High),
            module("adoption", 100, Confidence::High),
            module("security", 100, Confidence::High),
        ];
        assert_eq!(aggregate(&mods, &ModuleWeights::default()), 100);
    }

    #[test]
    fn low_confidence_module_contributes_less() {
        let high_only = vec![
            module("stars", 100, Confidence::High),
            module("activity", 0, Confidence::High),
        ];
        let mixed_conf = vec![
            module("stars", 100, Confidence::High),
            module("activity", 0, Confidence::Low),
        ];
        let mut w = ModuleWeights::default();
        w.stars = 0.5;
        w.activity = 0.5;
        w.maintainers = 0.0;
        w.adoption = 0.0;
        w.security = 0.0;

        let s_high = aggregate(&high_only, &w);
        let s_mixed = aggregate(&mixed_conf, &w);
        // Low-confidence "0" pulls less weight, so the mixed score is higher.
        assert!(s_mixed > s_high, "{s_mixed} should be > {s_high}");
    }
}
