//! deps.dev v3 REST client with ETag-aware caching.
//!
//! Federated read-only client for the Adoption Signals module. Two endpoints
//! are in scope per [`specs/deps-dev-client.md`](../../specs/deps-dev-client.md):
//!
//! - `GET /v3alpha/projects/github.com%2F{owner}%2F{repo}:packageversions` —
//!   versions list for every package whose source-of-record points at this
//!   GitHub repo. We dedupe to the underlying `(system, name)` pairs.
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

    /// `GET /v3alpha/projects/github.com%2F{owner}%2F{repo}:packageversions`
    /// — every published version whose source-of-record points at this
    /// repository. Filtered down to first-party publications and
    /// deduped to the unique `(system, name)` set (see
    /// [`first_party_packages_from_versions`] for the rule).
    ///
    /// We use `v3alpha` rather than `v3` because v3 has no `:packages`
    /// or `:packageversions` method on the project resource — it only
    /// exposes the project metadata blob (stars/scorecard/etc.), with
    /// no way to enumerate the published packages. v3alpha is the
    /// upstream-recommended path for this query.
    ///
    /// Returns:
    /// - `Ok(vec)` with packages **sorted by `(system, name)`** for
    ///   deterministic JSON output (per spec §2 and scenario S-102).
    /// - `Ok(Vec::new())` when deps.dev replies 404, OR when every
    ///   returned entry is a transitive mention rather than a
    ///   first-party publication (scenario S-101 covers both —
    ///   "no packages mapped" and "everything mapped is just
    ///   third-party `SOURCE_REPO` noise").
    /// - `Err(_)` on parse failures, transport failures, or non-404 HTTP
    ///   errors (4xx other than 404, or 5xx).
    pub async fn project_packages(&self, owner: &str, repo: &str) -> Result<Vec<PackageRef>> {
        let key = format!("deps_dev:projects:{owner}/{repo}:packageversions");
        // The project resource key is `github.com/{owner}/{repo}` — the
        // slashes inside it must be URL-encoded so the v3alpha router
        // doesn't mistake them for path separators. The `:packageversions`
        // suffix is a v3 method-style suffix and is NOT encoded.
        let project_key = encode_project_key(owner, repo);
        let path = format!("/v3alpha/projects/{project_key}:packageversions");
        let body = match self.fetch_json(&key, &path, TTL_DEPS_DEV).await {
            Ok(b) => b,
            Err(e) => {
                if let Some(DepsDevError::NotFound) = e.downcast_ref::<DepsDevError>() {
                    return Ok(Vec::new());
                }
                return Err(e);
            },
        };
        // `:packageversions` returns one entry per (system, name,
        // version) tuple — for any project with more than one release
        // the same (system, name) pair shows up many times. Parse the
        // rich wire shape so we can apply the first-party filter
        // (relationProvenance + name-match + version-count) before
        // dedup'ing to (system, name). See
        // [`first_party_packages_from_versions`] for the rule.
        let parsed: ProjectVersionsResponse = serde_json::from_slice(&body)
            .context("parse deps.dev project :packageversions response")?;
        Ok(first_party_packages_from_versions(
            &parsed.versions,
            owner,
            repo,
        ))
    }

    /// `GET /v3/systems/{system}/packages/{name}` — per-package metadata.
    ///
    /// `name` is URL-encoded so package identifiers that legitimately
    /// contain slashes (GO modules like `github.com/tokio-rs/tokio`,
    /// Maven coordinates, etc.) round-trip without the path router
    /// splitting them across path segments.
    ///
    /// Returns:
    /// - `Ok(info)` on a 200 response.
    /// - `Err(DepsDevError::NotFound)` on a 404 — the caller is asking for a
    ///   specific package that does not exist; surface it.
    /// - `Err(_)` on parse failures, transport failures, or non-404 HTTP
    ///   errors.
    pub async fn package(&self, system: &str, name: &str) -> Result<PackageInfo> {
        let key = format!("deps_dev:systems:{system}:{name}");
        let encoded_name = encode_path_segment(name);
        let path = format!("/v3/systems/{system}/packages/{encoded_name}");
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

/// Hand-rolled minimal percent-encoder for path segments that
/// legitimately contain `/` or `:` and must round-trip through
/// deps.dev's HTTP router without being split. Used for both the
/// project key (`github.com/{owner}/{repo}`) and per-package names
/// like `github.com/tokio-rs/tokio` (Go modules).
///
/// We do not pull `urlencoding` / `percent-encoding` runtime crates
/// for two replacements. GitHub identifiers and deps.dev package
/// names use `[A-Za-z0-9._-]` plus the `/` and `:` we deliberately
/// escape — anything else is rare enough that we'd rather see a
/// failed lookup than silently mis-route.
fn encode_path_segment(s: &str) -> String {
    s.replace('/', "%2F").replace(':', "%3A")
}

/// Project key for the v3alpha endpoint:
/// `github.com%2F{owner}%2F{repo}`.
fn encode_project_key(owner: &str, repo: &str) -> String {
    format!(
        "github.com%2F{}%2F{}",
        encode_path_segment(owner),
        encode_path_segment(repo)
    )
}

// ─── First-party publication filter ───────────────────────────────────────

/// `relationProvenance` values that indicate a verified first-party
/// publication relationship between the project and the package.
///
/// Empirical, not aspirational: this list contains only values that
/// actually appear in our captured fixtures across CARGO/NPM/GO/PYPI.
/// As of mid-2026, deps.dev's only verified provenance is `GO_ORIGIN`
/// (the canonical Go module path for the repository); CARGO, NPM,
/// PYPI, MAVEN entries all come back as `UNVERIFIED_METADATA`.
///
/// Adding a new entry to this list should be backed by a fixture
/// where it actually appears, never speculative — see the
/// `project_packages_filters_to_first_party_publications` test for
/// the regression contract.
const FIRST_PARTY_RELATIONS: &[&str] = &["GO_ORIGIN"];

/// Minimum number of distinct versions a `(system, name)` group must
/// have to count as a first-party publication.
///
/// Why a threshold at all: any GitHub repo with a single git tag is
/// reachable as a Go module path and therefore appears as a `GO_ORIGIN`
/// entry on `:packageversions`. Real publishers accumulate dozens or
/// hundreds of versions over time; demo / example / fork repos that
/// happen to have one auto-pseudo-version look identical to deps.dev
/// at the resource level. The version-count threshold is the cleanest
/// way to discriminate without per-package metadata calls.
///
/// `2` is intentionally low — even a brand-new project with a v0.1.0
/// and a v0.1.1 clears it. We undercount fresh single-release
/// projects, never overcount transitive mentions.
const MIN_FIRST_PARTY_VERSIONS: usize = 2;

/// Wire shape of one `versions[]` entry returned by
/// `:packageversions`. Wider than [`PackageRef`] because we need
/// `relationProvenance` to apply the first-party filter; the public
/// type stays minimal.
#[derive(Debug, Deserialize)]
struct VersionEntry {
    #[serde(rename = "versionKey")]
    version_key: VersionKeyWire,
    /// Either `relationProvenance` (v3alpha) or `relationType` (legacy
    /// v3). Optional so older fixtures without the field still parse;
    /// missing field falls through the filter as not-first-party.
    #[serde(default, rename = "relationProvenance")]
    relation_provenance: Option<String>,
    #[serde(default, rename = "relationType")]
    relation_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionKeyWire {
    system: String,
    name: String,
    #[serde(default)]
    #[allow(dead_code)] // surfaced in case a future filter rule needs it
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectVersionsResponse {
    #[serde(default)]
    versions: Vec<VersionEntry>,
}

/// Project the `versions[]` wire array onto a sorted, deduplicated
/// `Vec<PackageRef>` of the project's first-party publications.
///
/// Filter rule: a `(system, name)` group is kept iff
///
/// 1. Every entry's relation is in [`FIRST_PARTY_RELATIONS`] (verified
///    first-party provenance), OR `name` matches the GitHub repo
///    identifier (case-insensitive equal, or path-suffix match for
///    Go-module-style names like `github.com/owner/repo`); AND
/// 2. The group contains at least [`MIN_FIRST_PARTY_VERSIONS`]
///    distinct versions.
///
/// Rule (1) eliminates obvious noise (`@scope/x>y>lodash` style
/// transitive mentions). Rule (2) eliminates single-tagged demo
/// repos (`octocat/Hello-World`) that show up as `GO_ORIGIN` simply
/// because every tagged GitHub repo is reachable as a Go module
/// path.
fn first_party_packages_from_versions(
    versions: &[VersionEntry],
    owner: &str,
    repo: &str,
) -> Vec<PackageRef> {
    use std::collections::BTreeMap;

    // Bucket by (system, name) → set of distinct versions seen.
    let mut buckets: BTreeMap<(String, String), Vec<&VersionEntry>> = BTreeMap::new();
    for v in versions {
        buckets
            .entry((v.version_key.system.clone(), v.version_key.name.clone()))
            .or_default()
            .push(v);
    }

    let mut out: Vec<PackageRef> = Vec::new();
    for ((system, name), entries) in buckets {
        if entries.len() < MIN_FIRST_PARTY_VERSIONS {
            continue;
        }
        let any_first_party = entries.iter().any(|v| {
            let rel = v
                .relation_provenance
                .as_deref()
                .or(v.relation_type.as_deref())
                .unwrap_or("");
            FIRST_PARTY_RELATIONS.contains(&rel) || name_matches_repo(&name, owner, repo)
        });
        if any_first_party {
            out.push(PackageRef { system, name });
        }
    }
    // BTreeMap iteration is sorted by key, so `out` is already in
    // canonical (system, name) order.
    out
}

/// True if `pkg_name` plausibly identifies a package published by
/// `owner/repo` on GitHub.
///
/// Match patterns observed empirically across the captured fixtures:
///
/// - Bare repo-name match (case-insensitive): CARGO/`tokio` from
///   `tokio-rs/tokio`, PYPI/`django` from `django/django`,
///   NPM/`lodash` from `lodash/lodash`.
/// - GitHub path-suffix match (case-insensitive): GO/`github.com/tokio-rs/tokio`
///   matches owner/repo `tokio-rs/tokio`. The match requires the
///   **owner** segment too — not just `/repo` — so a transitive NPM
///   scoped package like `@some-other-author/hello-world` does NOT
///   accidentally match `octocat/Hello-World`.
/// - NPM owner-scoped match (case-insensitive): NPM/`@octocat/hello-world`
///   matches `octocat/Hello-World`.
fn name_matches_repo(pkg_name: &str, owner: &str, repo: &str) -> bool {
    let n = pkg_name.to_ascii_lowercase();
    let o = owner.to_ascii_lowercase();
    let r = repo.to_ascii_lowercase();
    if n == r {
        return true;
    }
    let path_suffix = format!("/{o}/{r}");
    let scope_form = format!("@{o}/{r}");
    n.ends_with(&path_suffix) || n == scope_form
}

// ─── DTOs ─────────────────────────────────────────────────────────────────

/// Identity of one published package (`(system, name)` pair). Used as the
/// element type of [`Client::project_packages`].
///
/// `Ord` / `PartialOrd` implementations sort lexicographically on
/// `(system, name)` — relied upon by [`Client::project_packages`] for
/// deterministic output (scenario S-102).
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageRef {
    /// Package system, e.g. `NPM`, `GO`, `PYPI`, `MAVEN`, `CARGO`.
    pub system: String,
    /// Package name within the system, e.g. `lodash`,
    /// `github.com/prometheus/prometheus`.
    pub name: String,
}

/// `PackageRef` deserialises from any of the three shapes deps.dev has
/// shipped over the past two years:
///
/// 1. `{ "system": "...", "name": "..." }` (flat — pre-v3, still in
///    some cached responses and the test fixtures we control).
/// 2. `{ "packageKey": { "system": "...", "name": "..." } }` (the
///    intermediate v3 shape some callers documented).
/// 3. `{ "versionKey": { "system": "...", "name": "...", "version":
///    "..." } }` (the actual current `:packageversions` shape — we
///    discard the per-version `version` because [`Client::project_packages`]
///    deduplicates to (system, name) anyway).
///
/// Same defensive-tolerant pattern as `deserialize_scorecard_date` in
/// `src/api/scorecard.rs` (E1.7).
impl<'de> Deserialize<'de> for PackageRef {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            system: String,
            name: String,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Flat(Inner),
            PackageKey {
                #[serde(rename = "packageKey")]
                package_key: Inner,
            },
            VersionKey {
                #[serde(rename = "versionKey")]
                version_key: Inner,
            },
        }
        let inner = match Wire::deserialize(de)? {
            Wire::Flat(i) => i,
            Wire::PackageKey { package_key: i } => i,
            Wire::VersionKey { version_key: i } => i,
        };
        Ok(Self {
            system: inner.system,
            name: inner.name,
        })
    }
}

/// Per-package metadata returned by [`Client::package`].
///
/// Deserialises from either of two shapes deps.dev v3 has shipped:
///
/// 1. Legacy flat:
///    `{ "system": "...", "name": "...", "weeklyDownloads": "50000000",
///       "latestVersion": "4.17.21" }`
/// 2. Current (2026): nested `packageKey` + `versions[]`:
///    `{ "packageKey": { "system": "...", "name": "..." },
///       "versions": [
///         { "versionKey": { ..., "version": "1.0.0" },
///           "publishedAt": "...", "isDefault": true, ... }, ... ] }`
///
/// `weekly_downloads` is no longer surfaced anywhere on the v3
/// per-package endpoint — every value is `None` against the live API.
/// Kept on the public type so downstream features/scorers don't need
/// to change; restoring populated downloads is a separate piece of
/// scope (deps.dev's `:queryContainer`-style endpoints, ecosystems'
/// own download APIs, or BigQuery exports).
#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub system: String,
    pub name: String,
    pub weekly_downloads: Option<u64>,
    pub latest_version: Option<String>,
}

impl<'de> Deserialize<'de> for PackageInfo {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VersionKey {
            version: String,
        }

        #[derive(Deserialize)]
        struct VersionEntry {
            #[serde(rename = "versionKey")]
            version_key: VersionKey,
            #[serde(default, rename = "isDefault")]
            is_default: bool,
        }

        #[derive(Deserialize)]
        struct PackageKey {
            system: String,
            name: String,
        }

        #[derive(Deserialize)]
        struct NestedShape {
            #[serde(rename = "packageKey")]
            package_key: PackageKey,
            #[serde(default)]
            versions: Vec<VersionEntry>,
        }

        #[derive(Deserialize)]
        struct FlatShape {
            system: String,
            name: String,
            #[serde(
                default,
                rename = "weeklyDownloads",
                deserialize_with = "deserialize_string_to_u64_option"
            )]
            weekly_downloads: Option<u64>,
            #[serde(default, rename = "latestVersion")]
            latest_version: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Nested(NestedShape),
            Flat(FlatShape),
        }

        Ok(match Wire::deserialize(de)? {
            Wire::Nested(n) => {
                // Pick the `isDefault` version's string if present, else
                // the last entry (deps.dev returns versions in publish
                // order — the last one is typically the most recent).
                let latest_version = n
                    .versions
                    .iter()
                    .find(|v| v.is_default)
                    .or(n.versions.last())
                    .map(|v| v.version_key.version.clone());
                PackageInfo {
                    system: n.package_key.system,
                    name: n.package_key.name,
                    weekly_downloads: None,
                    latest_version,
                }
            },
            Wire::Flat(f) => PackageInfo {
                system: f.system,
                name: f.name,
                weekly_downloads: f.weekly_downloads,
                latest_version: f.latest_version,
            },
        })
    }
}

/// Backward-compat envelope used only by the parsing-shape regression
/// tests in `mod tests` below — accepts either `{"packages":[...]}`
/// (legacy flat) or `{"versions":[...]}` (v3alpha) and projects the
/// items through `PackageRef`'s tolerant `Deserialize`. The live
/// client does NOT use this struct (it parses
/// `ProjectVersionsResponse` so it can apply the first-party filter)
/// — kept here so the fixture-shape tests still document the
/// historical wire shapes.
#[cfg(test)]
#[derive(Debug, Default, Deserialize)]
struct ProjectPackagesResponse {
    #[serde(default, alias = "versions")]
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
    type FixtureRow = (
        &'static str,
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
    );
    const FIXTURES: &[FixtureRow] = &[
        // (fixture stem, owner, repo, list of (system, name) the
        // first-party filter must surface)
        ("tokio-rs_tokio", "tokio-rs", "tokio", &[("CARGO", "tokio")]),
        ("django_django", "django", "django", &[("PYPI", "django")]),
        (
            "kubernetes_kubernetes",
            "kubernetes",
            "kubernetes",
            &[("GO", "github.com/kubernetes/kubernetes")],
        ),
    ];

    #[test]
    fn project_packages_response_parses_real_fixtures() {
        for (stem, _owner, _repo, must_contain) in FIXTURES {
            let path = format!(
                "{}/tests/fixtures/deps_dev/{stem}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let body =
                std::fs::read(&path).unwrap_or_else(|e| panic!("missing fixture {path}: {e}"));
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
    fn first_party_filter_yields_expected_packages_on_real_fixtures() {
        // Validates the full filter pipeline against captured deps.dev
        // bodies — same fixture set, but routed through
        // ProjectVersionsResponse + first_party_packages_from_versions
        // (the path Client::project_packages takes).
        for (stem, owner, repo, must_contain) in FIXTURES {
            let path = format!(
                "{}/tests/fixtures/deps_dev/{stem}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let body = std::fs::read(&path).unwrap();
            let parsed: ProjectVersionsResponse = serde_json::from_slice(&body)
                .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
            let pkgs = first_party_packages_from_versions(&parsed.versions, owner, repo);
            assert!(
                !pkgs.is_empty(),
                "first-party filter zeroed out fixture {stem}; got: {pkgs:?}",
            );
            for (sys, name) in *must_contain {
                assert!(
                    pkgs.iter().any(|p| {
                        p.system.eq_ignore_ascii_case(sys) && p.name.eq_ignore_ascii_case(name)
                    }),
                    "first-party filter dropped expected ({sys}, {name}) for {stem}; got: {pkgs:?}",
                );
            }
        }
    }

    #[test]
    fn first_party_filter_zeros_octocat_hello_world() {
        // The whole point of this filter is that octocat/Hello-World
        // — which has 3 GO_ORIGIN entries each with 1 auto-tagged
        // pseudo-version, plus 36 transitive NPM mentions — comes
        // back as 0 first-party packages. Captured fixture is
        // verified to contain exactly that shape.
        let path = format!(
            "{}/tests/fixtures/deps_dev/octocat_Hello-World.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let body = std::fs::read(&path).unwrap();
        let parsed: ProjectVersionsResponse = serde_json::from_slice(&body).unwrap();
        let pkgs = first_party_packages_from_versions(&parsed.versions, "octocat", "Hello-World");
        assert!(
            pkgs.is_empty(),
            "octocat/Hello-World must yield 0 first-party packages (the demo \
             repo has only single-pseudo-version GO entries + transitive NPM \
             mentions); got: {pkgs:?}",
        );
    }

    #[test]
    fn first_party_filter_keeps_verified_provenance_with_enough_versions() {
        // Two versions of CARGO/tokio under verified GO_ORIGIN
        // provenance — should survive.
        let body = br#"{
            "versions": [
                {
                    "versionKey": { "system": "GO", "name": "github.com/o/r", "version": "v1.0.0" },
                    "relationProvenance": "GO_ORIGIN"
                },
                {
                    "versionKey": { "system": "GO", "name": "github.com/o/r", "version": "v1.1.0" },
                    "relationProvenance": "GO_ORIGIN"
                }
            ]
        }"#;
        let parsed: ProjectVersionsResponse = serde_json::from_slice(body).unwrap();
        let pkgs = first_party_packages_from_versions(&parsed.versions, "o", "r");
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].system, "GO");
    }

    #[test]
    fn first_party_filter_keeps_unverified_provenance_when_name_matches() {
        // CARGO/tokio with UNVERIFIED_METADATA but the package name
        // matches the repo identifier exactly. Real-world tokio case.
        let body = br#"{
            "versions": [
                {
                    "versionKey": { "system": "CARGO", "name": "tokio", "version": "1.0.0" },
                    "relationProvenance": "UNVERIFIED_METADATA"
                },
                {
                    "versionKey": { "system": "CARGO", "name": "tokio", "version": "1.1.0" },
                    "relationProvenance": "UNVERIFIED_METADATA"
                }
            ]
        }"#;
        let parsed: ProjectVersionsResponse = serde_json::from_slice(body).unwrap();
        let pkgs = first_party_packages_from_versions(&parsed.versions, "tokio-rs", "tokio");
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "tokio");
    }

    #[test]
    fn first_party_filter_drops_unverified_provenance_when_name_mismatches() {
        // CARGO/broker_tokio for a tokio scan: UNVERIFIED_METADATA
        // and the package name does not match "tokio". Should be
        // filtered out (this is exactly the noise category that
        // motivated the filter).
        let body = br#"{
            "versions": [
                {
                    "versionKey": { "system": "CARGO", "name": "broker_tokio", "version": "0.1.0" },
                    "relationProvenance": "UNVERIFIED_METADATA"
                },
                {
                    "versionKey": { "system": "CARGO", "name": "broker_tokio", "version": "0.2.0" },
                    "relationProvenance": "UNVERIFIED_METADATA"
                }
            ]
        }"#;
        let parsed: ProjectVersionsResponse = serde_json::from_slice(body).unwrap();
        let pkgs = first_party_packages_from_versions(&parsed.versions, "tokio-rs", "tokio");
        assert!(
            pkgs.is_empty(),
            "transitive mention must be dropped; got: {pkgs:?}"
        );
    }

    #[test]
    fn first_party_filter_drops_single_version_packages() {
        // Even with a perfect name match AND verified provenance, a
        // single-version entry is treated as not-yet-published-in-
        // earnest. This is the rule that gets octocat to 0.
        let body = br#"{
            "versions": [
                {
                    "versionKey": { "system": "GO", "name": "github.com/o/Hello-World", "version": "v0.0.1" },
                    "relationProvenance": "GO_ORIGIN"
                }
            ]
        }"#;
        let parsed: ProjectVersionsResponse = serde_json::from_slice(body).unwrap();
        let pkgs = first_party_packages_from_versions(&parsed.versions, "o", "Hello-World");
        assert!(
            pkgs.is_empty(),
            "single-version entry must be dropped; got: {pkgs:?}"
        );
    }

    #[test]
    fn first_party_filter_treats_missing_provenance_as_not_first_party() {
        // Older deps.dev responses or non-package endpoints may not
        // include relationProvenance. Treat missing as not first-party
        // rather than panicking — safer to undercount than to count
        // unknown entries.
        let body = br#"{
            "versions": [
                { "versionKey": { "system": "CARGO", "name": "x", "version": "1.0.0" } },
                { "versionKey": { "system": "CARGO", "name": "x", "version": "1.1.0" } }
            ]
        }"#;
        let parsed: ProjectVersionsResponse = serde_json::from_slice(body).unwrap();
        // repo doesn't match name `x`, so name-match also fails →
        // entry has neither verified provenance nor name-match → dropped.
        let pkgs = first_party_packages_from_versions(&parsed.versions, "owner-y", "y");
        assert!(
            pkgs.is_empty(),
            "missing provenance + no name-match → drop; got: {pkgs:?}"
        );
    }

    #[test]
    fn name_matches_repo_owner_aware() {
        // GO module names have the `github.com/owner/repo` shape; the
        // matcher requires both owner AND repo to match (no plain
        // `/repo` suffix-only match — that would over-match e.g.
        // `@nloyyjuqc/hello-world` against `octocat/Hello-World`).
        assert!(name_matches_repo(
            "github.com/tokio-rs/tokio",
            "tokio-rs",
            "tokio"
        ));
        assert!(name_matches_repo(
            "github.com/Kubernetes/Kubernetes",
            "kubernetes",
            "kubernetes"
        ));
        // NPM scope-form match.
        assert!(name_matches_repo(
            "@octocat/hello-world",
            "octocat",
            "Hello-World"
        ));
        // Plain match still works.
        assert!(name_matches_repo("django", "django", "django"));
        // Negative cases — wrong owner, wrong scope.
        assert!(!name_matches_repo("broker_tokio", "tokio-rs", "tokio"));
        assert!(!name_matches_repo("tokio_macros", "tokio-rs", "tokio"));
        assert!(!name_matches_repo(
            "@nloyyjuqc/hello-world",
            "octocat",
            "Hello-World"
        ));
        assert!(!name_matches_repo(
            "github.com/someone-else/Hello-World",
            "octocat",
            "Hello-World"
        ));
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
        let body =
            br#"{ "packages": [ { "packageKey": { "system": "NPM", "name": "lodash" } } ] }"#;
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
