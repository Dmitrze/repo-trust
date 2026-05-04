//! Star Authenticity module (Day 3 — shallow).
//!
//! Wires `collectors::stars::collect` → `features::stars::compute` →
//! `scoring::stars::score`. See [`specs/star-authenticity-module-shallow.md`](../../specs/star-authenticity-module-shallow.md).

use async_trait::async_trait;

use super::TrustModule;
use crate::cli::scan::Mode as CliMode;
use crate::collectors::stars;
use crate::features::stars as features;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};
use crate::scoring::stars as scoring;
use crate::scoring::thresholds::StarsThresholds;

#[derive(Debug, Default)]
pub struct StarsModule;

#[async_trait]
impl TrustModule for StarsModule {
    fn name(&self) -> &'static str {
        "stars"
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        let (owner, repo) = ctx.owner_repo();
        // Mode-derived sample sizes per `default.toml` and `methodology.md` §6.
        let sample_size = match ctx.mode {
            CliMode::Quick => 0,
            CliMode::Standard => 200,
            CliMode::Deep => 2000,
        };
        let raw = stars::collect(&ctx.github, owner, repo, sample_size).await?;
        let features = features::compute(&raw, ctx.snapshot_at);
        let (result, evidence) = scoring::score(&features, &StarsThresholds::v1());
        Ok((result, evidence))
    }
}
