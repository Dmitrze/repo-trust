//! Security & Readiness module — federates OpenSSF Scorecard + OSV.

use async_trait::async_trait;

use super::TrustModule;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

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
        _ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        anyhow::bail!("security module: not yet implemented")
    }
}
