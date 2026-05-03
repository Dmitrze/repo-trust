//! The five trust modules.
//!
//! Every module implements the `TrustModule` trait (defined in this file).
//! See ADR-0006 for the rationale on having exactly five modules.
//!
//! Plugin registration is reserved for v1.2 (ADR-0010).

use async_trait::async_trait;

use crate::models::{EvidenceItem, ModuleResult, RepositoryContext};

pub mod activity;
pub mod adoption;
pub mod maintainers;
pub mod security;
pub mod stars;

/// Generic trust module — one of the five for v1.
#[async_trait]
pub trait TrustModule: Send + Sync {
    /// Stable identifier (e.g. `"stars"`).
    fn name(&self) -> &'static str;

    /// SemVer of this module's scoring logic. Bumped on threshold or weight changes.
    fn version(&self) -> &'static str;

    /// Run collect → features → score, returning the module's result
    /// and its evidence items.
    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)>;
}

/// Build the registry of all built-in modules.
#[must_use]
pub fn registry() -> Vec<Box<dyn TrustModule>> {
    vec![
        Box::new(stars::StarsModule),
        Box::new(activity::ActivityModule),
        Box::new(maintainers::MaintainersModule),
        Box::new(adoption::AdoptionModule),
        Box::new(security::SecurityModule),
    ]
}
