//! Maintainer Health module.
//!
//! Wires `collectors::maintainers::collect` → `features::maintainers::compute` →
//! `scoring::maintainers::score`. See
//! [`specs/maintainer-health-module.md`](../../specs/maintainer-health-module.md).

use async_trait::async_trait;

use super::TrustModule;
use crate::collectors::maintainers;
use crate::features::maintainers as features;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};
use crate::scoring::maintainers as scoring;
use crate::scoring::thresholds::MaintainerThresholds;

#[derive(Debug, Default)]
pub struct MaintainersModule;

#[async_trait]
impl TrustModule for MaintainersModule {
    fn name(&self) -> &'static str {
        "maintainers"
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        let (owner, repo) = ctx.owner_repo();
        let (metadata, raw) =
            maintainers::collect(&ctx.github, owner, repo, ctx.snapshot_at).await?;
        let features = features::compute(&raw, ctx.snapshot_at);
        let repo_age_days = (ctx.snapshot_at - metadata.created_at).whole_days().max(0) as u64;
        let (result, evidence) =
            scoring::score(&features, &MaintainerThresholds::v1(), repo_age_days);
        Ok((result, evidence))
    }
}
