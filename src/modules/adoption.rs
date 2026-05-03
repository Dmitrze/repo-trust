//! Adoption Signals module.
//!
//! Wires `collectors::adoption::collect` → `features::adoption::compute` →
//! `scoring::adoption::score`. See
//! [`specs/adoption-signals-module.md`](../../specs/adoption-signals-module.md).

use async_trait::async_trait;

use super::TrustModule;
use crate::collectors::adoption;
use crate::features::adoption as features;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};
use crate::scoring::adoption as scoring;
use crate::scoring::thresholds::AdoptionThresholds;

#[derive(Debug, Default)]
pub struct AdoptionModule;

#[async_trait]
impl TrustModule for AdoptionModule {
    fn name(&self) -> &'static str {
        "adoption"
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        let (owner, repo) = ctx.owner_repo();
        let (_metadata, raw) =
            adoption::collect(&ctx.github, &ctx.deps_dev, owner, repo, ctx.snapshot_at).await?;
        let features = features::compute(&raw, ctx.snapshot_at);
        let (result, evidence) = scoring::score(&features, &AdoptionThresholds::v1());
        Ok((result, evidence))
    }
}
