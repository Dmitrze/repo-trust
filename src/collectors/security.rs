//! Security & Readiness collector.
//!
//! Federates `api::scorecard::Client::get` for the OpenSSF Scorecard report,
//! probes the conventional doc-presence paths, lists `.github/workflows/`
//! for CI presence, and pulls release tags for semver consistency. OSV
//! advisories are intentionally **not** queried in Day 2 — the per-package
//! list arrives in Day 3 with the Adoption module's deps.dev mapping.
//!
//! See [`specs/security-readiness-module.md`](../../specs/security-readiness-module.md).

use anyhow::Result;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::github::{Client as GithubClient, ReleaseMeta, Repository};
use crate::api::osv::{Client as OsvClient, OsvAdvisory};
use crate::api::scorecard::{Client as ScorecardClient, ScorecardReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRawData {
    /// `None` when Scorecard returned 404 ("not yet scored").
    pub scorecard: Option<ScorecardReport>,
    /// Open OSV advisories. Always empty in Day 2 — Day 3 wires per-package
    /// queries via the Adoption module's deps.dev mapping.
    pub osv_advisories: Vec<OsvAdvisory>,
    pub has_security_md: bool,
    pub has_contributing_md: bool,
    pub has_code_of_conduct: bool,
    pub has_license: bool,
    pub has_codeowners: bool,
    pub has_ci_workflow: bool,
    pub releases: Vec<ReleaseMeta>,
    pub archived: bool,
}

const LICENSE_PATHS: &[&str] = &["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"];
const CODEOWNERS_PATHS: &[&str] = &[
    "CODEOWNERS",
    ".github/CODEOWNERS",
    "docs/CODEOWNERS",
    ".gitlab/CODEOWNERS",
];
const CI_PROBES: &[&str] = &[
    ".github/workflows",
    ".github/workflows/ci.yml",
    ".github/workflows/test.yml",
    ".github/workflows/main.yml",
    ".github/workflows/build.yml",
    ".circleci/config.yml",
];

/// Pull all security-relevant data through the federated clients.
pub async fn collect(
    github: &GithubClient,
    scorecard: &ScorecardClient,
    _osv: &OsvClient,
    owner: &str,
    repo: &str,
    _now: OffsetDateTime,
) -> Result<(Repository, SecurityRawData)> {
    let metadata = github.get_repo(owner, repo).await?;
    let releases = github.list_releases(owner, repo).await?;
    let scorecard_report = scorecard.get(owner, repo).await?;

    let security_md = github.file_exists(owner, repo, "SECURITY.md");
    let contributing_md = github.file_exists(owner, repo, "CONTRIBUTING.md");
    let code_of_conduct = github.file_exists(owner, repo, "CODE_OF_CONDUCT.md");
    let license_futs = LICENSE_PATHS
        .iter()
        .map(|p| github.file_exists(owner, repo, p));
    let codeowners_futs = CODEOWNERS_PATHS
        .iter()
        .map(|p| github.file_exists(owner, repo, p));
    let ci_futs = CI_PROBES.iter().map(|p| github.file_exists(owner, repo, p));

    let (
        security_md,
        contributing_md,
        code_of_conduct,
        license_results,
        codeowners_results,
        ci_results,
    ) = tokio::join!(
        security_md,
        contributing_md,
        code_of_conduct,
        try_join_all(license_futs),
        try_join_all(codeowners_futs),
        try_join_all(ci_futs),
    );
    let security_md = security_md?;
    let contributing_md = contributing_md?;
    let code_of_conduct = code_of_conduct?;
    let has_license = license_results?.into_iter().any(|x| x);
    let has_codeowners = codeowners_results?.into_iter().any(|x| x);
    let has_ci_workflow = ci_results?.into_iter().any(|x| x);

    let raw = SecurityRawData {
        scorecard: scorecard_report,
        osv_advisories: Vec::new(),
        has_security_md: security_md,
        has_contributing_md: contributing_md,
        has_code_of_conduct: code_of_conduct,
        has_license,
        has_codeowners,
        has_ci_workflow,
        releases,
        archived: metadata.archived,
    };
    Ok((metadata, raw))
}
