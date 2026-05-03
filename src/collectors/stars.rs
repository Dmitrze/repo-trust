//! Stargazer collector for the Star Authenticity module.
//!
//! Day 3 (shallow): fetches the most-recent N stargazers via the cached
//! GitHub client, then per-stargazer profile via `Client::get_user`. The
//! sample size N comes from the mode (Quick=0, Standard=200, Deep=2000)
//! and is capped at the repo's `stargazers_count`.
//!
//! See [`specs/star-authenticity-module-shallow.md`](../../specs/star-authenticity-module-shallow.md)
//! and `docs/architecture.md` §7 for the sampling algorithm.

use anyhow::Result;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};

use crate::api::github::{Client, Repository, StargazerEntry, UserProfile};

/// Raw inputs the stars scorer needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarsRawData {
    pub repo_metadata: Repository,
    /// `(stargazer_entry, fetched_profile)` pairs in the order they were
    /// returned by the API.
    pub sampled_profiles: Vec<(StargazerEntry, UserProfile)>,
    /// What the caller asked for (so the scorer can distinguish "below
    /// floor → 0 sampled" from "asked for 200, only got 30 from a small
    /// repo").
    pub sample_size_target: usize,
}

/// Pull all star-authenticity-relevant data through `client`.
///
/// `sample_size` is the mode-derived target (0 for Quick — module is
/// effectively skipped; 200 Standard; 2000 Deep). When `sample_size == 0`
/// we still return the repo metadata so the scorer can emit the
/// `below_sampling_floor` evidence.
pub async fn collect(
    client: &Client,
    owner: &str,
    repo: &str,
    sample_size: usize,
) -> Result<StarsRawData> {
    let repo_metadata = client.get_repo(owner, repo).await?;
    if sample_size == 0 {
        return Ok(StarsRawData {
            repo_metadata,
            sampled_profiles: Vec::new(),
            sample_size_target: 0,
        });
    }

    // The list_stargazers method returns the most-recent N entries. For Day 3
    // we treat them as the sample directly (no further sub-sampling); the
    // determinism guarantee holds because the upstream pagination is stable.
    let stargazers = client.list_stargazers(owner, repo, sample_size).await?;

    // Fetch each profile concurrently — the rate limiter inside the client
    // bounds the in-flight count, so we don't need our own semaphore here.
    let profile_futs = stargazers.iter().map(|entry| {
        let login = entry.login().to_string();
        let client = client.clone();
        async move {
            let profile = client.get_user(&login).await?;
            anyhow::Ok((entry.clone(), profile))
        }
    });
    let pairs: Vec<(StargazerEntry, UserProfile)> = try_join_all(profile_futs).await?;

    Ok(StarsRawData {
        repo_metadata,
        sampled_profiles: pairs,
        sample_size_target: sample_size,
    })
}
