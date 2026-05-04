//! `serve` — local web viewer.
//!
//! See `specs/web-viewer.md` for the spec and `docs/architecture.md` §12
//! for the architectural sketch.

use clap::Args;

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Bind address (`host:port`). Default `127.0.0.1:8765`.
    ///
    /// Binding to anything other than `127.0.0.1` (e.g. `0.0.0.0:8765`)
    /// exposes the viewer on the network and is logged at WARN level.
    /// See `docs/architecture.md` §12.
    #[arg(long, default_value = "127.0.0.1:8765")]
    pub bind: String,

    /// Allow `POST /scans` to trigger fresh scans from the web UI.
    ///
    /// Disabled by default: a malicious page on another origin can
    /// discover your localhost server via DNS rebinding and POST to it,
    /// burning your GitHub rate limit. Only enable when you control the
    /// browser session.
    #[arg(long)]
    pub allow_scan: bool,
}

#[cfg(feature = "web")]
pub async fn execute(args: ServeArgs) -> anyhow::Result<u8> {
    use anyhow::Context;

    let cfg = crate::config::load::<()>(None).context("loading config")?;
    let cache = crate::storage::Cache::open(cfg.cache.resolved_path()).context("opening cache")?;

    let router = crate::web::router(cache, args.allow_scan);

    let listener = tokio::net::TcpListener::bind(&args.bind)
        .await
        .with_context(|| format!("binding TCP listener at {}", args.bind))?;
    let local_addr = listener.local_addr().ok();

    tracing::info!(addr = %args.bind, allow_scan = args.allow_scan, "serving repo-trust web viewer");
    if !is_localhost_bind(&args.bind) {
        tracing::warn!(
            addr = %args.bind,
            "non-localhost bind: the web viewer is now reachable from the network. \
             See docs/architecture.md §12."
        );
    }
    if let Some(addr) = local_addr {
        eprintln!("repo-trust serve: listening on http://{addr}");
    }

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum::serve failed")?;

    Ok(0)
}

/// Returns `true` if the bind address is a loopback host
/// (`127.x.x.x`, `[::1]`, or the literal `localhost`). Anything else
/// triggers the warn-level log on startup.
#[cfg(feature = "web")]
fn is_localhost_bind(bind: &str) -> bool {
    // Strip port and IPv6 brackets for the host comparison.
    let host = match bind.rsplit_once(':') {
        Some((host, _)) => host,
        None => bind,
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host == "localhost" || host.starts_with("127.") || host == "::1"
}

#[cfg(feature = "web")]
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received; stopping web viewer");
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::is_localhost_bind;

    #[test]
    fn localhost_variants_are_localhost() {
        assert!(is_localhost_bind("127.0.0.1:8765"));
        assert!(is_localhost_bind("127.1.2.3:8765"));
        assert!(is_localhost_bind("localhost:8765"));
        assert!(is_localhost_bind("[::1]:8765"));
    }

    #[test]
    fn external_bind_is_not_localhost() {
        assert!(!is_localhost_bind("0.0.0.0:8765"));
        assert!(!is_localhost_bind("10.0.0.5:8765"));
        assert!(!is_localhost_bind("[::]:8765"));
    }
}
