//! Localhost web viewer (feature `web`).
//!
//! `axum` app on `localhost:8765` that renders cached [`TrustReport`]s as
//! HTML. Templates compile via [`askama`] (compile-time, escape-safe) and
//! static assets are embedded via [`rust_embed`] so the binary stays
//! single-file. Localhost-only by default; see `specs/web-viewer.md`.
//!
//! [`TrustReport`]: crate::models::TrustReport

pub mod handlers;
pub mod routes;

pub use routes::router;

use rust_embed::Embed;

/// Embedded static assets (CSS, future JS) served under `/static/*`.
#[derive(Debug, Embed)]
#[folder = "src/web/static/"]
pub struct Assets;
