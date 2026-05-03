//! Activity Health module.

use async_trait::async_trait;

use super::TrustModule;
use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

#[derive(Debug, Default)]
pub struct ActivityModule;

#[async_trait]
impl TrustModule for ActivityModule {
    fn name(&self) -> &'static str { "activity" }
    fn version(&self) -> &'static str { "1.0.0" }

    async fn run(&self, _ctx: &RepositoryContext) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)> {
        anyhow::bail!("activity module: not yet implemented")
    }
}
