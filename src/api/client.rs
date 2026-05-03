//! Shared `reqwest::Client` builder with ETag, retry, and rate-limit-aware behaviour.

use std::time::Duration;

use reqwest::Client;

/// Build the project-wide HTTP client.
pub fn build() -> reqwest::Result<Client> {
    Client::builder()
        .user_agent(format!(
            "repo-trust/{} (+https://github.com/Dmitrze/repo-trust)",
            crate::VERSION
        ))
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(8)
        .gzip(true)
        .http2_prior_knowledge()
        .build()
}
