use serde::{Deserialize, Serialize};

/// Verdict for a single piece of evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Verdict {
    /// Strong positive signal.
    Positive,
    /// Neither concerning nor reassuring.
    Neutral,
    /// Weak negative signal; worth noting.
    Concerning,
    /// Strong negative signal; impacts category.
    HighRisk,
}

/// A single, citable evidence item underpinning a module score.
///
/// Each module emits at least three of these. They are sorted by
/// `(module, code)` before serialization for deterministic output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Module name (e.g. "stars", "activity").
    pub module: String,

    /// Stable, machine-readable code (e.g. "low_activity_stargazer_share").
    pub code: String,

    /// Human-readable label.
    pub label: String,

    /// The observed value. Type varies by code.
    pub value: serde_json::Value,

    /// The threshold this value was compared against, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<serde_json::Value>,

    /// Verdict drawn from the comparison.
    pub verdict: Verdict,

    /// One- or two-sentence rationale for the verdict.
    pub rationale: String,
}
