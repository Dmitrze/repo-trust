//! `batch` — evaluate many repositories from a file.

use clap::Args;

#[derive(Debug, Args)]
pub struct BatchArgs {
    /// Path to a newline-separated file of `owner/repo` identifiers.
    pub file: std::path::PathBuf,

    /// Maximum concurrent scans.
    #[arg(long, default_value_t = 4)]
    pub concurrency: usize,

    /// Output directory for per-repo reports plus a combined CSV.
    #[arg(long, default_value = "./repo-trust-reports")]
    pub output: std::path::PathBuf,

    /// Output combined report format(s).
    #[arg(long, value_delimiter = ',', value_enum, default_value = "csv")]
    pub format: Vec<super::scan::Format>,

    /// GitHub PAT.
    #[arg(long, env = "GITHUB_TOKEN", hide_env_values = true)]
    pub token: Option<String>,
}

pub async fn execute(args: BatchArgs) -> anyhow::Result<u8> {
    tracing::info!(file = ?args.file, concurrency = args.concurrency, "batch starting");
    anyhow::bail!("batch: not yet implemented")
}
