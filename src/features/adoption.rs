//! Adoption Signals features.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdoptionFeatures {
    pub github_dependents: Option<u64>,
    pub weekly_downloads: Option<u64>,
    pub package_systems: Vec<String>,
    pub awesome_list_mentions: u64,
    pub doc_maturity_score: f64,
}
