//! Activity Health features.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityFeatures {
    pub commits_last_30d: u64,
    pub commits_last_90d: u64,
    pub commits_last_365d: u64,
    pub days_since_last_commit: u64,
    pub days_since_last_release: Option<u64>,
    pub release_count_last_year: u64,
    pub median_issue_first_response_hours: Option<f64>,
    pub median_pr_review_hours: Option<f64>,
    pub active_contributors_last_90d: u64,
    pub commit_count_variance_18m: f64,
}
