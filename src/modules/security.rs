//! Security & Readiness module.
//!
//! Wires `collectors::security::collect` → `features::security::compute` →
//! `scoring::security::score`.

use async_trait::async_trait;

use super::TrustModule;
use crate::collectors::security;
use crate::features::security as features;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};
use crate::scoring::security as scoring;

#[derive(Debug, Default)]
pub struct SecurityModule;

#[async_trait]
impl TrustModule for SecurityModule {
    fn name(&self) -> &'static str {
        "security"
    }
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        let (owner, repo) = ctx.owner_repo();
        let (_metadata, raw) = security::collect(
            &ctx.github,
            &ctx.scorecard,
            &ctx.osv,
            owner,
            repo,
            ctx.snapshot_at,
        )
        .await?;
        let features = features::compute(&raw, ctx.snapshot_at);
        let (result, evidence) = scoring::score(&features);
        Ok((result, evidence))
    }
}
