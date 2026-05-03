//! Maintainer Health features.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaintainerFeatures {
    pub active_maintainers_last_year: u64,
    pub commit_gini: f64,
    pub review_gini: f64,
    pub bus_factor_proxy: u64,
    pub contributor_retention_rate: f64,
    pub median_maintainer_response_hours: Option<f64>,
    pub has_codeowners: bool,
    pub has_governance_doc: bool,
}
