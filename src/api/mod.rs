//! HTTP clients for upstream services.
//!
//! Each module here is a thin async client around `reqwest` (with an
//! ETag-aware cache layer). See ADR-0005 for the federation policy.

pub mod client;
pub mod deps_dev;
pub mod github;
pub mod osv;
pub mod scorecard;

pub use osv::Client as OsvClient;
pub use scorecard::Client as ScorecardClient;
