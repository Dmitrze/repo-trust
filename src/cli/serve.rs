//! `serve` — local web viewer.

use clap::Args;

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Port to bind.
    #[arg(long, default_value_t = 8765)]
    pub port: u16,

    /// Bind address. The default `127.0.0.1` is intentional; do NOT change
    /// without understanding the security implications.
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,
}

#[cfg(feature = "web")]
pub async fn execute(args: ServeArgs) -> anyhow::Result<u8> {
    tracing::info!(bind = %args.bind, port = args.port, "serve starting");
    anyhow::bail!("serve: not yet implemented")
}
