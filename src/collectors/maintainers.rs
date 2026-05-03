//! Maintainer Health collector — fetches the raw GitHub data the
//! Maintainer Health module needs.
//!
//! Pulls one 18-month commit window (sliced to 365d in the features layer
//! — the Activity collector already cached the same window), the
//! contributors summary, and probes for the three governance documents
//! (`CODEOWNERS`, `MAINTAINERS.md`, `GOVERNANCE.md`).
//!
//! See [`specs/maintainer-health-module.md`](../../specs/maintainer-health-module.md)
//! and `docs/module-specs.md` §Maintainer Health.

use anyhow::Result;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::{Client, CommitMeta, ContributorMeta, Repository};

/// Raw inputs the maintainer scorer needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainersRawData {
    /// 18-month commit window — cached and shared with the Activity module.
    pub commits_18m: Vec<CommitMeta>,
    /// Aggregated contributors (anon-excluded).
    pub contributors: Vec<ContributorMeta>,
    /// Whether `CODEOWNERS` was found at any of the 4 conventional paths.
    pub has_codeowners: bool,
    /// Whether `MAINTAINERS.md` exists at the repo root.
    pub has_maintainers_md: bool,
    /// Whether any `GOVERNANCE*` doc exists at the repo root.
    pub has_governance_doc: bool,
    /// `Repository.archived` — surfaced as caveat by the scorer.
    pub archived: bool,
}

/// CODEOWNERS conventional locations per
/// <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-file-location>.
const CODEOWNERS_PATHS: &[&str] = &[
    "CODEOWNERS",
    ".github/CODEOWNERS",
    "docs/CODEOWNERS",
    ".gitlab/CODEOWNERS",
];

const GOVERNANCE_PATHS: &[&str] = &["GOVERNANCE.md", "GOVERNANCE", "docs/GOVERNANCE.md"];

/// Pull all maintainer-relevant data through `client`.
pub async fn collect(
    client: &Client,
    owner: &str,
    repo: &str,
    now: OffsetDateTime,
) -> Result<(Repository, MaintainersRawData)> {
    let metadata = client.get_repo(owner, repo).await?;

    let cutoff_18m = now - time::Duration::days(30 * 18);
    let commits_18m = client.list_commits(owner, repo, cutoff_18m, now).await?;
    let contributors = client.list_contributors(owner, repo).await?;

    // Run doc-presence checks concurrently — they are independent reads.
    let codeowners_futures = CODEOWNERS_PATHS
        .iter()
        .map(|p| client.file_exists(owner, repo, p));
    let codeowners_results: Vec<bool> = try_join_all(codeowners_futures).await?;
    let has_codeowners = codeowners_results.into_iter().any(|x| x);

    let has_maintainers_md = client.file_exists(owner, repo, "MAINTAINERS.md").await?;

    let governance_futures = GOVERNANCE_PATHS
        .iter()
        .map(|p| client.file_exists(owner, repo, p));
    let governance_results: Vec<bool> = try_join_all(governance_futures).await?;
    let has_governance_doc = governance_results.into_iter().any(|x| x);

    let raw = MaintainersRawData {
        commits_18m,
        contributors,
        has_codeowners,
        has_maintainers_md,
        has_governance_doc,
        archived: metadata.archived,
    };
    Ok((metadata, raw))
}
