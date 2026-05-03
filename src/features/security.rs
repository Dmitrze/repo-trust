//! Security & Readiness features.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityFeatures {
    pub scorecard_score: Option<f64>,
    pub scorecard_checks_failed: Vec<String>,
    pub osv_open_advisories: u64,
    pub has_security_md: bool,
    pub has_contributing_md: bool,
    pub has_code_of_conduct: bool,
    pub has_license: bool,
    pub has_codeowners: bool,
    pub has_ci_workflow: bool,
    pub semver_consistent: bool,
}
