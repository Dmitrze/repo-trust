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
//
// Pedantic-clippy lint posture for v0.1.0 (Day 5 polish per
// `docs/day-5-polish.md` §1):
//
// We enable `clippy::pedantic` at the warn level (CI gate) but
// crate-level allow specific lints whose firings are domain-justified
// patterns rather than real defects:
//   - `cast_possible_truncation` / `cast_sign_loss` / `cast_precision_loss`
//     fire on the score arithmetic where the input is already
//     `.clamp(0.0, 100.0)`-bounded (so `as u8` is safe), on `i64→u64`
//     after `.max(0)`, and on `usize→f64` for vec lengths bounded by
//     the rate-limit budget. Local rationale comments at use sites
//     document each.
//   - `cast_possible_wrap` fires on `usize→i64` for date arithmetic
//     where the series length is bounded by the sample window.
//   - `must_use_candidate` is a stylistic preference; we mark
//     fallible-by-design return values explicitly elsewhere.
//   - `missing_errors_doc` / `missing_panics_doc`: we document errors
//     and panics in the function body comment rather than via separate
//     rustdoc sections; pedantic's heuristic is over-broad.
//   - `module_name_repetitions`: our module-name → type-name pattern
//     (e.g. `scoring::activity::ActivityThresholds`) is intentional.
//   - `result_large_err`: `figment::Error` is large but we don't
//     control its layout; boxing it crate-wide has no benefit.
//   - `struct_excessive_bools`: clap-derived CLI args structs naturally
//     accumulate boolean flags.
//   - `similar_names`: deps.dev DTO field names mirror the upstream
//     wire format and are intentionally close.
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::result_large_err,
    clippy::struct_excessive_bools,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::unused_async,
    clippy::unreadable_literal,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::match_same_arms,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::uninlined_format_args,
    clippy::unnecessary_debug_formatting
)]

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
