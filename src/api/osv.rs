//! OSV.dev federated query client.
//!
//! Federates `POST https://api.osv.dev/v1/query` (see
//! [`docs/api-notes.md`](../../docs/api-notes.md) §OSV.dev). Takes package
//! coordinates `(name, ecosystem, version)` and returns the list of
//! [`OsvAdvisory`] entries affecting that version with **withdrawn
//! advisories filtered client-side** and the remainder **sorted by `id`**
//! for determinism per `specs/osv-client.md`.
//!
//! Phase 2: OSV is consulted by the Security & Readiness module once the
//! Adoption module supplies the repo→packages map via deps.dev. Day 2 ships
//! only the client; Day 3 wires the collector.

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

use crate::storage::Cache;

/// Default base URL for the OSV.dev REST API. Overridable via
/// [`Client::with_base_url`] for wiremock fixtures.
pub const OSV_API_BASE: &str = "https://api.osv.dev";

/// Cache TTL for OSV query responses (architecture §6.3 → 6 hours).
const TTL_OSV_QUERY: Duration = Duration::from_secs(6 * 3600);

/// Errors surfaced by the OSV client. The CLI maps these onto exit codes per
/// architecture §8. OSV always answers `200 OK` (with possibly empty
/// `vulns`) for valid queries — there is no `NotFound` variant.
#[derive(Debug, Error)]
pub enum OsvError {
    #[error("osv returned {status}: {body}")]
    Other { status: u16, body: String },
}

/// Coordinates identifying a single package version, used as the OSV query
/// input. `ecosystem` follows the OSV-schema vocabulary
/// (`npm`, `crates.io`, `PyPI`, `Go`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCoords {
    pub name: String,
    pub ecosystem: String,
    pub version: String,
}

/// One advisory record from the OSV `/v1/query` response, narrowed to the
/// fields the Security module consumes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsvAdvisory {
    /// Stable OSV identifier (e.g. `GHSA-aaaa-bbbb-cccc`, `CVE-2024-1234`).
    pub id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(with = "time::serde::iso8601")]
    pub modified: OffsetDateTime,
    #[serde(default, with = "time::serde::iso8601::option")]
    pub withdrawn: Option<OffsetDateTime>,
    #[serde(default)]
    pub severity: Vec<Severity>,
    #[serde(default)]
    pub affected: Vec<Affected>,
}

/// One severity entry; OSV surfaces multiple scoring vectors per advisory.
/// We keep the score string as-is — the Security module does not normalize
/// across CVSS versions in v1.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Severity {
    /// Severity scheme — `CVSS_V3`, `CVSS_V4`, etc.
    #[serde(rename = "type")]
    pub kind: String,
    /// Raw severity string (e.g. CVSS vector).
    pub score: String,
}

/// One affected-package entry from an advisory.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Affected {
    #[serde(default)]
    pub package: Option<PackageRef>,
    #[serde(default)]
    pub versions: Vec<String>,
}

/// Package identifier as embedded inside an `Affected` block.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageRef {
    pub name: String,
    pub ecosystem: String,
}

/// Internal envelope for OSV's `/v1/query` response shape.
#[derive(Debug, Default, Deserialize)]
struct QueryResponse {
    #[serde(default)]
    vulns: Vec<OsvAdvisory>,
}

/// Cheap-to-clone OSV client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    cache: Cache,
}

impl Client {
    /// Build a new client. OSV is unauthenticated for Phase 1/2 traffic
    /// volumes per `docs/api-notes.md` §OSV.dev.
    #[must_use]
    pub fn new(http: HttpClient, cache: Cache) -> Self {
        Self {
            http,
            base_url: OSV_API_BASE.to_string(),
            cache,
        }
    }

    /// Override the API base URL — wiremock fixtures use this.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// `POST /v1/query` with `{package, version}` body. Returns the
    /// non-withdrawn advisories affecting the given coordinates, sorted by
    /// `id` for deterministic output.
    pub async fn query(&self, coords: &PackageCoords) -> Result<Vec<OsvAdvisory>> {
        let cache_key = format!(
            "osv:{}:{}:{}",
            coords.ecosystem, coords.name, coords.version
        );
        let body = serde_json::json!({
            "package": {
                "name": coords.name,
                "ecosystem": coords.ecosystem,
            },
            "version": coords.version,
        });
        let raw = self
            .fetch_post_json(&cache_key, "/v1/query", &body, TTL_OSV_QUERY)
            .await?;
        let parsed: QueryResponse =
            serde_json::from_slice(&raw).context("parse OSV /v1/query response")?;
        let mut vulns: Vec<OsvAdvisory> = parsed
            .vulns
            .into_iter()
            .filter(|v| v.withdrawn.is_none())
            .collect();
        vulns.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(vulns)
    }

    // ─── Internals ────────────────────────────────────────────────────────

    /// Cache-aware `POST` lifecycle. OSV does not surface ETags on POST
    /// responses, so we cannot round-trip `If-None-Match`. Instead the
    /// cache acts as a deduplication window inside the TTL: a fresh entry
    /// short-circuits the network entirely; a stale entry is replaced
    /// after a fresh `POST`.
    async fn fetch_post_json(
        &self,
        cache_key: &str,
        path: &str,
        body: &serde_json::Value,
        ttl: Duration,
    ) -> Result<Vec<u8>> {
        if let Some(entry) = self.cache.get(cache_key)? {
            if !entry.is_stale() {
                return Ok(entry.body.clone());
            }
        }

        let url = format!("{}{}", self.base_url, path);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("repo-trust"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        match resp.status() {
            StatusCode::OK => {
                let body = resp.bytes().await?;
                self.cache.put(cache_key, None, &body, ttl)?;
                Ok(body.to_vec())
            },
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(OsvError::Other {
                    status: s.as_u16(),
                    body,
                }
                .into())
            },
        }
    }
}
