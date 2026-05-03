//! Layered configuration loader.
//!
//! Sources, in increasing priority:
//! 1. Built-in defaults
//! 2. `~/.repo-trust/config.toml`
//! 3. `./.repo-trust.toml`
//! 4. Environment variables (`REPO_TRUST_*`)
//! 5. CLI flags

pub mod loader;
