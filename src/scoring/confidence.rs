//! Overall confidence calculation.

use crate::models::{Confidence, ModuleResult, ModuleWeights};

/// Overall confidence is the **minimum** confidence across modules whose
/// weight contributes more than 10% of the total weight.
///
/// Rationale (ADR-0008): one low-confidence dominant module should not
/// be hidden by other high-confidence modules.
#[must_use]
pub fn overall_confidence(modules: &[ModuleResult], weights: &ModuleWeights) -> Confidence {
    let total = weights.total().max(f64::EPSILON);
    let mut min_conf = Confidence::High;

    for m in modules {
        let w = match m.module.as_str() {
            "stars" => weights.stars,
            "activity" => weights.activity,
            "maintainers" => weights.maintainers,
            "adoption" => weights.adoption,
            "security" => weights.security,
            _ => 0.0,
        };
        if w / total > 0.10 && m.confidence < min_conf {
            min_conf = m.confidence;
        }
    }

    min_conf
}
