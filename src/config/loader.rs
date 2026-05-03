//! `figment`-based config loader.
//!
//! See [`specs/config-loader.md`](../../specs/config-loader.md).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::Serialize;

use super::Config;

/// Embedded default configuration. Source of truth for every field's default
/// value (see also `Default` impls on the typed structs).
const DEFAULT_TOML: &str = include_str!("default.toml");

/// Load the resolved configuration in the standard precedence order:
/// defaults → user file → project file → env → CLI overrides.
///
/// `cli_overrides` is any `Serialize` value with the same shape as [`Config`]
/// (or a partial overlay). Pass [`None`] when no CLI overrides apply.
pub fn load<T: Serialize>(cli_overrides: Option<T>) -> Result<Config> {
    let user_path = dirs::home_dir().map(|h| h.join(".repo-trust/config.toml"));
    let project_path = PathBuf::from(".repo-trust.toml");
    load_layered(
        DEFAULT_TOML,
        user_path.as_deref(),
        Some(project_path.as_path()),
        cli_overrides,
    )
}

/// Construct a `Config` from a single TOML string. Convenience for tests and
/// `repo-trust config show`-style introspection.
pub fn load_from_str(toml_text: &str) -> Result<Config> {
    let cfg: Config = Figment::new()
        .merge(Toml::string(DEFAULT_TOML))
        .merge(Toml::string(toml_text))
        .extract()
        .context("loading config from string")?;
    Ok(cfg)
}

/// Internal builder. The two file paths are accepted as `Option<&Path>`; the
/// loader skips a file silently when it does not exist (so a fresh install
/// works without a user config).
pub(crate) fn load_layered<T: Serialize>(
    defaults_toml: &str,
    user_file: Option<&Path>,
    project_file: Option<&Path>,
    cli_overrides: Option<T>,
) -> Result<Config> {
    let mut fig = Figment::new().merge(Toml::string(defaults_toml));

    if let Some(p) = user_file {
        if p.exists() {
            fig = fig.merge(Toml::file(p));
        }
    }
    if let Some(p) = project_file {
        if p.exists() {
            fig = fig.merge(Toml::file(p));
        }
    }

    // Env vars: REPO_TRUST_WEIGHTS__ACTIVITY=0.30 → weights.activity = 0.30
    fig = fig.merge(Env::prefixed("REPO_TRUST_").split("__"));

    if let Some(overrides) = cli_overrides {
        fig = fig.merge(Serialized::defaults(overrides));
    }

    fig.extract().context("loading layered config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, WeightsConfig};
    use figment::Jail;
    use std::path::Path;

    #[test]
    fn defaults_apply_when_no_overrides_present() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            let cfg: Config =
                load_layered(DEFAULT_TOML, None, None, None::<()>).map_err(|e| e.to_string())?;
            assert_eq!(cfg.weights, WeightsConfig::default());
            assert_eq!(cfg.scan.default_mode, "standard");
            assert_eq!(cfg.github.token_env, "GITHUB_TOKEN");
            assert_eq!(cfg.cache.max_size_mb, 500);
            Ok(())
        });
    }

    #[test]
    fn user_file_overrides_defaults() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_file("user.toml", "[weights]\nactivity = 0.40\n")?;
            let user = Path::new("user.toml");
            let cfg: Config = load_layered(DEFAULT_TOML, Some(user), None, None::<()>)
                .map_err(|e| e.to_string())?;
            assert!((cfg.weights.activity - 0.40).abs() < 1e-9);
            // Other fields fall back to defaults.
            assert!((cfg.weights.stars - 0.20).abs() < 1e-9);
            Ok(())
        });
    }

    #[test]
    fn project_file_overrides_user_file() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_file("user.toml", "[weights]\nactivity = 0.40\n")?;
            jail.create_file("project.toml", "[weights]\nactivity = 0.50\n")?;
            let cfg = load_layered(
                DEFAULT_TOML,
                Some(Path::new("user.toml")),
                Some(Path::new("project.toml")),
                None::<()>,
            )
            .map_err(|e| e.to_string())?;
            assert!((cfg.weights.activity - 0.50).abs() < 1e-9);
            Ok(())
        });
    }

    #[test]
    fn env_overrides_files() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_file("project.toml", "[weights]\nactivity = 0.50\n")?;
            jail.set_env("REPO_TRUST_WEIGHTS__ACTIVITY", "0.60");
            let cfg = load_layered(
                DEFAULT_TOML,
                None,
                Some(Path::new("project.toml")),
                None::<()>,
            )
            .map_err(|e| e.to_string())?;
            assert!((cfg.weights.activity - 0.60).abs() < 1e-9);
            Ok(())
        });
    }

    #[test]
    fn cli_overrides_beat_env() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("REPO_TRUST_WEIGHTS__ACTIVITY", "0.60");
            let mut overrides = Config::default();
            overrides.weights.activity = 0.70;
            let cfg = load_layered(DEFAULT_TOML, None, None, Some(overrides))
                .map_err(|e| e.to_string())?;
            assert!((cfg.weights.activity - 0.70).abs() < 1e-9);
            Ok(())
        });
    }

    #[test]
    fn malformed_toml_returns_actionable_error() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_file("bad.toml", "[weights\nactivity = nope")?;
            let err = load_layered(DEFAULT_TOML, None, Some(Path::new("bad.toml")), None::<()>)
                .expect_err("malformed TOML should error");
            // The error chain should mention the file or the figment provider.
            let msg = format!("{err:#}");
            assert!(
                msg.to_lowercase().contains("loading layered config")
                    || msg.to_lowercase().contains("toml"),
                "expected actionable error message, got: {msg}"
            );
            Ok(())
        });
    }

    #[test]
    fn type_mismatch_in_user_config_errors() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_file("user.toml", "[weights]\nactivity = \"high\"\n")?;
            let err = load_layered(DEFAULT_TOML, Some(Path::new("user.toml")), None, None::<()>)
                .expect_err("type mismatch should error");
            let msg = format!("{err:#}").to_lowercase();
            assert!(
                msg.contains("activity") || msg.contains("float") || msg.contains("number"),
                "expected type-mismatch error, got: {msg}"
            );
            Ok(())
        });
    }

    #[test]
    fn weights_config_into_module_weights() {
        let wc = WeightsConfig::default();
        let mw: crate::models::ModuleWeights = wc.into();
        assert!((mw.total() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn token_resolution_returns_none_when_unset() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            let gh = crate::config::GithubConfig::default();
            assert!(gh.resolve_token().is_none());
            Ok(())
        });
    }

    #[test]
    fn token_resolution_returns_value_when_set() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("GITHUB_TOKEN", "ghp_test_value");
            let gh = crate::config::GithubConfig::default();
            assert_eq!(gh.resolve_token().as_deref(), Some("ghp_test_value"));
            Ok(())
        });
    }

    #[test]
    fn cache_config_expands_tilde() {
        let cc = crate::config::CacheConfig::default();
        let resolved = cc.resolved_path();
        // The default path starts with "~/", which should be expanded.
        assert!(
            !resolved.to_string_lossy().starts_with("~/"),
            "tilde should be expanded; got {resolved:?}"
        );
    }
}
