//! Repo Trust binary entry point.
//!
//! Thin wrapper around `repo_trust::cli::run`. All real logic lives in the library
//! crate (`src/lib.rs`) so it can be tested and embedded.

fn main() -> std::process::ExitCode {
    // Set up tracing-subscriber from RUST_LOG env var.
    repo_trust::utils::tracing::init();

    // Run the CLI; the function returns a numeric exit code per `docs/architecture.md` §8.
    let exit_code = match repo_trust::cli::run() {
        Ok(code) => code,
        Err(err) => {
            tracing::error!(error = ?err, "unhandled error");
            eprintln!("error: {err:#}");
            1
        },
    };

    std::process::ExitCode::from(exit_code)
}
