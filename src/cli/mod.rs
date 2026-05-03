//! Command-line interface.
//!
//! The CLI is built with `clap` v4 in `derive` mode — every subcommand is a
//! struct with `#[derive(Parser)]`. Subcommand handlers live in their own modules.

use clap::{Parser, Subcommand};

pub mod batch;
pub mod cache;
pub mod completions;
pub mod config;
pub mod explain;
pub mod scan;
pub mod serve;

/// Run the CLI. Returns the desired process exit code.
///
/// See `docs/architecture.md` §8 for the exit-code policy.
pub fn run() -> anyhow::Result<u8> {
    let cli = Cli::parse();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        match cli.command {
            Command::Scan(args) => match scan::execute(args).await {
                Ok(code) => Ok(code),
                Err(e) => {
                    let code = scan::exit_code_for(&e);
                    tracing::error!(error = ?e, exit_code = code, "scan failed");
                    eprintln!("error: {e:#}");
                    Ok(code)
                },
            },
            Command::Batch(args) => batch::execute(args).await,
            Command::Explain(args) => explain::execute(args).await,
            #[cfg(feature = "web")]
            Command::Serve(args) => serve::execute(args).await,
            Command::Cache(args) => cache::execute(args).await,
            Command::Config(args) => config::execute(args).await,
            Command::Completions(args) => completions::execute(args),
            Command::Version => {
                println!(
                    "repo-trust {} (scoring-model {})",
                    crate::VERSION,
                    crate::SCORING_VERSION
                );
                Ok(0)
            },
        }
    })
}

/// Repo Trust — a multi-dimensional trust score for any public GitHub repository.
#[derive(Debug, Parser)]
#[command(
    name = "repo-trust",
    version = crate::VERSION,
    about,
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan a single repository and produce a Trust Report.
    Scan(scan::ScanArgs),

    /// Scan many repositories from a newline-separated file.
    Batch(batch::BatchArgs),

    /// Print the full evidence walkthrough for a previously scanned repo.
    Explain(explain::ExplainArgs),

    /// Start the local web viewer at http://localhost:8765.
    #[cfg(feature = "web")]
    Serve(serve::ServeArgs),

    /// Cache management subcommands.
    Cache(cache::CacheArgs),

    /// Configuration subcommands.
    Config(config::ConfigArgs),

    /// Generate shell completion scripts.
    Completions(completions::CompletionsArgs),

    /// Print version and scoring-model information.
    Version,
}
