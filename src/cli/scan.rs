//! `scan` — evaluate a single repository.
//!
//! Day 1 partial wire: only Activity Health runs end-to-end. The remaining
//! modules (maintainers, security, stars, adoption) land Day 2/3.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Args;
use time::OffsetDateTime;

use crate::api::github::Client as GhClient;
use crate::api::github::GithubError;
use crate::api::osv::Client as OsvClient;
use crate::api::scorecard::Client as ScorecardClient;
use crate::config;
use crate::models::{
    Category, ModuleResult, ModuleWeights, RepositoryContext, RepositorySummary, TrustReport,
};
use crate::reports::json_report;
use crate::scoring::{aggregate, overall_confidence};
use crate::storage::Cache;
use crate::utils::{ratelimit::RateLimiter, repo_url};

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

    /// Override the GitHub API base URL. Hidden — used by integration tests
    /// to point at a wiremock server.
    #[arg(long, hide = true, env = "REPO_TRUST_API_BASE_URL")]
    pub api_base_url: Option<String>,
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

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Format {
    Terminal,
    Json,
    Md,
    Csv,
    Sarif,
}

pub async fn execute(args: ScanArgs) -> Result<u8> {
    let started = Instant::now();
    tracing::info!(repo = %args.repo, mode = ?args.mode, "scan starting");

    // ─── Repo URL ──────────────────────────────────────────────────────
    let full_name = repo_url::parse(&args.repo).context("invalid repo identifier")?;
    let canonical_url = url::Url::parse(&format!("https://github.com/{full_name}"))?;

    // ─── Config ────────────────────────────────────────────────────────
    let cfg = config::load::<()>(None).context("loading config")?;
    let token = args.token.or_else(|| cfg.github.resolve_token());
    if token.is_none() {
        tracing::warn!("no GitHub token configured; running unauthenticated (60 req/h limit)");
    }
    let weights = if let Some(p) = &args.weights {
        crate::scoring::weights::load(p).context("loading custom weights")?
    } else {
        ModuleWeights::from(cfg.weights)
    };

    // ─── Cache ─────────────────────────────────────────────────────────
    let cache_path = cfg.cache.resolved_path();
    let cache = Cache::open(&cache_path).context("opening cache")?;
    if args.refresh {
        let n = cache.delete_by_repo(&full_name)?;
        tracing::info!(invalidated = n, "cache invalidated for repo");
    }

    // ─── HTTP / federated clients ────────────────────────────────────
    let http = crate::api::client::build()?;
    let limiter = RateLimiter::default();
    let mut github = GhClient::new(http.clone(), cache.clone(), limiter, token);
    let mut scorecard = ScorecardClient::new(http.clone(), cache.clone());
    let mut osv = OsvClient::new(http.clone(), cache.clone());
    if let Some(base) = args.api_base_url.as_deref() {
        github = github.with_base_url(base);
        // The same wiremock server hosts the federated mocks in tests; in
        // production the federated clients hit their real endpoints.
        scorecard = scorecard.with_base_url(base);
        osv = osv.with_base_url(base);
    }

    // ─── Scoring version + seed ────────────────────────────────────────
    let scoring_version = match &args.scoring_version {
        Some(s) => semver::Version::parse(s).context("invalid scoring version")?,
        None => semver::Version::parse(crate::SCORING_VERSION)
            .expect("crate SCORING_VERSION is valid SemVer"),
    };
    let rng_seed = args.seed.unwrap_or_else(|| {
        crate::utils::sampling::derive_seed(&full_name, &scoring_version.to_string())
    });
    let snapshot_at = OffsetDateTime::now_utc();

    // ─── Build context ────────────────────────────────────────────────
    let ctx = RepositoryContext {
        full_name: full_name.clone(),
        canonical_url,
        mode: args.mode,
        scoring_version: scoring_version.clone(),
        weights,
        rng_seed,
        snapshot_at,
        cache,
        github: github.clone(),
        scorecard,
        osv,
    };

    // ─── Run modules ──────────────────────────────────────────────────
    // Day 1: only Activity is wired end-to-end. Day 2 adds Maintainers + Security.
    // Day 3 adds Stars + Adoption.
    let selected = select_modules(args.modules.as_ref(), args.skip_modules.as_ref());
    let mut module_results: Vec<ModuleResult> = Vec::new();
    let mut all_evidence = Vec::new();

    use crate::modules::TrustModule;
    for name in &selected {
        let result = match name.as_str() {
            "activity" => {
                let m = crate::modules::activity::ActivityModule;
                Some(m.run(&ctx).await)
            },
            "maintainers" => {
                let m = crate::modules::maintainers::MaintainersModule;
                Some(m.run(&ctx).await)
            },
            "security" => {
                let m = crate::modules::security::SecurityModule;
                Some(m.run(&ctx).await)
            },
            // Stars + Adoption land Day 3.
            other => {
                tracing::debug!(module = other, "skipping (not yet wired)");
                None
            },
        };
        if let Some(res) = result {
            let (r, ev) = res.with_context(|| format!("module '{name}' failed"))?;
            module_results.push(r);
            all_evidence.extend(ev);
        }
    }

    // ─── Aggregate ────────────────────────────────────────────────────
    let overall_score = aggregate(&module_results, &ctx.weights);
    let overall_conf = overall_confidence(&module_results, &ctx.weights);
    let category = Category::from_score(overall_score);
    let (top_strengths, top_concerns) =
        crate::scoring::explain::top_strengths_and_concerns(&all_evidence, 3);

    // ─── Repo summary (cheap re-fetch from cache; metadata was warmed by activity) ─
    let (owner, name) = full_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("invalid full_name"))?;
    let summary = match github.get_repo(owner, name).await {
        Ok(r) => RepositorySummary {
            full_name: r.full_name,
            url: r.html_url,
            default_branch: r.default_branch,
            primary_language: r.language,
            stars: r.stargazers_count,
            snapshot_at,
        },
        Err(e) => {
            // Map typed error → exit code per architecture §8.
            return Err(map_github_error(&e).unwrap_or(e));
        },
    };

    // ─── Build report ─────────────────────────────────────────────────
    let runtime_seconds = started.elapsed().as_secs_f64();
    let mut evidence_sorted = all_evidence;
    evidence_sorted.sort_by(|a, b| {
        (a.module.as_str(), a.code.as_str()).cmp(&(b.module.as_str(), b.code.as_str()))
    });

    let report = TrustReport {
        schema_version: crate::REPORT_SCHEMA_VERSION.to_string(),
        repository: summary,
        overall_score,
        overall_confidence: overall_conf,
        category,
        mode: match args.mode {
            Mode::Quick => crate::models::Mode::Quick,
            Mode::Standard => crate::models::Mode::Standard,
            Mode::Deep => crate::models::Mode::Deep,
        },
        modules: module_results,
        evidence: evidence_sorted,
        top_strengths,
        top_concerns,
        caveats: Vec::new(),
        scoring_version: scoring_version.to_string(),
        weights_used: ctx.weights,
        snapshot_at,
        runtime_seconds: crate::utils::time::round6(runtime_seconds),
    };

    // ─── Write reports ────────────────────────────────────────────────
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("creating output dir {:?}", args.output))?;
    let safe = full_name.replace('/', "_");
    let json_path = args.output.join(format!("{safe}.json"));
    json_report::write(&report, &json_path)?;
    if !args.quiet {
        println!(
            "wrote {} (score {} / {}, confidence {:?})",
            json_path.display(),
            report.overall_score,
            mode_label(args.mode),
            report.overall_confidence,
        );
    }

    // Cache the report.
    let json_bytes = serde_json::to_vec(&report)?;
    ctx.cache.put_report(
        &full_name,
        args.mode.as_str(),
        &scoring_version.to_string(),
        &json_bytes,
    )?;

    Ok(0)
}

fn mode_label(m: Mode) -> &'static str {
    m.as_str()
}

fn select_modules(enabled: Option<&Vec<String>>, skipped: Option<&Vec<String>>) -> Vec<String> {
    // Day 2 default set: 3 modules wired end-to-end. Stars + Adoption land Day 3.
    let default_set = vec![
        "activity".to_string(),
        "maintainers".to_string(),
        "security".to_string(),
    ];
    let mut selected: Vec<String> = match enabled {
        Some(list) if !list.is_empty() => list.clone(),
        _ => default_set,
    };
    if let Some(skip) = skipped {
        selected.retain(|m| !skip.iter().any(|s| s == m));
    }
    selected
}

fn map_github_error(e: &anyhow::Error) -> Option<anyhow::Error> {
    // Surface as the original anyhow::Error — exit-code mapping happens in
    // cli::run via downcast in a future refactor. For Day 1 we just propagate.
    let _ = e;
    None
}

/// Map a `GithubError` to the architecture-§8 exit code.
#[must_use]
pub fn exit_code_for(error: &anyhow::Error) -> u8 {
    match error.downcast_ref::<GithubError>() {
        Some(GithubError::NotFound) => 2,
        Some(GithubError::Unauthorized) => 3,
        Some(GithubError::Forbidden(_)) => 4,
        _ => 1,
    }
}
