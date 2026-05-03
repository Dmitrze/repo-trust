//! Activity collector — fetches the raw GitHub data the Activity Health
//! module needs.
//!
//! The collector pulls one 18-month window of commits and derives the
//! 30/90/365-day windows from it (one network round-trip instead of three),
//! along with releases, recent issues + PRs, and the repo's `archived` /
//! `has_issues` flags.
//!
//! See [`specs/activity-health-module.md`](../../specs/activity-health-module.md)
//! and [`docs/module-specs.md`](../../docs/module-specs.md) §Activity Health.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::{Client, CommitMeta, IssueMeta, PullMeta, ReleaseMeta, Repository};

/// Raw inputs the activity scorer needs. Serialized into the cache so a
/// re-scan that only re-runs the scorer (e.g. after a threshold change)
/// can skip refetching from GitHub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRawData {
    /// All commits in the most recent 18 months — superset of 30/90/365d.
    pub commits_18m: Vec<CommitMeta>,
    /// Issues touched in the last 90 days.
    pub issues_90d: Vec<IssueMeta>,
    /// Pull requests touched in the last 90 days.
    pub prs_90d: Vec<PullMeta>,
    /// Every published release (drafts filtered out client-side per spec).
    pub releases: Vec<ReleaseMeta>,
    /// `true` if `Repository.archived == true`.
    pub archived: bool,
    /// `true` if the repo has issues enabled (drives the scorer's
    /// "issues_enabled" branch).
    pub issues_enabled: bool,
}

/// Pull all the activity-relevant data from GitHub through `client`.
///
/// `now` is the scan's snapshot timestamp; we anchor the 18-month and 90-day
/// windows to it for determinism.
pub async fn collect(
    client: &Client,
    owner: &str,
    repo: &str,
    now: OffsetDateTime,
) -> Result<(Repository, ActivityRawData)> {
    let metadata = client.get_repo(owner, repo).await?;

    let cutoff_18m = now - time::Duration::days(30 * 18);
    let cutoff_90d = now - time::Duration::days(90);

    let commits_18m = client.list_commits(owner, repo, cutoff_18m, now).await?;
    let releases = client.list_releases(owner, repo).await?;
    let issues_90d = if metadata.has_issues {
        client.list_issues_since(owner, repo, cutoff_90d).await?
    } else {
        Vec::new()
    };
    let prs_90d = client.list_pulls(owner, repo, cutoff_90d).await?;

    let raw = ActivityRawData {
        commits_18m,
        issues_90d,
        prs_90d,
        releases,
        archived: metadata.archived,
        issues_enabled: metadata.has_issues,
    };
    Ok((metadata, raw))
}
