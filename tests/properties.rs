#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! Property-based tests for the scoring aggregate and Maintainer Gini.
//!
//! These five invariants are exercised against randomly generated inputs to
//! complement the deterministic unit / snapshot tests. Each property runs at
//! least 256 cases; together this file generates 1280+ test cases per `cargo
//! test` invocation.
//!
//! Properties:
//! 1. Aggregate score is always in `[0, 100]`.
//! 2. Aggregate is deterministic — same inputs always yield the same output.
//! 3. Aggregate is monotonically non-decreasing in any single module's score
//!    (others fixed, weights fixed).
//! 4. For modules whose scores are all `>= 50`, demoting confidence from `High`
//!    to `Low` never raises the aggregate above the `High` baseline (the
//!    confidence weighting of high scores cannot inflate the result).
//! 5. `features::maintainers::gini` is bounded `[0.0, 1.0]` for any input.

use proptest::prelude::*;
use repo_trust::features::maintainers::gini;
use repo_trust::models::{Confidence, ModuleResult, ModuleWeights};
use repo_trust::scoring::aggregate;
use std::collections::BTreeMap;

/// All five module names recognised by `aggregate::weight_for`. Module names
/// outside this set contribute zero weight, so we only sample inside it.
const MODULE_NAMES: &[&str] = &["stars", "activity", "maintainers", "adoption", "security"];

fn module_result_strategy() -> impl Strategy<Value = ModuleResult> {
    (
        prop::sample::select(MODULE_NAMES),
        0u8..=100u8,
        prop::sample::select(vec![Confidence::Low, Confidence::Medium, Confidence::High]),
    )
        .prop_map(|(name, score, confidence)| ModuleResult {
            module: (*name).to_string(),
            score,
            confidence,
            sub_scores: BTreeMap::new(),
            sample_size: None,
            missing_data: vec![],
        })
}

fn module_weights_strategy() -> impl Strategy<Value = ModuleWeights> {
    (
        0.0..=1.0f64,
        0.0..=1.0f64,
        0.0..=1.0f64,
        0.0..=1.0f64,
        0.0..=1.0f64,
    )
        .prop_map(|(s, a, m, ad, sec)| ModuleWeights {
            stars: s,
            activity: a,
            maintainers: m,
            adoption: ad,
            security: sec,
        })
}

/// Strategy for "high score" module results (score >= 50, confidence High)
/// used by the confidence-demotion property.
fn high_score_high_conf_strategy() -> impl Strategy<Value = ModuleResult> {
    (prop::sample::select(MODULE_NAMES), 50u8..=100u8).prop_map(|(name, score)| ModuleResult {
        module: (*name).to_string(),
        score,
        confidence: Confidence::High,
        sub_scores: BTreeMap::new(),
        sample_size: None,
        missing_data: vec![],
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Property 1 — Aggregate score is always within `[0, 100]`.
    #[test]
    fn aggregate_score_is_bounded(
        modules in proptest::collection::vec(module_result_strategy(), 0..6),
        weights in module_weights_strategy(),
    ) {
        let score = aggregate(&modules, &weights);
        prop_assert!(score <= 100, "score out of bounds: {score}");
    }

    /// Property 2 — Aggregate is deterministic: same inputs ⇒ same output.
    #[test]
    fn aggregate_is_deterministic(
        modules in proptest::collection::vec(module_result_strategy(), 0..6),
        weights in module_weights_strategy(),
    ) {
        let a = aggregate(&modules, &weights);
        let b = aggregate(&modules, &weights);
        prop_assert_eq!(a, b);
    }

    /// Property 3 — Increasing one module's score (others + weights fixed)
    /// must not DECREASE the aggregate. (Monotonicity in module score.)
    #[test]
    fn aggregate_is_monotonic_in_module_score(
        mut base_modules in proptest::collection::vec(module_result_strategy(), 1..6),
        weights in module_weights_strategy(),
        delta in 1u8..=20u8,
    ) {
        let baseline = aggregate(&base_modules, &weights);
        // Bump the first module's score by `delta`, saturating at 100.
        base_modules[0].score = base_modules[0].score.saturating_add(delta);
        let bumped = aggregate(&base_modules, &weights);
        prop_assert!(
            bumped >= baseline,
            "monotonicity violated: baseline={baseline}, bumped={bumped}, delta={delta}"
        );
    }

    /// Property 4 — For modules with score >= 50 and Confidence::High,
    /// demoting every confidence to Confidence::Low must NOT raise the
    /// aggregate above the High-confidence baseline.
    ///
    /// Rationale: confidence factors {Low=0.5, Med=0.75, High=1.0} appear in
    /// both numerator and denominator, but uniformly demoting confidence on a
    /// set of all-equal-confidence inputs is a no-op (factors cancel) and
    /// must therefore never inflate the score. This protects against future
    /// regressions where confidence weighting accidentally raises high-score
    /// aggregates.
    #[test]
    fn confidence_demotion_does_not_raise_aggregate_for_high_scores(
        modules_high in proptest::collection::vec(high_score_high_conf_strategy(), 1..6),
        weights in module_weights_strategy(),
    ) {
        let aggregate_high = aggregate(&modules_high, &weights);
        let modules_low: Vec<ModuleResult> = modules_high
            .iter()
            .map(|m| ModuleResult {
                confidence: Confidence::Low,
                ..m.clone()
            })
            .collect();
        let aggregate_low = aggregate(&modules_low, &weights);
        prop_assert!(
            aggregate_low <= aggregate_high,
            "Low confidence on high scores should NOT raise aggregate above High: \
             high={aggregate_high}, low={aggregate_low}"
        );
    }

    /// Property 5 — Maintainer Gini coefficient is bounded `[0.0, 1.0]` for
    /// any non-empty input. (Mirrors the existing Day-2 invariant but feeds
    /// the value through `u64::from(u32)` to exercise broader inputs without
    /// fresh `as` casts.)
    #[test]
    fn maintainer_gini_is_bounded(
        contributions in proptest::collection::vec(0u32..=10_000u32, 1..=50),
    ) {
        let values: Vec<u64> = contributions.iter().copied().map(u64::from).collect();
        let g = gini(&values);
        prop_assert!(
            (0.0..=1.0).contains(&g),
            "Gini out of bounds: {g}"
        );
    }
}
