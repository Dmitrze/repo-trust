//! Layered configuration.
//!
//! Sources, in increasing priority:
//! 1. Built-in defaults (embedded `default.toml` via `include_str!`).
//! 2. User config: `~/.repo-trust/config.toml`.
//! 3. Project config: `./.repo-trust.toml`.
//! 4. Environment variables (`REPO_TRUST_*` with `__` between section/field).
//! 5. CLI flags (highest priority).
//!
//! See [`specs/config-loader.md`](../../specs/config-loader.md) and
//! [`docs/architecture.md` §11](../../docs/architecture.md).

pub mod loader;

use serde::{Deserialize, Serialize};

pub use loader::{load, load_from_str};

/// Fully-resolved Repo Trust configuration. All fields are optional in the
/// raw TOML; missing fields fall back to [`Default::default`].
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub github: GithubConfig,
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub weights: WeightsConfig,
    #[serde(default)]
    pub stars: StarsConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GithubConfig {
    pub token_env: String,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            token_env: "GITHUB_TOKEN".to_string(),
        }
    }
}

impl GithubConfig {
    /// Read the token from the configured env var; returns `None` if unset
    /// or empty.
    #[must_use]
    pub fn resolve_token(&self) -> Option<String> {
        std::env::var(&self.token_env)
            .ok()
            .filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScanConfig {
    pub default_mode: String,
    pub default_modules: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            default_mode: "standard".to_string(),
            default_modules: vec![
                "stars".into(),
                "activity".into(),
                "maintainers".into(),
                "adoption".into(),
                "security".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct WeightsConfig {
    pub stars: f64,
    pub activity: f64,
    pub maintainers: f64,
    pub adoption: f64,
    pub security: f64,
}

impl Default for WeightsConfig {
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

impl From<WeightsConfig> for crate::models::ModuleWeights {
    fn from(c: WeightsConfig) -> Self {
        Self {
            stars: c.stars,
            activity: c.activity,
            maintainers: c.maintainers,
            adoption: c.adoption,
            security: c.security,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct StarsConfig {
    pub sample_size_quick: usize,
    pub sample_size_standard: usize,
    pub sample_size_deep: usize,
}

impl Default for StarsConfig {
    fn default() -> Self {
        Self {
            sample_size_quick: 0,
            sample_size_standard: 200,
            sample_size_deep: 2000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputConfig {
    pub default_formats: Vec<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default_formats: vec!["terminal".into(), "json".into()],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CacheConfig {
    /// Tilde expansion handled by [`CacheConfig::resolved_path`].
    pub path: String,
    pub max_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            path: "~/.repo-trust/cache.db".to_string(),
            max_size_mb: 500,
        }
    }
}

impl CacheConfig {
    /// Expand a leading `~/` to the user's home directory.
    #[must_use]
    pub fn resolved_path(&self) -> std::path::PathBuf {
        if let Some(rest) = self.path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        }
        std::path::PathBuf::from(&self.path)
    }
}
