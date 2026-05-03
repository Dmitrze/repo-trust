//! Per-module feature pipelines. A feature pipeline turns raw collector
//! output into the normalised feature struct that the module's scorer expects.

pub mod activity;
pub mod adoption;
pub mod maintainers;
pub mod security;
pub mod stars;
