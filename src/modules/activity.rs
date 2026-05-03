//! Activity Health module.
//!
//! Wires `collectors::activity::collect` → `features::activity::compute` →
//! `scoring::activity::score`. See [`specs/activity-health-module.md`](../../specs/activity-health-module.md).

use async_trait::async_trait;

use super::TrustModule;
use crate::collectors::activity;
use crate::features::activity as features;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};
use crate::scoring::activity as scoring;
use crate::scoring::thresholds::ActivityThresholds;

#[derive(Debug, Default)]
pub struct ActivityModule;

#[async_trait]
impl TrustModule for ActivityModule {
    fn name(&self) -> &'static str {
        "activity"
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        let (owner, repo) = ctx.owner_repo();
        let (metadata, raw) = activity::collect(&ctx.github, owner, repo, ctx.snapshot_at).await?;
        let features = features::compute(&raw, ctx.snapshot_at);
        let repo_age_days = (ctx.snapshot_at - metadata.created_at).whole_days().max(0) as u64;
        let (result, evidence) =
            scoring::score(&features, &ActivityThresholds::v1(), repo_age_days);
        Ok((result, evidence))
    }
}
