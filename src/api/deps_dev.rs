//! deps.dev v3 REST client with ETag-aware caching.
//!
//! Federated read-only client for the Adoption Signals module. Two endpoints
//! are in scope per [`specs/deps-dev-client.md`](../../specs/deps-dev-client.md):
//!
//! - `GET /v3/projects/github.com/{owner}/{repo}/packages` —
//!   list of published packages a GitHub repo maps to.
//! - `GET /v3/systems/{system}/packages/{name}` —
//!   per-package metadata, including `weeklyDownloads`.
//!
//! See [`docs/api-notes.md`](../../docs/api-notes.md#depsdev) for upstream
//! caveats. deps.dev is fully public — no authentication is required or
//! accepted.
//!
//! # Example
//!
//! ```no_run
//! use repo_trust::api::deps_dev::Client;
//! use repo_trust::storage::Cache;
//!
//! # async fn demo() -> anyhow::Result<()> {
//! let http = reqwest::Client::builder().build()?;
//! let cache = Cache::open("/tmp/repo-trust-cache.db")?;
//! let client = Client::new(http, cache);
//! let packages = client.project_packages("prometheus", "prometheus").await?;
//! for pkg in packages {
//!     let info = client.package(&pkg.system, &pkg.name).await?;
//!     println!("{}/{}: {:?}", info.system, info.name, info.weekly_downloads);
//! }
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::storage::Cache;

/// Base URL for the deps.dev REST API. Overridable for tests via
/// [`Client::with_base_url`].
pub const DEPS_DEV_API_BASE: &str = "https://api.deps.dev";

/// Cache TTL for deps.dev responses — 24h per `architecture.md` §6.3.
const TTL_DEPS_DEV: Duration = Duration::from_secs(24 * 3600);

/// Errors surfaced by the deps.dev client. The CLI maps these onto exit codes
/// per architecture §8.
///
/// `404 Not Found` on the project endpoint is *not* surfaced as an error —
/// [`Client::project_packages`] swallows it into `Ok(Vec::new())` because a
/// repo with no published packages mapped is a normal Adoption signal
/// ("no packages found" → Neutral evidence with Medium confidence). The
/// per-package endpoint, however, propagates `NotFound` because asking for a
/// specific package that does not exist is a bug or stale upstream mapping.
#[derive(Debug, Error)]
pub enum DepsDevError {
    /// HTTP 404 from a deps.dev endpoint.
    #[error("deps.dev returned 404 not found")]
    NotFound,
    /// Any non-200, non-304, non-404 response (4xx other than 404, or 5xx).
    #[error("deps.dev returned {status}: {body}")]
    Other { status: u16, body: String },
}

/// Cheap-to-clone deps.dev client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    cache: Cache,
}

impl Client {
    /// Build a new client with the default deps.dev base URL.
    /// deps.dev is fully public — no authentication is required or accepted.
    #[must_use]
    pub fn new(http: HttpClient, cache: Cache) -> Self {
        Self {
            http,
            base_url: DEPS_DEV_API_BASE.to_string(),
            cache,
        }
    }

    /// Override the API base URL — wiremock fixtures use this.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// `GET /v3/projects/github.com/{owner}/{repo}/packages` — published
    /// packages mapped to this repository.
    ///
    /// Returns:
    /// - `Ok(vec)` with packages **sorted by `(system, name)`** for
    ///   deterministic JSON output (per spec §2 and scenario S-102).
    /// - `Ok(Vec::new())` when deps.dev replies 404 — i.e. the repository has
    ///   no packages mapped (scenario S-101).
    /// - `Err(_)` on parse failures, transport failures, or non-404 HTTP
    ///   errors (4xx other than 404, or 5xx).
    pub async fn project_packages(&self, owner: &str, repo: &str) -> Result<Vec<PackageRef>> {
        let key = format!("deps_dev:projects:{owner}/{repo}:packages");
        let path = format!("/v3/projects/github.com/{owner}/{repo}/packages");
        let body = match self.fetch_json(&key, &path, TTL_DEPS_DEV).await {
            Ok(b) => b,
            Err(e) => {
                if let Some(DepsDevError::NotFound) = e.downcast_ref::<DepsDevError>() {
                    return Ok(Vec::new());
                }
                return Err(e);
            },
        };
        let parsed: ProjectPackagesResponse =
            serde_json::from_slice(&body).context("parse deps.dev project packages response")?;
        let mut packages = parsed.packages;
        packages.sort();
        Ok(packages)
    }

    /// `GET /v3/systems/{system}/packages/{name}` — per-package metadata.
    ///
    /// Returns:
    /// - `Ok(info)` on a 200 response.
    /// - `Err(DepsDevError::NotFound)` on a 404 — the caller is asking for a
    ///   specific package that does not exist; surface it.
    /// - `Err(_)` on parse failures, transport failures, or non-404 HTTP
    ///   errors.
    pub async fn package(&self, system: &str, name: &str) -> Result<PackageInfo> {
        let key = format!("deps_dev:systems:{system}:{name}");
        let path = format!("/v3/systems/{system}/packages/{name}");
        let body = self.fetch_json(&key, &path, TTL_DEPS_DEV).await?;
        let parsed: PackageInfo =
            serde_json::from_slice(&body).context("parse deps.dev PackageInfo")?;
        Ok(parsed)
    }

    // ─── Internals ────────────────────────────────────────────────────────

    /// ETag-aware fetch lifecycle. Hits cache first; on miss/stale sends a
    /// conditional `GET` with `If-None-Match`; on 304 reuses the cached body
    /// and refreshes its TTL; on 200 stores the new body+etag.
    ///
    /// Returns `Err(DepsDevError::NotFound)` for 404 — callers can downcast
    /// and translate that into whatever empty/missing semantics they need.
    async fn fetch_json(&self, cache_key: &str, path: &str, ttl: Duration) -> Result<Vec<u8>> {
        let cached = self.cache.get(cache_key)?;
        if let Some(entry) = &cached {
            if !entry.is_stale() {
                return Ok(entry.body.clone());
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
                Ok(body)
            },
            StatusCode::OK => {
                let new_etag = resp
                    .headers()
                    .get("etag")
                    .and_then(|h| h.to_str().ok())
                    .map(str::to_string);
                let body = resp.bytes().await?;
                self.cache.put(cache_key, new_etag.as_deref(), &body, ttl)?;
                Ok(body.to_vec())
            },
            StatusCode::NOT_FOUND => Err(DepsDevError::NotFound.into()),
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(DepsDevError::Other {
                    status: s.as_u16(),
                    body,
                }
                .into())
            },
        }
    }
}

// ─── DTOs ─────────────────────────────────────────────────────────────────

/// Identity of one published package (`(system, name)` pair). Used as the
/// element type of [`Client::project_packages`].
///
/// `Ord` / `PartialOrd` implementations sort lexicographically on
/// `(system, name)` — relied upon by [`Client::project_packages`] for
/// deterministic output (scenario S-102).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageRef {
    /// Package system, e.g. `NPM`, `GO`, `PYPI`, `MAVEN`, `CARGO`.
    pub system: String,
    /// Package name within the system, e.g. `lodash`,
    /// `github.com/prometheus/prometheus`.
    pub name: String,
}

/// Per-package metadata returned by `Client::package`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package system, e.g. `NPM`.
    pub system: String,
    /// Package name within the system, e.g. `lodash`.
    pub name: String,
    /// Weekly downloads where deps.dev provides them. **deps.dev returns this
    /// field as a JSON string** (e.g. `"50000000"`) rather than a number, so
    /// we parse it via the private `deserialize_string_to_u64_option`
    /// helper.
    #[serde(
        default,
        rename = "weeklyDownloads",
        deserialize_with = "deserialize_string_to_u64_option"
    )]
    pub weekly_downloads: Option<u64>,
    /// Latest published version where deps.dev knows it.
    #[serde(default, rename = "latestVersion")]
    pub latest_version: Option<String>,
}

/// Internal envelope for the `/v3/projects/.../packages` response shape.
#[derive(Debug, Default, Deserialize)]
struct ProjectPackagesResponse {
    #[serde(default)]
    packages: Vec<PackageRef>,
}

/// Custom deserializer: deps.dev returns numeric counters as JSON strings
/// (e.g. `"50000000"`) but may also send them as numbers, `null`, or omit
/// them. Accept all four shapes and surface as `Option<u64>` — strings that
/// fail to parse become `None` (we'd rather show "no data" than a panic on
/// upstream changes).
fn deserialize_string_to_u64_option<'de, D>(de: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(s.parse::<u64>().ok()),
        Some(serde_json::Value::Number(n)) => Ok(n.as_u64()),
        Some(_other) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_weekly_downloads_from_string() {
        let json = r#"{"system":"NPM","name":"x","weeklyDownloads":"42"}"#;
        let info: PackageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.weekly_downloads, Some(42));
    }

    #[test]
    fn deserialize_weekly_downloads_from_number() {
        let json = r#"{"system":"NPM","name":"x","weeklyDownloads":42}"#;
        let info: PackageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.weekly_downloads, Some(42));
    }

    #[test]
    fn deserialize_weekly_downloads_missing() {
        let json = r#"{"system":"NPM","name":"x"}"#;
        let info: PackageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.weekly_downloads, None);
    }

    #[test]
    fn deserialize_weekly_downloads_null() {
        let json = r#"{"system":"NPM","name":"x","weeklyDownloads":null}"#;
        let info: PackageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.weekly_downloads, None);
    }

    #[test]
    fn deserialize_weekly_downloads_unparseable_string() {
        let json = r#"{"system":"NPM","name":"x","weeklyDownloads":"not a number"}"#;
        let info: PackageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.weekly_downloads, None);
    }

    /// Real-world fixtures captured from `api.deps.dev` — see commit
    /// message and `tests/fixtures/deps_dev/README.md` for capture
    /// commands. Paths anchored at `CARGO_MANIFEST_DIR` so the tests
    /// work under any cwd (CI runners do not always cd into the
    /// package root before invoking cargo test — see E1.7 fix
    /// commit `8c8ed7c`).
    const FIXTURES: &[(&str, &[(&str, &str)])] = &[
        // (fixture stem, list of (system, name) the fixture must contain)
        (
            "tokio-rs_tokio",
            &[("CARGO", "tokio")],
        ),
        (
            "django_django",
            &[("PYPI", "django")],
        ),
        (
            "kubernetes_kubernetes",
            &[("GO", "github.com/kubernetes/kubernetes")],
        ),
    ];

    #[test]
    fn project_packages_response_parses_real_fixtures() {
        for (stem, must_contain) in FIXTURES {
            let path = format!(
                "{}/tests/fixtures/deps_dev/{stem}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let body = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("missing fixture {path}: {e}"));
            let parsed: ProjectPackagesResponse = serde_json::from_slice(&body)
                .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
            assert!(
                !parsed.packages.is_empty(),
                "fixture {stem} must yield at least one PackageRef; got 0",
            );
            for (sys, name) in *must_contain {
                assert!(
                    parsed.packages.iter().any(|p| {
                        p.system.eq_ignore_ascii_case(sys)
                            && p.name.eq_ignore_ascii_case(name)
                    }),
                    "fixture {stem} must contain ({sys}, {name}); got {} unique pkgs starting with {:?}",
                    parsed.packages.len(),
                    parsed.packages.iter().take(5).collect::<Vec<_>>(),
                );
            }
        }
    }

    #[test]
    fn project_packages_handles_legacy_flat_shape() {
        // Defensive — if deps.dev ever flattens or simplifies the
        // envelope, our parser still works (custom Deserialize on
        // PackageRef accepts flat / packageKey / versionKey shapes).
        let body = br#"{ "packages": [ { "system": "CARGO", "name": "x" } ] }"#;
        let parsed: ProjectPackagesResponse = serde_json::from_slice(body).unwrap();
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].name, "x");
    }

    #[test]
    fn project_packages_handles_packagekey_nested_shape() {
        // Architect's hypothesis from E1.9 — accept it too in case
        // deps.dev ever switches the response shape from `versions`
        // to `packages` while keeping the nested key.
        let body = br#"{ "packages": [ { "packageKey": { "system": "NPM", "name": "lodash" } } ] }"#;
        let parsed: ProjectPackagesResponse = serde_json::from_slice(body).unwrap();
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].system, "NPM");
        assert_eq!(parsed.packages[0].name, "lodash");
    }

    #[test]
    fn package_ref_sorts_by_system_then_name() {
        let mut v = vec![
            PackageRef {
                system: "NPM".into(),
                name: "b".into(),
            },
            PackageRef {
                system: "GO".into(),
                name: "a".into(),
            },
            PackageRef {
                system: "NPM".into(),
                name: "a".into(),
            },
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                PackageRef {
                    system: "GO".into(),
                    name: "a".into(),
                },
                PackageRef {
                    system: "NPM".into(),
                    name: "a".into(),
                },
                PackageRef {
                    system: "NPM".into(),
                    name: "b".into(),
                },
            ]
        );
    }
}
