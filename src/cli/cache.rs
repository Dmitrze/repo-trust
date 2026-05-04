//! `cache` — inspect or clear the local cache.
//!
//! See [`specs/cache-subcommands.md`](../../specs/cache-subcommands.md).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config;
use crate::storage::Cache;

#[derive(Debug, Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCmd,
}

#[derive(Debug, Subcommand)]
pub enum CacheCmd {
    /// Show cache statistics: file path, size, per-table row counts.
    Info,
    /// Remove cached entries. Defaults to api_cache only.
    Clear {
        /// Scope deletion to a single repo (`owner/name`).
        #[arg(long)]
        repo: Option<String>,
        /// Also clear the features and reports tables.
        #[arg(long)]
        all: bool,
    },
    /// Remove `api_cache` rows whose `expires_at` is in the past.
    Prune,
}

pub async fn execute(args: CacheArgs) -> Result<u8> {
    let cfg = config::load::<()>(None).context("loading config")?;
    let cache = Cache::open(cfg.cache.resolved_path()).context("opening cache")?;

    match args.command {
        CacheCmd::Info => print_info(&cache, cfg.cache.max_size_mb)?,
        CacheCmd::Clear { repo, all } => {
            if let Some(r) = repo {
                let n = cache.delete_by_repo(&r)?;
                println!("Cleared {n} entries for {r} from api_cache.");
            } else if all {
                let (api, features, reports) = cache.clear_all()?;
                println!(
                    "Cleared {} entries (api_cache={api}, features={features}, reports={reports}).",
                    api + features + reports
                );
            } else {
                let n = cache.clear_api_cache()?;
                println!("Cleared {n} entries from api_cache.");
            }
        },
        CacheCmd::Prune => {
            let n = cache.prune_expired()?;
            println!("Pruned {n} expired entries from api_cache.");
        },
    }
    Ok(0)
}

fn print_info(cache: &Cache, max_size_mb: u64) -> Result<()> {
    let info = cache.info()?;
    println!(
        "Cache: {} (size: {})",
        info.path.display(),
        human_bytes(info.bytes_on_disk)
    );
    println!("  api_cache rows:  {}", info.api_rows);
    println!("  features rows:   {}", info.feature_rows);
    println!("  reports rows:    {}", info.report_rows);
    println!("  soft cap:        {max_size_mb} MB");
    Ok(())
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::human_bytes;

    #[test]
    fn human_bytes_picks_unit() {
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(2048), "2.00 KB");
        assert_eq!(human_bytes(2 * 1024 * 1024), "2.00 MB");
        assert_eq!(human_bytes(2 * 1024 * 1024 * 1024), "2.00 GB");
    }
}
