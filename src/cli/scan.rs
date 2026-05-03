//! `scan` — evaluate a single repository.

use clap::Args;

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Repository identifier: `owner/repo` or full GitHub URL.
    pub repo: String,

    /// Execution mode.
    #[arg(long, value_enum, default_value_t = Mode::Standard)]
    pub mode: Mode,

    /// Comma-separated list of modules to enable (default: all).
    #[arg(long, value_delimiter = ',')]
    pub modules: Option<Vec<String>>,

    /// Comma-separated list of modules to skip.
    #[arg(long, value_delimiter = ',')]
    pub skip_modules: Option<Vec<String>>,

    /// Output directory for written report files.
    #[arg(long, default_value = "./repo-trust-reports")]
    pub output: std::path::PathBuf,

    /// Output formats to write (terminal is always shown unless --quiet).
    #[arg(long, value_delimiter = ',', value_enum)]
    pub format: Vec<Format>,

    /// Path to a TOML file with custom module weights.
    #[arg(long)]
    pub weights: Option<std::path::PathBuf>,

    /// Pin a specific scoring version. If unset, latest is used.
    #[arg(long)]
    pub scoring_version: Option<String>,

    /// GitHub Personal Access Token. Prefer the `GITHUB_TOKEN` env var.
    #[arg(long, env = "GITHUB_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// RNG seed for sampling (deterministic output). Default derived from repo+scoring_version.
    #[arg(long)]
    pub seed: Option<u64>,

    /// Invalidate all cache entries for this repo before scanning.
    #[arg(long)]
    pub refresh: bool,

    /// Invalidate cache for a specific module before scanning.
    #[arg(long)]
    pub refresh_module: Option<String>,

    /// Verbose tracing logs (sets RUST_LOG=debug).
    #[arg(long)]
    pub debug: bool,

    /// Suppress progress output.
    #[arg(long)]
    pub quiet: bool,

    /// Disable terminal colors.
    #[arg(long)]
    pub no_color: bool,

    /// Shorthand for `--format json --quiet`.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Mode {
    /// < 5s, < 30 API calls, headline signals only.
    Quick,
    /// < 30s, < 200 API calls, all modules at default sampling.
    Standard,
    /// < 5min, < 2000 API calls, larger sampling and graph analysis.
    Deep,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Format {
    Terminal,
    Json,
    Md,
    Csv,
    Sarif,
}

pub async fn execute(args: ScanArgs) -> anyhow::Result<u8> {
    tracing::info!(repo = %args.repo, mode = ?args.mode, "scan starting");
    // TODO: wire collectors → features → modules → aggregator → reporters.
    // See docs/architecture.md §1 for the data-flow diagram.
    anyhow::bail!("scan: not yet implemented (Phase 1 in progress)")
}
