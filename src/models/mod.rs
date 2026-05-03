//! Shared data types used across the crate.
//!
//! Most types are `serde::Serialize + serde::Deserialize` to support the
//! frozen JSON report schema (see `docs/architecture.md` §5).

pub mod evidence;
pub mod reports;
pub mod repository;
pub mod scores;

pub use evidence::{EvidenceItem, Verdict};
pub use reports::{Mode, TrustReport};
pub use repository::{RepositoryContext, RepositorySummary};
pub use scores::{Category, Confidence, ModuleResult, ModuleWeights};
