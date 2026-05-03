use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::scores::ModuleWeights;
use crate::api::github::Client as GithubClient;
use crate::cli::scan::Mode as CliMode;
use crate::storage::Cache;

/// Public-facing repository summary in the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySummary {
    pub full_name: String,
    pub url: String,
    pub default_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_language: Option<String>,
    pub stars: u64,
    #[serde(with = "crate::utils::time::iso8601")]
    pub snapshot_at: OffsetDateTime,
}

/// Internal context shared across the run.
///
/// Not serialised — holds API clients, cache handles, etc. Cheap to clone;
/// the [`Cache`] and [`GithubClient`] are `Arc`-internal.
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    pub full_name: String,
    pub canonical_url: url::Url,
    pub mode: CliMode,
    pub scoring_version: semver::Version,
    pub weights: ModuleWeights,
    pub rng_seed: u64,
    pub snapshot_at: OffsetDateTime,
    pub cache: Cache,
    pub github: GithubClient,
}

impl RepositoryContext {
    /// `(owner, repo)` split of `full_name`. Panics only if the constructor
    /// accepted an invalid identifier — call sites already normalize via
    /// `utils::repo_url::parse`.
    #[must_use]
    pub fn owner_repo(&self) -> (&str, &str) {
        self.full_name
            .split_once('/')
            .expect("full_name is owner/repo per utils::repo_url::parse")
    }
}
