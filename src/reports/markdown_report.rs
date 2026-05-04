//! Markdown report writer.
//!
//! Emits a self-contained, GitHub-flavored markdown document for a
//! [`TrustReport`]. The output is suitable for embedding in a PR body, an
//! issue comment, or rendering through any GFM-compliant renderer
//! (GitHub, VS Code, mdBook, pulldown-cmark).
//!
//! See `specs/reports-markdown.md` for the full contract and
//! `tests/scenarios/reports-markdown.md` for the scenarios this module
//! must satisfy.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use time::format_description::well_known::Iso8601;

use crate::models::evidence::EvidenceItem;
use crate::models::reports::Mode;
use crate::models::scores::{Confidence, ModuleResult};
use crate::models::TrustReport;

/// Write a markdown rendering of `report` to `path`.
///
/// Output is byte-deterministic given the same `TrustReport` (modulo
/// `snapshot_at` and `runtime_seconds`, which the caller pins for tests
/// per ADR-0007).
pub fn write(report: &TrustReport, path: &Path) -> Result<()> {
    let s = render(report);
    let mut f =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    f.write_all(s.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Render a [`TrustReport`] to a `String`. Exposed for tests + the
/// pulldown-cmark round-trip check.
fn render(report: &TrustReport) -> String {
    let mut out = String::with_capacity(4096);
    write_header(&mut out, report);
    for module in &report.modules {
        let evidence: Vec<&EvidenceItem> = report
            .evidence
            .iter()
            .filter(|e| e.module == module.module)
            .collect();
        write_module_section(&mut out, module, &evidence);
    }
    write_strengths(&mut out, &report.top_strengths);
    write_concerns(&mut out, &report.top_concerns);
    write_caveats(&mut out, &report.caveats);
    write_methodology(&mut out, &report.scoring_version);
    out
}

fn write_header(out: &mut String, report: &TrustReport) {
    let _ = writeln!(out, "# Trust Report — {}", report.repository.full_name);
    let _ = writeln!(out);
    let _ = writeln!(out, "| Field | Value |");
    let _ = writeln!(out, "|-|-|");
    let _ = writeln!(out, "| Overall Score | {} |", report.overall_score);
    let _ = writeln!(out, "| Category | {:?} |", report.category);
    let _ = writeln!(
        out,
        "| Confidence | {} |",
        confidence_label(report.overall_confidence)
    );
    let _ = writeln!(out, "| Mode | {} |", mode_label(report.mode));
    let _ = writeln!(out, "| Scoring Version | {} |", report.scoring_version);
    let _ = writeln!(out, "| Snapshot | {} |", iso8601(report));
    let _ = writeln!(out, "| Runtime | {:.1}s |", report.runtime_seconds);
    let _ = writeln!(out);
}

fn write_module_section(out: &mut String, module: &ModuleResult, evidence: &[&EvidenceItem]) {
    let _ = writeln!(
        out,
        "## {} (`{}`)",
        module_title(&module.module),
        module.module
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "**Score:** {} / 100  **Confidence:** {}",
        module.score,
        confidence_label(module.confidence),
    );
    let _ = writeln!(out);

    // Sub-scores.
    let _ = writeln!(out, "### Sub-scores");
    let _ = writeln!(out);
    if module.sub_scores.is_empty() {
        let _ = writeln!(out, "_(none)_");
    } else {
        let _ = writeln!(out, "| Key | Value |");
        let _ = writeln!(out, "|-|-|");
        // BTreeMap iteration is already sorted — that's deterministic.
        for (k, v) in &module.sub_scores {
            let _ = writeln!(out, "| {} | {} |", escape_cell(k), v);
        }
    }
    let _ = writeln!(out);

    // Evidence.
    let _ = writeln!(out, "### Evidence");
    let _ = writeln!(out);
    if evidence.is_empty() {
        let _ = writeln!(out, "_(no evidence emitted)_");
    } else {
        let _ = writeln!(
            out,
            "| Code | Label | Value | Threshold | Verdict | Rationale |"
        );
        let _ = writeln!(out, "|-|-|-|-|-|-|");
        for ev in evidence {
            let value = json_inline(&ev.value);
            let threshold = match &ev.threshold {
                Some(t) => format!("`{}`", escape_cell(&json_inline_raw(t))),
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "| {} | {} | `{}` | {} | {:?} | {} |",
                escape_cell(&ev.code),
                escape_cell(&ev.label),
                escape_cell(&value),
                threshold,
                ev.verdict,
                escape_cell(&ev.rationale),
            );
        }
    }
    let _ = writeln!(out);

    // Missing data.
    if !module.missing_data.is_empty() {
        let joined = module
            .missing_data
            .iter()
            .map(|s| escape_cell(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "*Missing data:* {joined}");
        let _ = writeln!(out);
    }
}

fn write_strengths(out: &mut String, strengths: &[EvidenceItem]) {
    if strengths.is_empty() {
        // Keep the header to give consumers a stable anchor; emit an
        // explicit "_(none)_" placeholder so renderers don't show an
        // empty section. The scenarios only require omission for
        // top_concerns (S-102), not strengths.
        let _ = writeln!(out, "## Top Strengths");
        let _ = writeln!(out);
        let _ = writeln!(out, "_(none)_");
        let _ = writeln!(out);
        return;
    }
    let _ = writeln!(out, "## Top Strengths");
    let _ = writeln!(out);
    for ev in strengths {
        let _ = writeln!(
            out,
            "- **[{}] {}** — {}",
            escape_cell(&ev.module),
            escape_cell(&ev.code),
            escape_cell(&ev.rationale),
        );
    }
    let _ = writeln!(out);
}

fn write_concerns(out: &mut String, concerns: &[EvidenceItem]) {
    // S-102: empty top_concerns omits the entire section (header included).
    if concerns.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Top Concerns");
    let _ = writeln!(out);
    for ev in concerns {
        let _ = writeln!(
            out,
            "- **[{}] {}** — {}",
            escape_cell(&ev.module),
            escape_cell(&ev.code),
            escape_cell(&ev.rationale),
        );
    }
    let _ = writeln!(out);
}

fn write_caveats(out: &mut String, caveats: &[String]) {
    if caveats.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Caveats");
    let _ = writeln!(out);
    for c in caveats {
        let _ = writeln!(out, "- {}", escape_cell(c));
    }
    let _ = writeln!(out);
}

fn write_methodology(out: &mut String, scoring_version: &str) {
    let _ = writeln!(out, "## Methodology");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "This report was generated by [repo-trust](https://github.com/Dmitrze/repo-trust)"
    );
    let _ = writeln!(
        out,
        "using scoring model v{scoring_version}. See [methodology](https://github.com/Dmitrze/repo-trust/blob/main/docs/methodology.md)"
    );
    let _ = writeln!(out, "for the full algorithm and threshold tables.");
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// Title-case label for a module key.
fn module_title(module: &str) -> &'static str {
    match module {
        "stars" => "Star Authenticity",
        "activity" => "Activity Health",
        "maintainers" => "Maintainer Health",
        "adoption" => "Adoption Signals",
        "security" => "Security & Readiness",
        // Fallback for forward-compat / unknown modules. The current
        // registry only emits the five names above.
        _ => "Module",
    }
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Quick => "quick",
        Mode::Standard => "standard",
        Mode::Deep => "deep",
    }
}

fn confidence_label(c: Confidence) -> &'static str {
    match c {
        Confidence::Low => "Low",
        Confidence::Medium => "Medium",
        Confidence::High => "High",
    }
}

fn iso8601(report: &TrustReport) -> String {
    report
        .snapshot_at
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Render a `serde_json::Value` for a markdown table cell. Strips the
/// surrounding quotes from string values so the visible rendering
/// reads naturally; numbers / bools / arrays / objects fall through
/// to JSON. The caller wraps the result in backticks.
fn json_inline(v: &serde_json::Value) -> String {
    json_inline_raw(v)
}

fn json_inline_raw(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Escape characters that would otherwise break a GFM table cell.
///
/// - `|` becomes `\|` (column separator).
/// - Newlines become a literal space (cells must be single-line).
/// - All other characters pass through.
fn escape_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '|' => out.push_str(r"\|"),
            '\n' | '\r' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::evidence::{EvidenceItem, Verdict};
    use crate::models::reports::Mode;
    use crate::models::repository::RepositorySummary;
    use crate::models::scores::{Category, Confidence, ModuleResult, ModuleWeights};
    use std::collections::BTreeMap;
    use time::OffsetDateTime;

    fn fixture_summary() -> RepositorySummary {
        RepositorySummary {
            full_name: "octocat/Hello-World".to_string(),
            url: "https://github.com/octocat/Hello-World".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 1000,
            snapshot_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        }
    }

    fn ev(
        module: &str,
        code: &str,
        label: &str,
        rationale: &str,
        verdict: Verdict,
    ) -> EvidenceItem {
        EvidenceItem {
            module: module.to_string(),
            code: code.to_string(),
            label: label.to_string(),
            value: serde_json::json!(42),
            threshold: Some(serde_json::json!(50)),
            verdict,
            rationale: rationale.to_string(),
        }
    }

    fn module(name: &str, score: u8) -> ModuleResult {
        let mut sub = BTreeMap::new();
        sub.insert("alpha".to_string(), 80);
        sub.insert("beta".to_string(), 70);
        ModuleResult {
            module: name.to_string(),
            score,
            confidence: Confidence::High,
            sub_scores: sub,
            sample_size: None,
            missing_data: Vec::new(),
        }
    }

    fn fixture_report(modules: Vec<ModuleResult>, evidence: Vec<EvidenceItem>) -> TrustReport {
        TrustReport {
            schema_version: "1.0.0".to_string(),
            repository: fixture_summary(),
            overall_score: 73,
            overall_confidence: Confidence::High,
            category: Category::Good,
            mode: Mode::Standard,
            modules,
            evidence,
            top_strengths: vec![ev(
                "activity",
                "recent_commits",
                "Recent commits",
                "12 commits in the last 90 days.",
                Verdict::Positive,
            )],
            top_concerns: vec![],
            caveats: vec![],
            scoring_version: "1.0.0".to_string(),
            weights_used: ModuleWeights::default(),
            snapshot_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            runtime_seconds: 12.3,
        }
    }

    #[test]
    fn five_module_report_contains_all_five_h2_headers() {
        let modules = vec![
            module("stars", 81),
            module("activity", 74),
            module("maintainers", 68),
            module("adoption", 70),
            module("security", 72),
        ];
        let report = fixture_report(modules, Vec::new());
        let out = render(&report);
        assert!(out.contains("## Star Authenticity (`stars`)"));
        assert!(out.contains("## Activity Health (`activity`)"));
        assert!(out.contains("## Maintainer Health (`maintainers`)"));
        assert!(out.contains("## Adoption Signals (`adoption`)"));
        assert!(out.contains("## Security & Readiness (`security`)"));
    }

    #[test]
    fn output_round_trips_through_pulldown_cmark_without_panic() {
        let modules = vec![module("activity", 74), module("security", 72)];
        let evidence = vec![ev(
            "activity",
            "commit_freq",
            "Commit frequency",
            "Plenty of commits.",
            Verdict::Positive,
        )];
        let report = fixture_report(modules, evidence);
        let out = render(&report);
        // Drain the parser to assert no panic and well-formed GFM.
        let parser = pulldown_cmark::Parser::new(&out);
        let count = parser.count();
        assert!(count > 0, "parser produced no events");
    }

    #[test]
    fn pipe_in_evidence_rationale_is_escaped() {
        let modules = vec![module("activity", 74)];
        let evidence = vec![ev(
            "activity",
            "boundary",
            "Boundary check",
            "value is a|b across the | boundary",
            Verdict::Concerning,
        )];
        let report = fixture_report(modules, evidence);
        let out = render(&report);
        assert!(
            out.contains(r"value is a\|b across the \| boundary"),
            "expected escaped pipe in rationale, got:\n{out}"
        );
    }

    #[test]
    fn empty_top_concerns_does_not_emit_header() {
        let report = fixture_report(vec![module("activity", 74)], Vec::new());
        let out = render(&report);
        assert!(
            !out.contains("## Top Concerns"),
            "empty top_concerns must not emit the header (S-102), got:\n{out}"
        );
    }

    #[test]
    fn methodology_footer_mentions_scoring_version() {
        let mut report = fixture_report(vec![module("activity", 74)], Vec::new());
        report.scoring_version = "2.4.7".to_string();
        let out = render(&report);
        assert!(out.contains("## Methodology"));
        assert!(
            out.contains("scoring model v2.4.7"),
            "methodology footer should mention scoring_version"
        );
    }

    #[test]
    fn pipe_in_evidence_value_string_is_escaped() {
        // Direct coverage of S-101: the value field itself contains a pipe.
        let modules = vec![module("activity", 74)];
        let mut e = ev(
            "activity",
            "weird_value",
            "Weird value",
            "value contains a pipe",
            Verdict::Neutral,
        );
        e.value = serde_json::Value::String("a|b".to_string());
        e.threshold = None;
        let report = fixture_report(modules, vec![e]);
        let out = render(&report);
        assert!(
            out.contains(r"`a\|b`"),
            "expected escaped pipe inside the backticked value cell, got:\n{out}"
        );
    }

    #[test]
    fn write_creates_file_with_expected_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("octocat_Hello-World.md");
        let report = fixture_report(vec![module("activity", 74)], Vec::new());
        write(&report, &path).expect("write succeeds");
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.starts_with("# Trust Report — octocat/Hello-World"));
    }
}
