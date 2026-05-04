//! Router construction for the localhost web viewer.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::storage::Cache;

use super::handlers;

/// Shared application state for handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    pub cache: Arc<Cache>,
    pub allow_scan: bool,
}

/// Build the axum [`Router`] for `repo-trust serve`.
///
/// `allow_scan` gates `POST /scans`; when `false` the route returns
/// `405 Method Not Allowed` (DNS-rebinding mitigation per
/// `specs/web-viewer.md` §2).
pub fn router(cache: Cache, allow_scan: bool) -> Router {
    let state = AppState {
        cache: Arc::new(cache),
        allow_scan,
    };

    Router::new()
        .route("/", get(handlers::index))
        .route("/reports/:owner/:name", get(handlers::report))
        .route("/api/reports/:owner/:name", get(handlers::report_json))
        .route("/scans", post(handlers::scan))
        .route("/static/*path", get(handlers::static_asset))
        .fallback(handlers::not_found)
        .with_state(state)
}
