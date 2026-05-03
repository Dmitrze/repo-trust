//! `config` — show or modify configuration.

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCmd,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Print the merged effective configuration.
    Show,
    /// Set a configuration key in `~/.repo-trust/config.toml`.
    Set { key: String, value: String },
    /// Print the path of the user config file.
    Path,
}

pub async fn execute(args: ConfigArgs) -> anyhow::Result<u8> {
    tracing::info!(?args.command, "config subcommand");
    anyhow::bail!("config: not yet implemented")
}
