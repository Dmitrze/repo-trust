//! scorecard.dev REST client with ETag-aware caching.
//!
//! Federated read-only client for the Security & Readiness module. Hits
//! `https://api.scorecard.dev/projects/github.com/{owner}/{repo}` and returns
//! the latest Scorecard run, or `Ok(None)` when the repository has not yet
//! been scored. See [`specs/scorecard-client.md`](../../specs/scorecard-client.md)
//! and [`docs/api-notes.md`](../../docs/api-notes.md#scorecarddev).
//!
//! # Example
//!
//! ```no_run
//! use repo_trust::api::scorecard::Client;
//! use repo_trust::storage::Cache;
//!
//! # async fn demo() -> anyhow::Result<()> {
//! let http = reqwest::Client::builder().build()?;
//! let cache = Cache::open("/tmp/repo-trust-cache.db")?;
//! let client = Client::new(http, cache);
//! match client.get("prometheus", "prometheus").await? {
//!     Some(report) => println!("score = {}", report.score),
//!     None => println!("not yet scored"),
//! }
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

use crate::storage::Cache;

/// Base URL for the scorecard.dev REST API. Overridable for tests via
/// [`Client::with_base_url`].
pub const SCORECARD_API_BASE: &str = "https://api.scorecard.dev";

/// Cache TTL for Scorecard reports — Scorecard re-runs weekly per
/// `architecture.md` §6.3.
const TTL_SCORECARD: Duration = Duration::from_secs(7 * 24 * 3600);

/// Errors surfaced by the client. The CLI maps these onto exit codes per
/// architecture §8.
///
/// `404 Not Found` is *not* an error here — it is a "Scorecard has not yet
/// scored this repository" signal and surfaces as `Ok(None)` from
/// [`Client::get`]. The Security & Readiness module degrades to doc-presence
/// only with Low confidence in that case.
#[derive(Debug, Error)]
pub enum ScorecardError {
    /// Any non-200, non-304, non-404 response (4xx other than 404, or 5xx).
    #[error("scorecard.dev returned {status}: {body}")]
    Other { status: u16, body: String },
}

/// Cheap-to-clone scorecard.dev client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    cache: Cache,
}

impl Client {
    /// Build a new client with the default scorecard.dev base URL.
    /// Scorecard is fully public — no authentication is required or accepted.
    #[must_use]
    pub fn new(http: HttpClient, cache: Cache) -> Self {
        Self {
            http,
            base_url: SCORECARD_API_BASE.to_string(),
            cache,
        }
    }

    /// Override the API base URL — wiremock fixtures use this.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// `GET /projects/github.com/{owner}/{repo}` — the latest Scorecard run.
    ///
    /// Returns:
    /// - `Ok(Some(report))` when Scorecard has scored this repository.
    /// - `Ok(None)` when Scorecard has *not* scored this repository (HTTP
    ///   404). This is a normal "not yet scored" signal.
    /// - `Err(_)` on parse failures, transport failures, or non-404 HTTP
    ///   errors (4xx other than 404, or 5xx).
    pub async fn get(&self, owner: &str, repo: &str) -> Result<Option<ScorecardReport>> {
        let key = format!("scorecard:projects/github.com/{owner}/{repo}");
        let path = format!("/projects/github.com/{owner}/{repo}");
        let body = match self.fetch_json(&key, &path, TTL_SCORECARD).await? {
            Some(b) => b,
            None => return Ok(None),
        };
        let parsed: ScorecardReport =
            serde_json::from_slice(&body).context("parse ScorecardReport")?;
        Ok(Some(parsed))
    }

    // ─── Internals ────────────────────────────────────────────────────────

    /// ETag-aware fetch lifecycle. Hits cache first; on miss/stale sends a
    /// conditional `GET` with `If-None-Match`; on 304 reuses the cached body
    /// and refreshes its TTL; on 200 stores the new body+etag.
    ///
    /// Returns `Ok(None)` for HTTP 404 (Scorecard's "not yet scored" signal),
    /// `Ok(Some(body))` for 200/304, and `Err` for everything else.
    async fn fetch_json(
        &self,
        cache_key: &str,
        path: &str,
        ttl: Duration,
    ) -> Result<Option<Vec<u8>>> {
        let cached = self.cache.get(cache_key)?;
        if let Some(entry) = &cached {
            if !entry.is_stale() {
                return Ok(Some(entry.body.clone()));
            }
        }
        let cached_etag = cached.as_ref().and_then(|e| e.etag.clone());
        let cached_body = cached.as_ref().map(|e| e.body.clone());

        let url = format!("{}{}", self.base_url, path);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("repo-trust"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        if let Some(e) = &cached_etag {
            headers.insert(IF_NONE_MATCH, HeaderValue::from_str(e)?);
        }

        let resp = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        match resp.status() {
            StatusCode::NOT_MODIFIED => {
                let body = cached_body
                    .ok_or_else(|| anyhow::anyhow!("304 received without cached body"))?;
                // Refresh both fetched_at and expires_at by re-putting with
                // the same etag + body but a fresh TTL.
                self.cache
                    .put(cache_key, cached_etag.as_deref(), &body, ttl)?;
                Ok(Some(body))
            },
            StatusCode::OK => {
                let new_etag = resp
                    .headers()
                    .get("etag")
                    .and_then(|h| h.to_str().ok())
                    .map(str::to_string);
                let body = resp.bytes().await?;
                self.cache.put(cache_key, new_etag.as_deref(), &body, ttl)?;
                Ok(Some(body.to_vec()))
            },
            StatusCode::NOT_FOUND => Ok(None),
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(ScorecardError::Other {
                    status: s.as_u16(),
                    body,
                }
                .into())
            },
        }
    }
}

// ─── DTOs (only the fields the security module uses) ──────────────────────

/// One Scorecard run for a repository.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScorecardReport {
    /// When the Scorecard run was executed.
    #[serde(
        deserialize_with = "deserialize_scorecard_date",
        serialize_with = "time::serde::iso8601::serialize"
    )]
    pub date: OffsetDateTime,
    /// The repository the run was performed against.
    pub repo: ScorecardRepoRef,
    /// Aggregate Scorecard score in the range `0.0..=10.0`.
    pub score: f64,
    /// Per-check results that compose the aggregate score.
    pub checks: Vec<CheckResult>,
}

/// Repository identifier echoed back by Scorecard.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScorecardRepoRef {
    /// Fully qualified repo name, e.g. `github.com/prometheus/prometheus`.
    pub name: String,
    /// Commit SHA the run was performed against.
    pub commit: String,
}

/// A single Scorecard check result.
///
/// `score` may be `-1` when Scorecard could not gather enough data to evaluate
/// the check (for example a repository with no releases for the
/// `Signed-Releases` check). Otherwise it lies in `0..=10`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub score: i32,
    pub reason: String,
    /// The raw Scorecard `documentation` object (typically `{ "short": …,
    /// "url": … }`). Kept as a [`serde_json::Value`] so we don't churn when
    /// Scorecard adds fields.
    #[serde(default)]
    pub documentation: serde_json::Value,
}

/// Tolerant deserialiser for the `date` field returned by scorecard.dev.
///
/// scorecard.dev does not commit to a single ISO-8601 sub-format — runs
/// have shipped with all of:
///
/// 1. RFC 3339 with no fractional seconds       `2026-04-30T00:00:00Z`
/// 2. RFC 3339 with explicit `+00:00` offset    `2026-04-30T00:00:00+00:00`
/// 3. Full extended ISO-8601                    `2026-04-30T00:00:00.123456789Z`
/// 4. Date-only                                 `2026-04-30`
///
/// As of 2026-04 the public API returns format (4) for every run we've
/// observed, but we don't assume that won't change again. This
/// deserialiser tries the wider RFC 3339 grammar first, then the
/// extended ISO-8601 parser, then a date-only fallback (promoted to
/// midnight UTC).
fn deserialize_scorecard_date<'de, D>(de: D) -> Result<OffsetDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    use time::format_description::well_known::{Iso8601, Rfc3339};

    let raw = String::deserialize(de)?;

    // Strategy 1 — RFC 3339 (covers the no-fractional-seconds and
    // explicit-offset cases that scorecard.dev emits).
    if let Ok(dt) = OffsetDateTime::parse(&raw, &Rfc3339) {
        return Ok(dt);
    }
    // Strategy 2 — full extended ISO-8601, in case scorecard.dev does
    // emit nanoseconds for some checks.
    if let Ok(dt) = OffsetDateTime::parse(&raw, &Iso8601::DEFAULT) {
        return Ok(dt);
    }
    // Strategy 3 — date-only (`YYYY-MM-DD`), promoted to midnight UTC.
    if let Ok(d) = time::Date::parse(
        &raw,
        time::macros::format_description!("[year]-[month]-[day]"),
    ) {
        return Ok(d.midnight().assume_utc());
    }

    Err(D::Error::custom(format!(
        "scorecard.dev `date` field is not a recognised ISO-8601 / RFC 3339 / \
         date-only string: {raw:?}"
    )))
}

#[cfg(test)]
mod date_parsing_tests {
    use super::*;

    /// Real-world fixtures captured from `api.scorecard.dev` — the file
    /// names map to the upstream `github.com/<owner>/<repo>` paths. Kept
    /// as a hard-coded list to avoid pulling a `glob` dev-dep in for one
    /// test. Paths are anchored at `CARGO_MANIFEST_DIR` so the test
    /// works under any cwd (CI runners do not always cd into the
    /// package root before invoking cargo test).
    const FIXTURES: &[&str] = &[
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/scorecard/clap-rs_clap.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/scorecard/octocat_hello-world.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/scorecard/rust-lang_rust.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/scorecard/tokio-rs_tokio.json"
        ),
    ];

    #[test]
    fn parses_rfc3339_no_fractional_seconds() {
        let s = r#"{
            "date":"2026-04-30T00:00:00Z",
            "repo":{"name":"github.com/o/r","commit":"abc"},
            "score":7.5,
            "checks":[]
        }"#;
        let r: ScorecardReport = serde_json::from_str(s).unwrap();
        assert_eq!(r.score, 7.5);
    }

    #[test]
    fn parses_rfc3339_with_explicit_offset() {
        let s = r#"{
            "date":"2026-04-30T00:00:00+00:00",
            "repo":{"name":"github.com/o/r","commit":"abc"},
            "score":7.5,
            "checks":[]
        }"#;
        serde_json::from_str::<ScorecardReport>(s).unwrap();
    }

    #[test]
    fn parses_full_extended_iso8601_with_nanos() {
        let s = r#"{
            "date":"2026-04-30T00:00:00.123456789Z",
            "repo":{"name":"github.com/o/r","commit":"abc"},
            "score":7.5,
            "checks":[]
        }"#;
        serde_json::from_str::<ScorecardReport>(s).unwrap();
    }

    #[test]
    fn parses_date_only_as_midnight_utc() {
        let s = r#"{
            "date":"2026-04-30",
            "repo":{"name":"github.com/o/r","commit":"abc"},
            "score":7.5,
            "checks":[]
        }"#;
        let r: ScorecardReport = serde_json::from_str(s).unwrap();
        assert_eq!(r.date.hour(), 0);
        assert_eq!(r.date.minute(), 0);
        assert_eq!(r.date.second(), 0);
    }

    #[test]
    fn rejects_garbage() {
        let s = r#"{
            "date":"not a date",
            "repo":{"name":"github.com/o/r","commit":"abc"},
            "score":7.5,
            "checks":[]
        }"#;
        assert!(serde_json::from_str::<ScorecardReport>(s).is_err());
    }

    /// Real-world fixture sweep — every captured `api.scorecard.dev`
    /// payload must round-trip through our DTO without error and yield
    /// a Scorecard score in the documented range.
    #[test]
    fn real_world_fixtures_round_trip() {
        for path in FIXTURES {
            let body =
                std::fs::read(path).unwrap_or_else(|e| panic!("missing fixture {path}: {e}"));
            // Defensive: skip non-JSON bodies (e.g. a 404 HTML page if the
            // fixture was captured against a never-scored repo).
            if !body.starts_with(b"{") {
                continue;
            }
            let r: ScorecardReport = serde_json::from_slice(&body)
                .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
            assert!(
                (0.0..=10.0).contains(&r.score),
                "score out of range in {path}: {}",
                r.score
            );
        }
    }
}
