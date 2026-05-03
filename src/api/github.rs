//! GitHub REST client with ETag-aware caching and rate-limit coordination.
//!
//! Implements the methods Phase 1 modules need (Activity + Maintainers +
//! Stars + Adoption + Security): repo metadata, commits with windows,
//! releases, issues, pulls, contributors, stargazers. See
//! [`specs/github-api-client.md`](../../specs/github-api-client.md) and
//! [`docs/api-notes.md`](../../docs/api-notes.md).

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

use crate::storage::Cache;
use crate::utils::ratelimit::RateLimiter;

/// Base URL for the GitHub REST API. Overridable for tests via
/// [`Client::with_base_url`].
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// Cache TTL for repo metadata (architecture §6.3).
const TTL_REPO_METADATA: Duration = Duration::from_secs(24 * 3600);
/// Cache TTL for active windows: commits, issues, pulls (architecture §6.3).
const TTL_ACTIVE: Duration = Duration::from_secs(3600);
/// Cache TTL for releases (architecture §6.3).
const TTL_RELEASES: Duration = Duration::from_secs(6 * 3600);
/// Cache TTL for contributors summary (architecture §6.3).
const TTL_CONTRIBUTORS: Duration = Duration::from_secs(24 * 3600);
/// Cache TTL for stargazer pages (architecture §6.3).
const TTL_STARGAZERS: Duration = Duration::from_secs(7 * 24 * 3600);

/// Phase-1 cap on total pages we'll walk for any windowed list. 100 entries
/// per page × 10 pages = 1,000 records, enough for every real-world repo we
/// target in the benchmark set.
const MAX_PAGES: u32 = 10;

/// Errors surfaced by the client. The CLI maps these onto exit codes per
/// architecture §8.
#[derive(Debug, Error)]
pub enum GithubError {
    #[error("repository not found")]
    NotFound,
    #[error("authentication failed")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("github returned {status}: {body}")]
    Other { status: u16, body: String },
}

/// Cheap-to-clone GitHub client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    token: Option<String>,
    cache: Cache,
    limiter: RateLimiter,
}

impl Client {
    /// Build a new client. Token is optional; without one we get the
    /// public 60-req/h budget per `docs/api-notes.md`.
    #[must_use]
    pub fn new(
        http: HttpClient,
        cache: Cache,
        limiter: RateLimiter,
        token: Option<String>,
    ) -> Self {
        Self {
            http,
            base_url: GITHUB_API_BASE.to_string(),
            token,
            cache,
            limiter,
        }
    }

    /// Override the API base URL — wiremock fixtures use this.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// `GET /repos/{owner}/{repo}` — repository metadata.
    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<Repository> {
        let key = format!("github:repos:{owner}/{repo}:metadata");
        let path = format!("/repos/{owner}/{repo}");
        let body = self
            .fetch_json(&key, &path, None, TTL_REPO_METADATA)
            .await?;
        let parsed: Repository = serde_json::from_slice(&body).context("parse Repository")?;
        Ok(parsed)
    }

    /// `GET /repos/{owner}/{repo}/commits` over a `[since, until]` window,
    /// paginated up to [`MAX_PAGES`].
    pub async fn list_commits(
        &self,
        owner: &str,
        repo: &str,
        since: OffsetDateTime,
        until: OffsetDateTime,
    ) -> Result<Vec<CommitMeta>> {
        let since_s = since.format(&Iso8601::DEFAULT)?;
        let until_s = until.format(&Iso8601::DEFAULT)?;
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let path = format!(
                "/repos/{owner}/{repo}/commits?per_page=100&since={since_s}&until={until_s}&page={page}"
            );
            let key = format!(
                "github:repos:{owner}/{repo}:commits:{}:{}:p{page}",
                since.unix_timestamp(),
                until.unix_timestamp()
            );
            let body = self.fetch_json(&key, &path, None, TTL_ACTIVE).await?;
            let mut chunk: Vec<CommitMeta> =
                serde_json::from_slice(&body).context("parse commits page")?;
            let len = chunk.len();
            all.append(&mut chunk);
            if len < 100 || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    /// `GET /repos/{owner}/{repo}/releases` — most recent first.
    pub async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<ReleaseMeta>> {
        let key = format!("github:repos:{owner}/{repo}:releases");
        let path = format!("/repos/{owner}/{repo}/releases?per_page=100");
        let body = self.fetch_json(&key, &path, None, TTL_RELEASES).await?;
        let parsed: Vec<ReleaseMeta> = serde_json::from_slice(&body).context("parse releases")?;
        Ok(parsed)
    }

    /// `GET /repos/{owner}/{repo}/issues` (excludes PRs server-side via
    /// `pulls` filter on consumer side: GitHub's issues endpoint returns
    /// both; we filter `pull_request: None`).
    pub async fn list_issues_since(
        &self,
        owner: &str,
        repo: &str,
        since: OffsetDateTime,
    ) -> Result<Vec<IssueMeta>> {
        let since_s = since.format(&Iso8601::DEFAULT)?;
        let key = format!(
            "github:repos:{owner}/{repo}:issues:{}",
            since.unix_timestamp()
        );
        let path = format!("/repos/{owner}/{repo}/issues?state=all&per_page=100&since={since_s}");
        let body = self.fetch_json(&key, &path, None, TTL_ACTIVE).await?;
        let raw: Vec<IssueMeta> = serde_json::from_slice(&body).context("parse issues")?;
        // Drop pull-requests; GitHub returns them through the issues endpoint too.
        Ok(raw
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .collect())
    }

    /// `GET /repos/{owner}/{repo}/pulls?state=all` — Phase 1 wants recent
    /// PRs to compute review latency.
    pub async fn list_pulls(
        &self,
        owner: &str,
        repo: &str,
        since: OffsetDateTime,
    ) -> Result<Vec<PullMeta>> {
        let key = format!(
            "github:repos:{owner}/{repo}:pulls:{}",
            since.unix_timestamp()
        );
        // The PRs endpoint does not accept `since`; we paginate and stop
        // once we cross the cutoff.
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let key = format!("{key}:p{page}");
            let path = format!(
                "/repos/{owner}/{repo}/pulls?state=all&sort=updated&direction=desc&per_page=100&page={page}"
            );
            let body = self.fetch_json(&key, &path, None, TTL_ACTIVE).await?;
            let mut chunk: Vec<PullMeta> =
                serde_json::from_slice(&body).context("parse pulls page")?;
            let len = chunk.len();
            // Stop early if last item is older than `since`.
            let crossed = chunk.last().map(|p| p.updated_at < since).unwrap_or(false);
            // Filter by since-cutoff before appending.
            chunk.retain(|p| p.updated_at >= since);
            all.append(&mut chunk);
            if crossed || len < 100 || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    /// `GET /repos/{owner}/{repo}/contributors` — anonymous-excluded.
    pub async fn list_contributors(&self, owner: &str, repo: &str) -> Result<Vec<ContributorMeta>> {
        let key = format!("github:repos:{owner}/{repo}:contributors");
        let path = format!("/repos/{owner}/{repo}/contributors?per_page=100&anon=false");
        let body = self.fetch_json(&key, &path, None, TTL_CONTRIBUTORS).await?;
        let parsed: Vec<ContributorMeta> =
            serde_json::from_slice(&body).context("parse contributors")?;
        Ok(parsed)
    }

    /// `GET /repos/{owner}/{repo}/stargazers` with the `vnd.github.star+json`
    /// Accept header so we get `starred_at` (per `docs/api-notes.md`).
    pub async fn list_stargazers(
        &self,
        owner: &str,
        repo: &str,
        max: usize,
    ) -> Result<Vec<StargazerEntry>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        let accept = "application/vnd.github.star+json";
        let max_pages = ((max + 99) / 100).clamp(1, MAX_PAGES as usize) as u32;
        loop {
            let key = format!("github:repos:{owner}/{repo}:stargazers:p{page}");
            let path = format!("/repos/{owner}/{repo}/stargazers?per_page=100&page={page}");
            let body = self
                .fetch_json(&key, &path, Some(accept), TTL_STARGAZERS)
                .await?;
            let mut chunk: Vec<StargazerEntry> =
                serde_json::from_slice(&body).context("parse stargazers page")?;
            let len = chunk.len();
            all.append(&mut chunk);
            if all.len() >= max || len < 100 || page >= max_pages {
                break;
            }
            page += 1;
        }
        all.truncate(max);
        Ok(all)
    }

    /// `GET /repos/{owner}/{repo}/contents/{path}` — for doc-presence checks
    /// in the Security module. Returns `Ok(true)` on 200, `Ok(false)` on 404.
    pub async fn file_exists(&self, owner: &str, repo: &str, path: &str) -> Result<bool> {
        let cache_key = format!("github:repos:{owner}/{repo}:contents:{path}");
        let api_path = format!("/repos/{owner}/{repo}/contents/{path}");
        match self
            .fetch_json(&cache_key, &api_path, None, TTL_REPO_METADATA)
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => match e.downcast_ref::<GithubError>() {
                Some(GithubError::NotFound) => Ok(false),
                _ => Err(e),
            },
        }
    }

    // ─── Internals ────────────────────────────────────────────────────────

    /// ETag-aware fetch lifecycle. Hits cache first; on miss/stale sends a
    /// conditional `GET` with `If-None-Match`; on 304 reuses the cached
    /// body and refreshes its TTL; on 200 stores the new body+etag.
    async fn fetch_json(
        &self,
        cache_key: &str,
        path: &str,
        accept: Option<&str>,
        ttl: Duration,
    ) -> Result<Vec<u8>> {
        let cached = self.cache.get(cache_key)?;
        if let Some(entry) = &cached {
            if !entry.is_stale() {
                return Ok(entry.body.clone());
            }
        }
        let cached_etag = cached.as_ref().and_then(|e| e.etag.clone());
        let cached_body = cached.as_ref().map(|e| e.body.clone());

        let _permit = self.limiter.acquire().await?;

        let url = format!("{}{}", self.base_url, path);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("repo-trust"));
        let accept_val = accept.unwrap_or("application/vnd.github+json");
        headers.insert(ACCEPT, HeaderValue::from_str(accept_val)?);
        if let Some(t) = &self.token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {t}"))?,
            );
        }
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
        self.limiter.record(&resp).await;

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
            StatusCode::NOT_FOUND => Err(GithubError::NotFound.into()),
            StatusCode::UNAUTHORIZED => Err(GithubError::Unauthorized.into()),
            StatusCode::FORBIDDEN => {
                let body = resp.text().await.unwrap_or_default();
                Err(GithubError::Forbidden(body).into())
            },
            s => {
                let body = resp.text().await.unwrap_or_default();
                Err(GithubError::Other {
                    status: s.as_u16(),
                    body,
                }
                .into())
            },
        }
    }
}

// ─── DTOs (only the fields the modules use) ───────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Repository {
    pub full_name: String,
    pub html_url: String,
    pub default_branch: String,
    #[serde(default)]
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub watchers_count: u64,
    pub open_issues_count: u64,
    pub archived: bool,
    #[serde(default)]
    pub has_issues: bool,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub pushed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitMeta {
    pub sha: String,
    pub commit: CommitDetails,
    #[serde(default)]
    pub author: Option<UserStub>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitDetails {
    pub author: AuthorTimestamp,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorTimestamp {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserStub {
    pub login: String,
    #[serde(default, rename = "type")]
    pub user_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseMeta {
    pub tag_name: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option", default)]
    pub published_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueMeta {
    pub number: u64,
    pub state: String,
    pub title: String,
    pub user: Option<UserStub>,
    pub comments: u64,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option", default)]
    pub closed_at: Option<OffsetDateTime>,
    /// Present iff the "issue" is actually a pull-request.
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullMeta {
    pub number: u64,
    pub state: String,
    pub title: String,
    pub user: Option<UserStub>,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option", default)]
    pub closed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::iso8601::option", default)]
    pub merged_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContributorMeta {
    pub login: String,
    pub contributions: u64,
    #[serde(default, rename = "type")]
    pub user_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StargazerEntry {
    /// Returned with the `vnd.github.star+json` Accept header.
    WithDate {
        #[serde(with = "time::serde::iso8601")]
        starred_at: OffsetDateTime,
        user: UserStub,
    },
    /// Plain stargazer record (default Accept header).
    Plain(UserStub),
}

impl StargazerEntry {
    #[must_use]
    pub fn login(&self) -> &str {
        match self {
            Self::WithDate { user, .. } | Self::Plain(user) => &user.login,
        }
    }
}
