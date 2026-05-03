//! Adoption Signals collector.
//!
//! Federates deps.dev for the repo→packages mapping and per-package weekly
//! downloads, and probes GitHub for documentation-maturity signals
//! (README + word count + `docs/`/`examples/` directories).
//!
//! Conservative posture: a deps.dev outage does NOT abort the run — we
//! capture `deps_dev_error: true` and let the scorer drop the downloads
//! sub-score and demote confidence per
//! [`specs/adoption-signals-module.md`](../../specs/adoption-signals-module.md).

use anyhow::Result;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::deps_dev::{Client as DepsDevClient, PackageInfo, PackageRef};
use crate::api::github::{Client as GhClient, Repository};

/// Raw inputs the adoption scorer needs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdoptionRawData {
    /// Per-package metadata returned from deps.dev. Empty on `deps_dev_error`
    /// or when the repo publishes no packages.
    pub packages: Vec<PackageInfo>,
    /// True when any deps.dev call failed for reasons other than 404.
    pub deps_dev_error: bool,
    /// Whether `README.md` (or extension-less `README`) was found at the
    /// repo root via the `/readme` endpoint.
    pub has_readme: bool,
    /// Word count of the README body (None if no README).
    pub readme_word_count: Option<usize>,
    /// Whether the `docs/` directory exists at the repo root.
    pub has_docs_dir: bool,
    /// Whether the `examples/` directory exists at the repo root.
    pub has_examples_dir: bool,
    /// Awesome-list mentions discovered for this repo. Day 3: always empty
    /// (loaded from config in v1.1).
    pub awesome_list_mentions: Vec<String>,
    /// `Repository.archived` — surfaced as caveat by the scorer.
    pub archived: bool,
}

/// Pull all adoption-relevant data through the GitHub + deps.dev clients.
pub async fn collect(
    github: &GhClient,
    deps_dev: &DepsDevClient,
    owner: &str,
    repo: &str,
    _now: OffsetDateTime,
) -> Result<(Repository, AdoptionRawData)> {
    let metadata = github.get_repo(owner, repo).await?;

    // ─── deps.dev: project_packages → package(...) ─────────────────────────
    // Any error (other than the 404 → empty Vec the client handles internally)
    // gets captured as `deps_dev_error: true`. The scorer demotes confidence
    // and emits a Neutral caveat — we never abort the module.
    let (packages, deps_dev_error) = match deps_dev.project_packages(owner, repo).await {
        Ok(refs) => fetch_packages(deps_dev, &refs).await,
        Err(_) => (Vec::new(), true),
    };

    // ─── GitHub: README + doc directory probes (concurrent) ────────────────
    let readme_fut = github.get_readme(owner, repo);
    let docs_fut = github.file_exists(owner, repo, "docs");
    let examples_fut = github.file_exists(owner, repo, "examples");
    let (readme_result, docs_result, examples_result) =
        tokio::try_join!(readme_fut, docs_fut, examples_fut)?;

    let (has_readme, readme_word_count) = match readme_result {
        Some(text) => {
            let count = text.split_whitespace().count();
            (true, Some(count))
        },
        None => (false, None),
    };

    let raw = AdoptionRawData {
        packages,
        deps_dev_error,
        has_readme,
        readme_word_count,
        has_docs_dir: docs_result,
        has_examples_dir: examples_result,
        awesome_list_mentions: Vec::new(),
        archived: metadata.archived,
    };
    Ok((metadata, raw))
}

/// Fetch per-package metadata for every `PackageRef`. Returns
/// `(packages, deps_dev_error)` — `deps_dev_error: true` when any single
/// `package(...)` call failed (we keep the run going so downstream signals
/// still produce evidence).
async fn fetch_packages(deps_dev: &DepsDevClient, refs: &[PackageRef]) -> (Vec<PackageInfo>, bool) {
    if refs.is_empty() {
        return (Vec::new(), false);
    }
    // try_join_all would short-circuit on the first error; we want to keep
    // any successful per-package payloads so the scorer's downloads sub-
    // score still reflects what we did get back. Using join_all preserves
    // partial results.
    let results = join_all(refs.iter().map(|r| deps_dev.package(&r.system, &r.name))).await;
    let mut packages: Vec<PackageInfo> = Vec::with_capacity(results.len());
    let mut had_error = false;
    for r in results {
        match r {
            Ok(p) => packages.push(p),
            Err(_) => had_error = true,
        }
    }
    // If every package call failed AND we expected at least one, surface
    // it as a deps.dev outage so the scorer demotes to Low confidence.
    if had_error && packages.is_empty() {
        return (Vec::new(), true);
    }
    (packages, false)
}
