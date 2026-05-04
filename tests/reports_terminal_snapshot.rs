//! Insta snapshot test for the terminal report writer.
//!
//! Spec: `specs/reports-terminal.md` S-401 — render a fixed
//! "octocat/Hello-World inactive baseline" `TrustReport` with `color = false`
//! and assert byte-equality against the committed snapshot.
//!
//! The fixture mirrors the example block in spec §5 so the snapshot doubles
//! as a visual reference of what the writer produces. `snapshot_at` is
//! pinned to a deterministic value; no insta redactions are required.

use std::collections::BTreeMap;

use repo_trust::models::{
    evidence::Verdict, Category, Confidence, EvidenceItem, Mode, ModuleResult, ModuleWeights,
    RepositorySummary, TrustReport,
};
use repo_trust::reports::terminal;
use time::macros::datetime;

fn module(
    name: &str,
    score: u8,
    confidence: Confidence,
    sub: &[(&str, u8)],
    missing: &[&str],
) -> ModuleResult {
    let mut sub_scores = BTreeMap::new();
    for (k, v) in sub {
        sub_scores.insert((*k).to_string(), *v);
    }
    ModuleResult {
        module: name.to_string(),
        score,
        confidence,
        sub_scores,
        sample_size: None,
        missing_data: missing.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn ev(module: &str, code: &str, verdict: Verdict, rationale: &str) -> EvidenceItem {
    EvidenceItem {
        module: module.to_string(),
        code: code.to_string(),
        label: code.replace('_', " "),
        value: serde_json::json!(0),
        threshold: None,
        verdict,
        rationale: rationale.to_string(),
    }
}

fn inactive_baseline_report() -> TrustReport {
    TrustReport {
        schema_version: "1.0.0".to_string(),
        repository: RepositorySummary {
            full_name: "octocat/Hello-World".to_string(),
            url: "https://github.com/octocat/Hello-World".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 80,
            snapshot_at: datetime!(2026-05-04 10:23:45 UTC),
        },
        overall_score: 12,
        overall_confidence: Confidence::High,
        category: Category::HighRisk,
        mode: Mode::Standard,
        modules: vec![
            module(
                "stars",
                45,
                Confidence::Medium,
                &[("low_activity_share", 85), ("watcher_to_star_ratio", 30)],
                &[],
            ),
            module(
                "activity",
                8,
                Confidence::High,
                &[("commits_last_90d", 0)],
                &["no_releases"],
            ),
            module(
                "maintainers",
                25,
                Confidence::Low,
                &[("bus_factor_proxy", 25)],
                &[],
            ),
            module(
                "adoption",
                30,
                Confidence::Medium,
                &[("documentation_maturity", 60)],
                &["no_packages"],
            ),
            module(
                "security",
                55,
                Confidence::Medium,
                &[("documentation_presence", 80)],
                &[],
            ),
        ],
        evidence: vec![],
        top_strengths: vec![
            ev(
                "security",
                "documentation_presence",
                Verdict::Positive,
                "4/5 expected docs present (LICENSE/CONTRIBUTING/CODE_OF_CONDUCT/CODEOWNERS)",
            ),
            ev(
                "stars",
                "watcher_to_star_ratio",
                Verdict::Positive,
                "watcher/star ratio = 0.0162; ecosystem-adjusted threshold >= 0.0050",
            ),
        ],
        top_concerns: vec![
            ev(
                "activity",
                "no_commits_in_window",
                Verdict::HighRisk,
                "The 18-month commit window contained zero commits.",
            ),
            ev(
                "maintainers",
                "solo_maintainer",
                Verdict::Concerning,
                "Only one human author committed in the last year.",
            ),
            ev(
                "adoption",
                "no_readme",
                Verdict::Concerning,
                "README endpoint returned 404.",
            ),
        ],
        caveats: vec![
            "osv_deferred_to_phase_3".to_string(),
            "recency_biased_sample".to_string(),
        ],
        scoring_version: "1.0.0".to_string(),
        weights_used: ModuleWeights::default(),
        snapshot_at: datetime!(2026-05-04 10:23:45 UTC),
        runtime_seconds: 1.234567,
    }
}

#[test]
fn snapshot_inactive_baseline_no_color() {
    let report = inactive_baseline_report();
    let mut buf = Vec::new();
    terminal::write(&report, &mut buf, false).expect("write succeeds");
    let rendered = String::from_utf8(buf).expect("utf-8 output");
    insta::assert_snapshot!("terminal_inactive_baseline", rendered);
}
