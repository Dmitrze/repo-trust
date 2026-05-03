//! `cache` — inspect or clear the local cache.

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCmd,
}

#[derive(Debug, Subcommand)]
pub enum CacheCmd {
    /// Show cache statistics.
    Info,
    /// Remove all cached entries.
    Clear,
    /// Remove expired and least-recently-used entries.
    Prune,
}

pub async fn execute(args: CacheArgs) -> anyhow::Result<u8> {
    tracing::info!(?args.command, "cache subcommand");
    anyhow::bail!("cache: not yet implemented")
}
