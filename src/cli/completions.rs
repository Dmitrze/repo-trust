//! `completions` — generate shell completion scripts.

use clap::{Args, CommandFactory};
use clap_complete::Shell;

use super::Cli;

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn execute(args: CompletionsArgs) -> anyhow::Result<u8> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    clap_complete::generate(args.shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(0)
}
