//! Maintainer Health module.

use async_trait::async_trait;

use super::TrustModule;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

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
        _ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        anyhow::bail!("maintainers module: not yet implemented")
    }
}
