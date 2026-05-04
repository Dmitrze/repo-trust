//! Pure scoring functions. **No I/O.** Takes feature structs, returns
//! score structs.
//!
//! Determinism (ADR-0007) is enforced here:
//! - Floats are rounded to 6 decimals before serialization (`utils::round6`).
//! - All sorting uses explicit keys, never `HashMap` order.
//! - Sums are performed in deterministic order.

pub mod activity;
pub mod adoption;
pub mod aggregate;
pub mod confidence;
pub mod explain;
pub mod maintainers;
pub mod security;
pub mod stars;
pub mod thresholds;
pub mod weights;

pub use aggregate::aggregate;
pub use confidence::overall_confidence;
