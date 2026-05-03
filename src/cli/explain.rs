//! `explain` — print full evidence walkthrough.

use clap::Args;

#[derive(Debug, Args)]
pub struct ExplainArgs {
    pub repo: String,
}

pub async fn execute(args: ExplainArgs) -> anyhow::Result<u8> {
    tracing::info!(repo = %args.repo, "explain starting");
    anyhow::bail!("explain: not yet implemented")
}
