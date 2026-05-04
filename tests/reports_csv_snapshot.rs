//! Insta snapshot test for the CSV report writer.
//!
//! Builds a fixed 5-module `TrustReport` and asserts the byte-exact
//! header + row output. `snapshot_at` and `runtime_seconds` are pinned so
//! the snapshot is stable across runs (per ADR-0007).

use std::collections::BTreeMap;

use repo_trust::models::evidence::{EvidenceItem, Verdict};
use repo_trust::models::repository::RepositorySummary;
use repo_trust::models::scores::{Category, Confidence, ModuleResult, ModuleWeights};
use repo_trust::models::{Mode, TrustReport};
use repo_trust::reports::csv_report;
use time::macros::datetime;

fn module(name: &str, score: u8, conf: Confidence) -> ModuleResult {
    ModuleResult {
        module: name.to_string(),
        score,
        confidence: conf,
        sub_scores: BTreeMap::new(),
        sample_size: None,
        missing_data: Vec::new(),
    }
}

fn fixture() -> TrustReport {
    let modules = vec![
        module("stars", 81, Confidence::High),
        module("activity", 72, Confidence::High),
        module("maintainers", 68, Confidence::High),
        module("adoption", 75, Confidence::Medium),
        module("security", 68, Confidence::High),
    ];

    let top_concern = EvidenceItem {
        module: "adoption".to_string(),
        code: "no_packages".to_string(),
        label: "No packages mapped".to_string(),
        value: serde_json::Value::Null,
        threshold: None,
        verdict: Verdict::Concerning,
        rationale: "deps.dev returned no packages for this repo.".to_string(),
    };

    TrustReport {
        schema_version: "1.0.0".to_string(),
        repository: RepositorySummary {
            full_name: "acme/widget".to_string(),
            url: "https://github.com/acme/widget".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 250,
            snapshot_at: datetime!(2026-05-04 10:23:45 UTC),
        },
        overall_score: 73,
        overall_confidence: Confidence::High,
        category: Category::Good,
        mode: Mode::Standard,
        modules,
        evidence: Vec::new(),
        top_strengths: Vec::new(),
        top_concerns: vec![top_concern],
        caveats: Vec::new(),
        scoring_version: "1.0.0".to_string(),
        weights_used: ModuleWeights::default(),
        snapshot_at: datetime!(2026-05-04 10:23:45 UTC),
        runtime_seconds: 12.3,
    }
}

#[test]
fn csv_report_snapshot_5_module_baseline() {
    let report = fixture();
    let mut buf = Vec::new();
    csv_report::write_header(&mut buf).unwrap();
    csv_report::write_row(&report, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    insta::assert_snapshot!("csv_5_module_baseline", s);
}
