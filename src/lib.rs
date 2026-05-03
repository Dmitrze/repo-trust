//! Repo Trust — a CLI tool that produces a multi-dimensional Trust Report
//! for any public GitHub repository.
//!
//! See [the README](https://github.com/Dmitrze/repo-trust) for an overview.
//!
//! # Crate organisation
//!
//! - [`cli`] — command-line interface and argument parsing
//! - [`api`] — HTTP clients for GitHub, deps.dev, Scorecard, OSV
//! - [`collectors`] — module-specific data collection from APIs
//! - [`features`] — normalisation of raw data into per-module feature structs
//! - [`modules`] — the five trust modules (stars, activity, maintainers, adoption, security)
//! - [`scoring`] — pure scoring functions; aggregates module results
//! - [`models`] — shared data types (reports, evidence, scores)
//! - [`reports`] — output writers (terminal, JSON, Markdown, CSV)
//! - [`storage`] — SQLite-backed cache and connection pool
//! - [`config`] — layered configuration loader
//! - [`utils`] — cross-cutting helpers (sampling, time, ratelimit, tracing)
//!
//! # Determinism
//!
//! See ADR-0007. Same inputs + same upstream API state ⇒ byte-identical JSON.
//! All sampling uses [`rand_chacha::ChaCha20Rng`] seeded from
//! `(repo, scoring_version)` via blake3.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
// NOTE: clippy::pedantic is intentionally NOT enabled at the crate level
// during pre-alpha. The crate skeleton is intentionally sparse and pedantic
// produces a flood of noise that obscures real issues. Re-enable once
// Phase 1 modules ship with real implementations.

pub mod api;
pub mod cli;
pub mod collectors;
pub mod config;
pub mod features;
pub mod models;
pub mod modules;
pub mod reports;
pub mod scoring;
pub mod storage;
pub mod utils;

#[cfg(feature = "web")]
pub mod web;

/// Library version string — useful for `--version` output.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// SemVer of the scoring model. Bumped independently of the CLI version.
///
/// See `docs/scoring-model.md` for the change log.
pub const SCORING_VERSION: &str = "1.0.0";

/// JSON report schema version. Bumped on any breaking schema change.
pub const REPORT_SCHEMA_VERSION: &str = "1.0.0";
