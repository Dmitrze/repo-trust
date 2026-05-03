//! Per-module data collection. Each collector is responsible for fetching
//! the raw inputs its module needs and storing them via the cache facade.

pub mod activity;
pub mod adoption;
pub mod maintainers;
pub mod repo;
pub mod security;
pub mod stars;
