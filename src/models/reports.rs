use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::evidence::EvidenceItem;
use super::repository::RepositorySummary;
use super::scores::{Category, Confidence, ModuleResult, ModuleWeights};

/// Execution mode echoed into the report for reproducibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Quick,
    Standard,
    Deep,
}

/// The frozen JSON report schema.
///
/// `schema_version` is bumped on any breaking change; see
/// `docs/scoring-model.md` for migration notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustReport {
    pub schema_version: String,

    pub repository: RepositorySummary,

    /// 0–100, weighted aggregate of module scores.
    pub overall_score: u8,

    pub overall_confidence: Confidence,

    pub category: Category,

    pub mode: Mode,

    pub modules: Vec<ModuleResult>,

    /// Sorted by (module, code) for determinism.
    pub evidence: Vec<EvidenceItem>,

    pub top_strengths: Vec<EvidenceItem>,

    pub top_concerns: Vec<EvidenceItem>,

    /// Caveats about partial data, sample size limits, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub caveats: Vec<String>,

    pub scoring_version: String,

    pub weights_used: ModuleWeights,

    #[serde(with = "crate::utils::time::iso8601")]
    pub snapshot_at: OffsetDateTime,

    /// Wall-clock seconds the scan took. Excluded from determinism guarantees.
    pub runtime_seconds: f64,
}
