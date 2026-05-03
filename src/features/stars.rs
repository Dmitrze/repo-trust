//! Star Authenticity features.
//!
//! Heuristics from StarScout (ICSE 2026) and Dagster's fake-star-detector (2023).
//! See `docs/methodology.md` for the full spec.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StarsFeatures {
    pub total_stars: u64,
    pub fork_to_star_ratio: f64,
    pub watcher_to_star_ratio: f64,
    pub low_activity_share: f64,
    pub lockstep_z_score: f64,
    pub sample_size: usize,
    pub median_stargazer_account_age_days: f64,
}
