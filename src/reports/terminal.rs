//! Terminal output — the colored summary shown by default.
//!
//! Output shape (≤ 30 lines, ≤ 80 cols target):
//!
//! ```text
//! {full_name} — {url}
//! Score: NN  Category: X  Confidence: Y
//! Mode: standard | Scoring: 1.0.0 | Snapshot: <iso8601>
//!
//! ┌────────────────────┬───────┬────────────┬──────────────────┬──────────────┐
//! │ Module             │ Score │ Confidence │ Top Sub-Score    │ Missing Data │
//! ├────────────────────┼───────┼────────────┼──────────────────┼──────────────┤
//! │ Star Authenticity  │   45  │ Medium     │ low_share=85     │              │
//! │ ...                                                                        │
//! └────────────────────┴───────┴────────────┴──────────────────┴──────────────┘
//!
//! Top strengths
//!   ✔ Positive [security] documentation_presence — ...
//!
//! Top concerns
//!   ✖ HighRisk [activity] no_commits_in_window — ...
//!
//! Caveats
//!   - osv_deferred_to_phase_3
//! ```
//!
//! Section titles ("Top strengths" / "Top concerns") are colored green / red
//! when `color = true`. Per-item glyph and verdict label are colored by the
//! item's `Verdict`: Positive (green ✔), Concerning (yellow ✖),
//! HighRisk (red ✖), Neutral (gray •). Module names in the table use
//! display-friendly forms ("Star Authenticity" rather than "stars") that
//! mirror the README banner.
//!
//! When `color = false`, no ANSI escape sequences are emitted; the glyphs
//! become `[+]` / `[!]` / `[-]` / `[~]` for Positive / Concerning / HighRisk /
//! Neutral respectively, for plain-ASCII friendliness. The output is
//! verified by the snapshot test in `tests/reports_terminal_snapshot.rs`.
//!
//! `comfy_table` uses `ContentArrangement::Dynamic` and may wrap long cell
//! content across multiple rows when the table can't fit the whole row.
//! That's intentional — narrow terminals still get a readable table — but
//! it means string-equality assertions in tests should use the
//! `normalize_table_text` helper to fold those wraps.

use std::io::Write;

use comfy_table::{
    presets::UTF8_BORDERS_ONLY, Cell, Color as TableColor, ContentArrangement, Table,
};
use console::{Color, Style};
use time::format_description::well_known::Iso8601;

use crate::models::{
    evidence::Verdict, Category, Confidence, EvidenceItem, ModuleResult, TrustReport,
};

/// Write a human-friendly summary of the trust report to `w`.
///
/// `color = true` emits ANSI escape sequences for category-colored
/// score cells, dim/normal/bold confidence styling, green/red section
/// titles, and per-verdict glyph + label colors. `color = false` emits
/// zero ANSI sequences — safe for piping into `cat`, files, or CI logs.
///
/// # Errors
///
/// Returns any I/O error from the underlying writer.
pub fn write<W: Write>(report: &TrustReport, w: &mut W, color: bool) -> std::io::Result<()> {
    write_header(report, w, color)?;
    writeln!(w)?;
    write_module_table(&report.modules, w, color)?;
    writeln!(w)?;
    write_evidence_section("Top strengths", &report.top_strengths, true, w, color)?;
    writeln!(w)?;
    write_evidence_section("Top concerns", &report.top_concerns, false, w, color)?;
    if !report.caveats.is_empty() {
        writeln!(w)?;
        write_caveats_section(&report.caveats, w)?;
    }
    Ok(())
}

// ─── Header ────────────────────────────────────────────────────────────

fn write_header<W: Write>(report: &TrustReport, w: &mut W, color: bool) -> std::io::Result<()> {
    writeln!(
        w,
        "{} — {}",
        report.repository.full_name, report.repository.url
    )?;

    let cat_label = paint(
        category_label(report.category).to_string(),
        category_console_color(report.category),
        color,
    );
    let conf_label = confidence_styled(report.overall_confidence, color);
    writeln!(
        w,
        "Score: {score:>2}  Category: {cat}  Confidence: {conf}",
        score = report.overall_score,
        cat = cat_label,
        conf = conf_label,
    )?;

    let snapshot = report
        .snapshot_at
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".to_string());
    writeln!(
        w,
        "Mode: {mode} | Scoring: {scoring} | Snapshot: {snapshot}",
        mode = mode_label(report.mode),
        scoring = report.scoring_version,
    )?;

    Ok(())
}

// ─── Module table ──────────────────────────────────────────────────────

fn write_module_table<W: Write>(
    modules: &[ModuleResult],
    w: &mut W,
    color: bool,
) -> std::io::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_BORDERS_ONLY)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Module",
            "Score",
            "Confidence",
            "Top Sub-Score",
            "Missing Data",
        ]);

    for m in modules {
        let score_cell = if color {
            Cell::new(format!("{:>3}", m.score))
                .fg(category_table_color(Category::from_score(m.score)))
        } else {
            Cell::new(format!("{:>3}", m.score))
        };
        let conf_cell = Cell::new(confidence_label(m.confidence));
        let top_sub = top_sub_score_str(m).unwrap_or_default();
        let missing = m.missing_data.join(", ");

        table.add_row(vec![
            Cell::new(module_display_name(&m.module)),
            score_cell,
            conf_cell,
            Cell::new(top_sub),
            Cell::new(missing),
        ]);
    }

    writeln!(w, "{table}")?;
    Ok(())
}

/// Pick the highest-valued sub-score entry, formatted as `key=value`.
/// Ties broken by lexicographic key order (sub_scores is `BTreeMap`,
/// so iteration is already stable).
fn top_sub_score_str(m: &ModuleResult) -> Option<String> {
    let (key, value) = m
        .sub_scores
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))?;
    Some(format!("{key}={value}"))
}

// ─── Evidence sections ─────────────────────────────────────────────────

fn write_evidence_section<W: Write>(
    title: &str,
    items: &[EvidenceItem],
    is_strengths_section: bool,
    w: &mut W,
    color: bool,
) -> std::io::Result<()> {
    // Section title is colored green for strengths, red for concerns,
    // matching the README terminal mockup (`assets/terminal.svg`).
    let title_color = if is_strengths_section {
        Color::Green
    } else {
        Color::Red
    };
    writeln!(w, "{}", paint(title.to_string(), title_color, color))?;

    if items.is_empty() {
        writeln!(w, "  (none)")?;
        return Ok(());
    }

    for item in items {
        // Per-item styling driven by the verdict — Color::Color256(244)
        // is a mid-grey for Neutral so it reads as informational rather
        // than green/yellow/red.
        let (glyph_char, fallback, verdict_color, verdict_text) = match item.verdict {
            Verdict::Positive => ("✔", "[+]", Color::Green, "Positive"),
            Verdict::Concerning => ("✖", "[!]", Color::Yellow, "Concerning"),
            Verdict::HighRisk => ("✖", "[-]", Color::Red, "HighRisk"),
            Verdict::Neutral => ("•", "[~]", Color::Color256(244), "Neutral"),
        };
        let glyph = if color {
            paint(glyph_char.to_string(), verdict_color, true)
        } else {
            fallback.to_string()
        };
        let label = paint(verdict_text.to_string(), verdict_color, color);

        writeln!(
            w,
            "  {glyph} {label} [{module}] {code} — {rationale}",
            module = item.module,
            code = item.code,
            rationale = item.rationale,
        )?;
    }
    Ok(())
}

// ─── Caveats ───────────────────────────────────────────────────────────

fn write_caveats_section<W: Write>(caveats: &[String], w: &mut W) -> std::io::Result<()> {
    writeln!(w, "Caveats")?;
    for c in caveats {
        writeln!(w, "  - {c}")?;
    }
    Ok(())
}

// ─── Helpers: labels and colors ────────────────────────────────────────

/// Map a module ID to its display-friendly name shown in the README banner
/// and the terminal table. Unknown IDs pass through unchanged so future
/// modules render gracefully without code changes.
fn module_display_name(id: &str) -> &str {
    match id {
        "stars" => "Star Authenticity",
        "activity" => "Activity Health",
        "maintainers" => "Maintainer Health",
        "adoption" => "Adoption Signals",
        "security" => "Security & Readiness",
        _ => id,
    }
}

const fn category_label(c: Category) -> &'static str {
    match c {
        Category::Strong => "Strong",
        Category::Good => "Good",
        Category::Mixed => "Mixed",
        Category::Weak => "Weak",
        Category::HighRisk => "HighRisk",
    }
}

const fn confidence_label(c: Confidence) -> &'static str {
    match c {
        Confidence::Low => "Low",
        Confidence::Medium => "Medium",
        Confidence::High => "High",
    }
}

fn mode_label(m: crate::models::Mode) -> &'static str {
    match m {
        crate::models::Mode::Quick => "quick",
        crate::models::Mode::Standard => "standard",
        crate::models::Mode::Deep => "deep",
    }
}

/// Map category to the `console` palette per spec §2.
const fn category_console_color(c: Category) -> Color {
    match c {
        Category::Strong => Color::Green,
        Category::Good => Color::Cyan,
        Category::Mixed => Color::Yellow,
        // Orange (xterm 214). `console` lacks a native orange, so we use
        // the 256-color palette per spec §2.
        Category::Weak => Color::Color256(214),
        Category::HighRisk => Color::Red,
    }
}

/// Map category to the `comfy_table` palette. Mirrors
/// [`category_console_color`] but uses `comfy_table`'s `Color` enum
/// because the table renders independently of `console`.
const fn category_table_color(c: Category) -> TableColor {
    match c {
        Category::Strong => TableColor::Green,
        Category::Good => TableColor::Cyan,
        Category::Mixed => TableColor::Yellow,
        Category::Weak => TableColor::AnsiValue(214),
        Category::HighRisk => TableColor::Red,
    }
}

/// Apply a foreground color when `color = true`, otherwise return
/// the plain string (no ANSI). Uses `force_styling` so global terminal
/// detection cannot leak codes when the caller has asked for plain text.
fn paint(s: String, fg: Color, color: bool) -> String {
    if color {
        Style::new()
            .force_styling(true)
            .fg(fg)
            .apply_to(s)
            .to_string()
    } else {
        s
    }
}

/// Confidence styling: Low=dim, Medium=normal, High=bold per spec §2.
fn confidence_styled(c: Confidence, color: bool) -> String {
    let label = confidence_label(c).to_string();
    if !color {
        return label;
    }
    let style = Style::new().force_styling(true);
    match c {
        Confidence::Low => style.dim().apply_to(label).to_string(),
        Confidence::Medium => label,
        Confidence::High => style.bold().apply_to(label).to_string(),
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        evidence::Verdict, scores::Confidence, Mode, ModuleWeights, RepositorySummary,
    };
    use std::collections::BTreeMap;
    use time::macros::datetime;

    /// Fold comfy_table cell wrapping back into single-line strings so
    /// integration assertions don't break when long content wraps.
    ///
    /// `comfy_table` with `ContentArrangement::Dynamic` may split a long
    /// cell value across multiple visual rows — e.g. `low_activity_share=85`
    /// can render as
    /// ```text
    /// │ ... low_activity_share                 │
    /// │     =85                                │
    /// ```
    /// All of our integration tests in this module care about *what* gets
    /// rendered, not the exact wrap column; this helper collapses runs of
    /// whitespace (including newlines and table-internal padding) into a
    /// single space and folds the leading-space-before-`=` pattern that
    /// `comfy_table` produces when wrapping `key=value` cells.
    fn normalize_table_text(s: &str) -> String {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace(" =", "=")
    }

    fn fixture_summary() -> RepositorySummary {
        RepositorySummary {
            full_name: "octocat/Hello-World".to_string(),
            url: "https://github.com/octocat/Hello-World".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 1234,
            snapshot_at: datetime!(2026-05-04 10:23:45 UTC),
        }
    }

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

    fn five_module_report() -> TrustReport {
        TrustReport {
            schema_version: "1.0.0".to_string(),
            repository: fixture_summary(),
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
                    &["no_packages", "no_dependents"],
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
                    "watcher/star ratio = 0.0162; ecosystem-adjusted threshold ≥ 0.0050",
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

    fn render(report: &TrustReport, color: bool) -> String {
        let mut buf = Vec::new();
        write(report, &mut buf, color).expect("write succeeds");
        String::from_utf8(buf).expect("utf-8 output")
    }

    #[test]
    fn header_line_contains_full_name_and_url() {
        let report = five_module_report();
        let out = render(&report, false);
        let first = out.lines().next().expect("at least one line");
        assert!(
            first.contains("octocat/Hello-World"),
            "header line: {first}"
        );
        assert!(
            first.contains("https://github.com/octocat/Hello-World"),
            "header line: {first}"
        );
    }

    #[test]
    fn module_table_contains_all_module_display_names_and_scores() {
        let report = five_module_report();
        let out = render(&report, false);
        // Long display names ("Security & Readiness") may wrap across
        // multiple table rows under ContentArrangement::Dynamic — fold
        // the wrap before comparing.
        let normalized = normalize_table_text(&out);
        for m in &report.modules {
            let display = module_display_name(&m.module);
            assert!(
                normalized.contains(display),
                "module display name '{display}' missing in normalized output:\n{normalized}"
            );
            // Score is right-aligned to width 3.
            let score_str = format!("{:>3}", m.score);
            assert!(
                out.contains(&score_str),
                "score {} missing in output:\n{out}",
                m.score
            );
        }
    }

    #[test]
    fn module_id_appears_in_evidence_brackets() {
        // The evidence section retains the short module ID in [brackets]
        // for compactness; the table uses display names.
        let report = five_module_report();
        let out = render(&report, false);
        for id in ["stars", "activity", "maintainers", "adoption", "security"] {
            assert!(
                out.contains(&format!("[{id}]")),
                "[{id}] missing in evidence section:\n{out}"
            );
        }
    }

    #[test]
    fn top_sub_score_picks_higher_value_over_lower_unit() {
        // Direct unit test for top_sub_score_str — exercises the picker
        // logic without depending on the table renderer.
        let m = module(
            "stars",
            45,
            Confidence::Medium,
            &[("low_activity_share", 85), ("watcher_to_star_ratio", 30)],
            &[],
        );
        assert_eq!(
            top_sub_score_str(&m).as_deref(),
            Some("low_activity_share=85")
        );
    }

    #[test]
    fn top_sub_score_picks_highest_entry() {
        let report = five_module_report();
        let out = render(&report, false);
        let normalized = normalize_table_text(&out);
        // stars sub-scores: low_activity_share=85 (chosen) vs
        // watcher_to_star_ratio=30 (not chosen).
        assert!(
            normalized.contains("low_activity_share=85"),
            "expected highest sub-score 'low_activity_share=85' in normalized output:\n{normalized}"
        );
        // The lower-valued sub-score must not appear as the chosen top.
        // (`watcher_to_star_ratio` does appear as an evidence code in
        // the rationale below, but never as `watcher_to_star_ratio=30`.)
        assert!(
            !normalized.contains("watcher_to_star_ratio=30"),
            "lower sub-score 'watcher_to_star_ratio=30' should not appear as the chosen top:\n{normalized}"
        );
    }

    #[test]
    fn missing_data_cell_joins_entries_with_comma_space() {
        let report = five_module_report();
        let out = render(&report, false);
        let normalized = normalize_table_text(&out);
        assert!(
            normalized.contains("no_packages, no_dependents"),
            "expected joined missing_data 'no_packages, no_dependents' in normalized output:\n{normalized}"
        );
    }

    #[test]
    fn color_false_emits_no_ansi_escape_sequences() {
        let report = five_module_report();
        let out = render(&report, false);
        assert!(
            !out.contains("\x1b["),
            "expected zero ANSI escape sequences in plain output, got:\n{out}"
        );
    }

    #[test]
    fn empty_caveats_omits_caveats_header() {
        let mut report = five_module_report();
        report.caveats.clear();
        let out = render(&report, false);
        assert!(
            !out.contains("Caveats"),
            "Caveats header should be omitted when caveats is empty:\n{out}"
        );
    }

    #[test]
    fn single_module_report_renders_without_panic() {
        let mut report = five_module_report();
        report.modules.truncate(1);
        let out = render(&report, false);
        let normalized = normalize_table_text(&out);
        // After truncation only the "stars" module remains in the table.
        assert!(
            normalized.contains("Star Authenticity"),
            "single-module table should contain the module's display name:\n{normalized}"
        );
        assert!(out.contains("octocat/Hello-World"));
    }

    #[test]
    fn color_true_emits_ansi_escape_sequences() {
        let report = five_module_report();
        let out = render(&report, true);
        assert!(
            out.contains("\x1b["),
            "expected at least one ANSI escape sequence when color=true"
        );
    }

    #[test]
    fn confidence_low_styled_dim_when_color() {
        let styled = confidence_styled(Confidence::Low, true);
        assert!(
            styled.contains("\x1b[2m") || styled.contains("\x1b[2;"),
            "Low should be dim: {styled:?}"
        );
    }

    #[test]
    fn confidence_high_styled_bold_when_color() {
        let styled = confidence_styled(Confidence::High, true);
        assert!(
            styled.contains("\x1b[1m") || styled.contains("\x1b[1;"),
            "High should be bold: {styled:?}"
        );
    }

    #[test]
    fn top_sub_score_none_when_empty() {
        let m = module("empty", 0, Confidence::Low, &[], &[]);
        assert!(top_sub_score_str(&m).is_none());
    }

    #[test]
    fn module_display_name_maps_known_ids() {
        assert_eq!(module_display_name("stars"), "Star Authenticity");
        assert_eq!(module_display_name("activity"), "Activity Health");
        assert_eq!(module_display_name("maintainers"), "Maintainer Health");
        assert_eq!(module_display_name("adoption"), "Adoption Signals");
        assert_eq!(module_display_name("security"), "Security & Readiness");
    }

    #[test]
    fn module_display_name_passes_unknown_ids_through() {
        // Future modules (e.g. "governance") should render gracefully
        // without code changes — they appear by their ID until added here.
        assert_eq!(module_display_name("governance"), "governance");
        assert_eq!(module_display_name(""), "");
    }

    #[test]
    fn evidence_includes_verdict_label_in_plain_mode() {
        let report = five_module_report();
        let out = render(&report, false);
        // Each evidence line carries the verdict label after the glyph.
        assert!(
            out.contains("[+] Positive"),
            "expected positive verdict label '[+] Positive' in output:\n{out}"
        );
        assert!(
            out.contains("[-] HighRisk"),
            "expected high-risk verdict label '[-] HighRisk' in output:\n{out}"
        );
        assert!(
            out.contains("[!] Concerning"),
            "expected concerning verdict label '[!] Concerning' in output:\n{out}"
        );
    }

    #[test]
    fn evidence_renders_neutral_verdict_with_tilde_glyph() {
        // Defensive — the snapshot fixture currently has no Neutral items,
        // but the match arm must cover the variant or compilation fails.
        // Use a tiny ad-hoc report rather than mutating the shared fixture.
        let mut report = five_module_report();
        report.top_strengths = vec![ev(
            "stars",
            "watcher_to_star_ratio",
            Verdict::Neutral,
            "ratio is within the expected band — neither suspicious nor strong.",
        )];
        let out = render(&report, false);
        assert!(
            out.contains("[~] Neutral"),
            "expected neutral verdict label '[~] Neutral' in output:\n{out}"
        );
    }

    #[test]
    fn normalize_table_text_collapses_wrapped_cells() {
        // Self-test for the helper so its behaviour is documented and
        // regressions are caught locally rather than via the consumers.
        let wrapped = "│ low_activity_share                 │\n│ =85                                │";
        assert!(normalize_table_text(wrapped).contains("low_activity_share=85"));
        let two_word = "│ Security &            55 │\n│ Readiness                │";
        assert!(normalize_table_text(two_word).contains("Security & Readiness"));
        let comma_wrap = "│ no_packages,  │\n│ no_dependents │";
        assert!(normalize_table_text(comma_wrap).contains("no_packages, no_dependents"));
    }
}
