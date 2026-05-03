//! Adoption Signals module.

use async_trait::async_trait;

use super::TrustModule;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

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
        _ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        anyhow::bail!("adoption module: not yet implemented")
    }
}
