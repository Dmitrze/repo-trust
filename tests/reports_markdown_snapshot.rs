#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

//! Insta snapshot test for the markdown report writer.
//!
//! Covers scenario S-401: a known-good 5-module `TrustReport` fixture
//! renders to a stable markdown string. `snapshot_at` and
//! `runtime_seconds` are pinned so the snapshot is byte-deterministic.
//!
//! Spec: `specs/reports-markdown.md`
//! Scenario: `tests/scenarios/reports-markdown.md` S-401

use std::collections::BTreeMap;

use repo_trust::models::evidence::{EvidenceItem, Verdict};
use repo_trust::models::reports::{Mode, TrustReport};
use repo_trust::models::repository::RepositorySummary;
use repo_trust::models::scores::{Category, Confidence, ModuleResult, ModuleWeights};
use time::OffsetDateTime;

fn pinned_snapshot_at() -> OffsetDateTime {
    // 2026-05-03T08:23:45Z — pinned for snapshot determinism.
    // (Comment was incorrect on Day 4; reconciled per docs/day-5-polish.md §2c.)
    OffsetDateTime::from_unix_timestamp(1_777_796_625).unwrap()
}

fn ev(
    module: &str,
    code: &str,
    label: &str,
    value: serde_json::Value,
    threshold: Option<serde_json::Value>,
    verdict: Verdict,
    rationale: &str,
) -> EvidenceItem {
    EvidenceItem {
        module: module.to_string(),
        code: code.to_string(),
        label: label.to_string(),
        value,
        threshold,
        verdict,
        rationale: rationale.to_string(),
    }
}

fn module(name: &str, score: u8, conf: Confidence, sub: &[(&str, u8)]) -> ModuleResult {
    let mut sub_scores = BTreeMap::new();
    for (k, v) in sub {
        sub_scores.insert((*k).to_string(), *v);
    }
    ModuleResult {
        module: name.to_string(),
        score,
        confidence: conf,
        sub_scores,
        sample_size: None,
        missing_data: Vec::new(),
    }
}

/// Inactive 5-module baseline, modeled on `octocat/Hello-World`.
/// Evidence is sorted (module, code) in line with the JSON writer's
/// determinism contract (ADR-0007).
fn fixture_report() -> TrustReport {
    let modules = vec![
        module(
            "stars",
            72,
            Confidence::Medium,
            &[
                ("fork_to_star_ratio", 60),
                ("low_activity_share", 80),
                ("watcher_to_star_ratio", 70),
            ],
        ),
        module(
            "activity",
            65,
            Confidence::High,
            &[
                ("commit_frequency", 60),
                ("pull_request_velocity", 70),
                ("release_cadence", 65),
            ],
        ),
        module(
            "maintainers",
            58,
            Confidence::High,
            &[
                ("bus_factor_proxy", 55),
                ("commit_concentration", 65),
                ("contributor_retention", 50),
                ("governance_docs", 60),
            ],
        ),
        module(
            "adoption",
            70,
            Confidence::Medium,
            &[
                ("documentation_maturity", 75),
                ("package_systems_count", 50),
                ("weekly_downloads", 75),
            ],
        ),
        module(
            "security",
            68,
            Confidence::High,
            &[
                ("doc_presence", 80),
                ("scorecard_aggregate", 60),
                ("semver_consistency", 65),
            ],
        ),
    ];

    let mut evidence = vec![
        ev(
            "activity",
            "commit_frequency",
            "Commit frequency",
            serde_json::json!(12),
            Some(serde_json::json!(20)),
            Verdict::Concerning,
            "12 commits in the last 90 days, below the 20 threshold.",
        ),
        ev(
            "activity",
            "last_commit_age_days",
            "Days since last commit",
            serde_json::json!(45),
            Some(serde_json::json!(30)),
            Verdict::Concerning,
            "Last commit was 45 days ago.",
        ),
        ev(
            "activity",
            "release_cadence",
            "Releases in 18 months",
            serde_json::json!(2),
            Some(serde_json::json!(4)),
            Verdict::Neutral,
            "2 releases observed.",
        ),
        ev(
            "adoption",
            "documentation_maturity",
            "Documentation maturity",
            serde_json::json!(0.75),
            None,
            Verdict::Positive,
            "README, docs/ and examples/ all present.",
        ),
        ev(
            "adoption",
            "package_systems_count",
            "Package systems",
            serde_json::json!(1),
            None,
            Verdict::Neutral,
            "Published to a single package system.",
        ),
        ev(
            "adoption",
            "weekly_downloads",
            "Weekly downloads",
            serde_json::json!(15_000),
            Some(serde_json::json!(10_000)),
            Verdict::Positive,
            "15k weekly downloads across packages.",
        ),
        ev(
            "maintainers",
            "bus_factor_proxy",
            "Bus factor proxy",
            serde_json::json!(2),
            Some(serde_json::json!(5)),
            Verdict::Concerning,
            "Only 2 effective maintainers in the last year.",
        ),
        ev(
            "maintainers",
            "commit_concentration_gini",
            "Commit-share Gini coefficient",
            serde_json::json!(0.78),
            Some(serde_json::json!(0.40)),
            Verdict::Concerning,
            "Gini of 0.78 indicates concentrated commit share.",
        ),
        ev(
            "maintainers",
            "governance_docs",
            "Governance docs present",
            serde_json::json!(true),
            None,
            Verdict::Positive,
            "CODEOWNERS and GOVERNANCE.md both present.",
        ),
        ev(
            "security",
            "doc_presence",
            "Required docs present",
            serde_json::json!(true),
            None,
            Verdict::Positive,
            "SECURITY.md, CONTRIBUTING.md and CODE_OF_CONDUCT.md present.",
        ),
        ev(
            "security",
            "scorecard_score",
            "OpenSSF Scorecard score",
            serde_json::json!(7.5),
            Some(serde_json::json!(5.0)),
            Verdict::Positive,
            "Scorecard score 7.5 is above the 5.0 threshold.",
        ),
        ev(
            "security",
            "semver_consistency",
            "SemVer consistency",
            serde_json::json!(true),
            None,
            Verdict::Positive,
            "All releases use vX.Y.Z tags.",
        ),
        ev(
            "stars",
            "fork_to_star_ratio",
            "Fork-to-star ratio",
            serde_json::json!(0.05),
            Some(serde_json::json!(0.04)),
            Verdict::Positive,
            "Fork-to-star ratio is healthy for the language.",
        ),
        ev(
            "stars",
            "low_activity_stargazer_share",
            "Low-activity stargazer share",
            serde_json::json!(0.08),
            Some(serde_json::json!(0.20)),
            Verdict::Positive,
            "8% of sampled stargazers match a low-activity profile.",
        ),
        ev(
            "stars",
            "watcher_to_star_ratio",
            "Watcher-to-star ratio",
            serde_json::json!(0.006),
            Some(serde_json::json!(0.005)),
            Verdict::Positive,
            "Watcher-to-star ratio is healthy.",
        ),
    ];
    evidence.sort_by(|a, b| {
        (a.module.as_str(), a.code.as_str()).cmp(&(b.module.as_str(), b.code.as_str()))
    });

    let top_strengths = vec![
        ev(
            "security",
            "scorecard_score",
            "OpenSSF Scorecard score",
            serde_json::json!(7.5),
            Some(serde_json::json!(5.0)),
            Verdict::Positive,
            "Scorecard score 7.5 is above the 5.0 threshold.",
        ),
        ev(
            "stars",
            "low_activity_stargazer_share",
            "Low-activity stargazer share",
            serde_json::json!(0.08),
            Some(serde_json::json!(0.20)),
            Verdict::Positive,
            "8% of sampled stargazers match a low-activity profile.",
        ),
    ];
    let top_concerns = vec![ev(
        "maintainers",
        "bus_factor_proxy",
        "Bus factor proxy",
        serde_json::json!(2),
        Some(serde_json::json!(5)),
        Verdict::Concerning,
        "Only 2 effective maintainers in the last year.",
    )];

    TrustReport {
        schema_version: "1.0.0".to_string(),
        repository: RepositorySummary {
            full_name: "octocat/Hello-World".to_string(),
            url: "https://github.com/octocat/Hello-World".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 1000,
            snapshot_at: pinned_snapshot_at(),
        },
        overall_score: 67,
        overall_confidence: Confidence::Medium,
        category: Category::Mixed,
        mode: Mode::Standard,
        modules,
        evidence,
        top_strengths,
        top_concerns,
        caveats: vec!["Stargazer sample size limited by API budget.".to_string()],
        scoring_version: "1.0.0".to_string(),
        weights_used: ModuleWeights::default(),
        snapshot_at: pinned_snapshot_at(),
        runtime_seconds: 12.3,
    }
}

#[test]
fn markdown_report_matches_inactive_baseline_snapshot() {
    let report = fixture_report();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("octocat_Hello-World.md");
    repo_trust::reports::markdown_report::write(&report, &path).expect("write succeeds");
    let rendered = std::fs::read_to_string(&path).expect("read back rendered markdown");

    insta::assert_snapshot!("inactive_baseline_5_modules", rendered);
}
