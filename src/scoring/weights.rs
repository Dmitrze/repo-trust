//! Loader for custom `weights.toml` files.

use crate::models::ModuleWeights;

/// Load a `ModuleWeights` from a TOML file. Falls back to defaults if a
/// field is missing.
pub fn load(path: &std::path::Path) -> anyhow::Result<ModuleWeights> {
    let content = std::fs::read_to_string(path)?;
    let parsed: ModuleWeights = toml::from_str(&content)?;
    Ok(parsed)
}
