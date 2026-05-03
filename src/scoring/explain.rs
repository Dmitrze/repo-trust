//! Build the human-friendly explanation list (top strengths, top concerns)
//! from the full evidence vector.

use crate::models::{EvidenceItem, Verdict};

/// Pick the top N positive and concerning evidence items, in deterministic
/// order. Used for the headline section of every report.
#[must_use]
pub fn top_strengths_and_concerns(
    evidence: &[EvidenceItem],
    n: usize,
) -> (Vec<EvidenceItem>, Vec<EvidenceItem>) {
    let mut strengths: Vec<&EvidenceItem> = evidence
        .iter()
        .filter(|e| matches!(e.verdict, Verdict::Positive))
        .collect();
    let mut concerns: Vec<&EvidenceItem> = evidence
        .iter()
        .filter(|e| matches!(e.verdict, Verdict::Concerning | Verdict::HighRisk))
        .collect();

    // Deterministic order: by (module, code).
    strengths.sort_by(|a, b| {
        (a.module.as_str(), a.code.as_str()).cmp(&(b.module.as_str(), b.code.as_str()))
    });
    concerns.sort_by(|a, b| {
        (a.module.as_str(), a.code.as_str()).cmp(&(b.module.as_str(), b.code.as_str()))
    });

    (
        strengths.into_iter().take(n).cloned().collect(),
        concerns.into_iter().take(n).cloned().collect(),
    )
}
