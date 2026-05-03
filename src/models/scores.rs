use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Confidence band. Three values — see ADR-0008.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Five-bucket category derived from the overall score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    /// 85–100 — multiple modules score high, no significant warnings.
    Strong,
    /// 70–84 — generally healthy.
    Good,
    /// 50–69 — notable strengths and notable weaknesses.
    Mixed,
    /// 30–49 — significant concerns across multiple modules.
    Weak,
    /// 0–29 — strong negative signals; treat as suspicious.
    HighRisk,
}

impl Category {
    /// Bucket a score into a category.
    #[must_use]
    pub const fn from_score(score: u8) -> Self {
        match score {
            85..=100 => Self::Strong,
            70..=84 => Self::Good,
            50..=69 => Self::Mixed,
            30..=49 => Self::Weak,
            _ => Self::HighRisk,
        }
    }
}

/// Default + configurable module weights. See ADR-0006 for the v1 defaults.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModuleWeights {
    pub stars: f64,
    pub activity: f64,
    pub maintainers: f64,
    pub adoption: f64,
    pub security: f64,
}

impl Default for ModuleWeights {
    fn default() -> Self {
        Self {
            stars: 0.20,
            activity: 0.25,
            maintainers: 0.20,
            adoption: 0.20,
            security: 0.15,
        }
    }
}

impl ModuleWeights {
    /// Sum of all module weights. Should be ≈1.0; we don't enforce strictly
    /// because users may want to disable modules entirely (weight 0).
    #[must_use]
    pub fn total(&self) -> f64 {
        self.stars + self.activity + self.maintainers + self.adoption + self.security
    }
}

/// Result from a single module's scorer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub module: String,

    /// 0–100
    pub score: u8,

    pub confidence: Confidence,

    /// Named sub-scores (e.g. for stars: low_activity_share, lockstep_timing).
    pub sub_scores: BTreeMap<String, u8>,

    /// For sample-based modules: actual sample size achieved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_size: Option<usize>,

    /// Inputs we expected but didn't get.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_data: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_buckets() {
        assert_eq!(Category::from_score(100), Category::Strong);
        assert_eq!(Category::from_score(85), Category::Strong);
        assert_eq!(Category::from_score(84), Category::Good);
        assert_eq!(Category::from_score(70), Category::Good);
        assert_eq!(Category::from_score(69), Category::Mixed);
        assert_eq!(Category::from_score(50), Category::Mixed);
        assert_eq!(Category::from_score(49), Category::Weak);
        assert_eq!(Category::from_score(30), Category::Weak);
        assert_eq!(Category::from_score(29), Category::HighRisk);
        assert_eq!(Category::from_score(0), Category::HighRisk);
    }

    #[test]
    fn default_weights_sum_to_one() {
        let w = ModuleWeights::default();
        let total = w.total();
        assert!((total - 1.0).abs() < 1e-9, "weights total = {total}");
    }

    #[test]
    fn confidence_orders_correctly() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
    }
}
