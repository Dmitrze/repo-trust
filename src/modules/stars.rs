//! Star Authenticity module.

use async_trait::async_trait;

use super::TrustModule;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

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
        _ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        anyhow::bail!("stars module: not yet implemented")
    }
}
