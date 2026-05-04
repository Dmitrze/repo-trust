//! CSV report writer (used by batch mode).
//!
//! One row per `TrustReport`. Suitable for spreadsheet tools, BI dashboards,
//! and downstream automation. Header row is written separately so batch mode
//! can emit one header followed by N rows.
//!
//! See `specs/reports-csv.md` for the contract; column order is fixed in
//! [`COLUMNS`] and any change must bump the spec.
//!
//! Quoting follows RFC 4180 conventions:
//! - fields containing `,`, `"`, `\n`, or `\r` are wrapped in double quotes;
//! - inner `"` is doubled (`""`);
//! - all other fields are emitted bare.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use time::format_description::well_known::Iso8601;

use crate::models::scores::Category;
use crate::models::{Confidence, Mode, ModuleResult, TrustReport};

/// The fixed column order for every CSV row. **Do not reorder.** New columns
/// must be appended (and the spec / scenarios updated).
///
/// 21 columns total: 9 report-level + 5 modules × 2 (score/confidence) + 2
/// top-concern columns.
pub const COLUMNS: &[&str] = &[
    "full_name",
    "url",
    "overall_score",
    "overall_confidence",
    "category",
    "mode",
    "scoring_version",
    "snapshot_at",
    "runtime_seconds",
    "stars_score",
    "stars_confidence",
    "activity_score",
    "activity_confidence",
    "maintainers_score",
    "maintainers_confidence",
    "adoption_score",
    "adoption_confidence",
    "security_score",
    "security_confidence",
    "top_concern_code",
    "top_concern_module",
];

/// Per-module column order, paired with the lookup key used against
/// `report.modules[*].module`. Drives the score+confidence pair section of
/// every row.
const MODULE_COLUMN_ORDER: &[&str] = &["stars", "activity", "maintainers", "adoption", "security"];

/// Write the CSV header row.
///
/// # Examples
///
/// ```
/// use repo_trust::reports::csv_report;
///
/// let mut buf = Vec::new();
/// csv_report::write_header(&mut buf).unwrap();
/// let s = String::from_utf8(buf).unwrap();
/// assert!(s.starts_with("full_name,url,overall_score,"));
/// assert!(s.ends_with('\n'));
/// ```
///
/// # Errors
///
/// Returns the underlying `std::io::Error` if the writer fails.
pub fn write_header(w: &mut impl Write) -> std::io::Result<()> {
    writeln!(w, "{}", COLUMNS.join(","))
}

/// Write one CSV data row for the given `TrustReport`.
///
/// Modules absent from `report.modules` (e.g. skipped via `--skip-modules`)
/// produce two empty cells in their slot. The row always has exactly
/// `COLUMNS.len()` columns.
///
/// # Errors
///
/// Returns the underlying `std::io::Error` if the writer fails, or an
/// I/O error if the snapshot timestamp cannot be formatted.
pub fn write_row(report: &TrustReport, w: &mut impl Write) -> std::io::Result<()> {
    let snapshot = report
        .snapshot_at
        .format(&Iso8601::DEFAULT)
        .map_err(std::io::Error::other)?;

    let (top_code, top_module) = report
        .top_concerns
        .first()
        .map_or((String::new(), String::new()), |ev| {
            (ev.code.clone(), ev.module.clone())
        });

    let mut cells: Vec<String> = Vec::with_capacity(COLUMNS.len());
    cells.push(report.repository.full_name.clone());
    cells.push(report.repository.url.clone());
    cells.push(report.overall_score.to_string());
    cells.push(confidence_str(report.overall_confidence).to_string());
    cells.push(category_str(report.category).to_string());
    cells.push(mode_str(report.mode).to_string());
    cells.push(report.scoring_version.clone());
    cells.push(snapshot);
    cells.push(format!("{:.1}", report.runtime_seconds));

    for module_name in MODULE_COLUMN_ORDER {
        match find_module(&report.modules, module_name) {
            Some(m) => {
                cells.push(m.score.to_string());
                cells.push(confidence_str(m.confidence).to_string());
            },
            None => {
                cells.push(String::new());
                cells.push(String::new());
            },
        }
    }

    cells.push(top_code);
    cells.push(top_module);

    debug_assert_eq!(cells.len(), COLUMNS.len(), "row column count drift");

    let escaped: Vec<String> = cells.iter().map(|c| escape_csv(c)).collect();
    writeln!(w, "{}", escaped.join(","))
}

/// Convenience: open `path`, write the header followed by one data row,
/// and close the file.
///
/// # Errors
///
/// Returns any I/O error opening or writing the file, or a formatting
/// error if the snapshot timestamp cannot be serialised.
pub fn write(report: &TrustReport, path: &Path) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("creating CSV report at {}", path.display()))?;
    write_header(&mut file).context("writing CSV header")?;
    write_row(report, &mut file).context("writing CSV row")?;
    Ok(())
}

/// RFC 4180-style quoting. Quotes any field containing `,`, `"`, `\n`, or
/// `\r`; doubles inner `"`. All other fields render bare.
fn escape_csv(field: &str) -> String {
    let needs_quote = field
        .as_bytes()
        .iter()
        .any(|&b| b == b',' || b == b'"' || b == b'\n' || b == b'\r');
    if !needs_quote {
        return field.to_string();
    }
    let mut out = String::with_capacity(field.len() + 2);
    out.push('"');
    for ch in field.chars() {
        if ch == '"' {
            out.push('"');
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

fn find_module<'a>(modules: &'a [ModuleResult], name: &str) -> Option<&'a ModuleResult> {
    modules.iter().find(|m| m.module == name)
}

fn confidence_str(c: Confidence) -> &'static str {
    match c {
        Confidence::Low => "Low",
        Confidence::Medium => "Medium",
        Confidence::High => "High",
    }
}

/// `Category` rendered with the same casing as its serde wire form
/// (PascalCase, see `models::scores::Category`).
fn category_str(c: Category) -> &'static str {
    match c {
        Category::Strong => "Strong",
        Category::Good => "Good",
        Category::Mixed => "Mixed",
        Category::Weak => "Weak",
        Category::HighRisk => "HighRisk",
    }
}

/// `Mode` rendered with the same casing as its serde wire form
/// (snake_case, see `models::reports::Mode`).
fn mode_str(m: Mode) -> &'static str {
    match m {
        Mode::Quick => "quick",
        Mode::Standard => "standard",
        Mode::Deep => "deep",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::evidence::Verdict;
    use crate::models::repository::RepositorySummary;
    use crate::models::scores::{Category, ModuleWeights};
    use crate::models::{EvidenceItem, Mode};
    use std::collections::BTreeMap;
    use time::macros::datetime;

    fn fixture_report() -> TrustReport {
        let make_module = |name: &str, score: u8, conf: Confidence| ModuleResult {
            module: name.to_string(),
            score,
            confidence: conf,
            sub_scores: BTreeMap::new(),
            sample_size: None,
            missing_data: Vec::new(),
        };

        let modules = vec![
            make_module("stars", 81, Confidence::High),
            make_module("activity", 72, Confidence::High),
            make_module("maintainers", 68, Confidence::High),
            make_module("adoption", 75, Confidence::Medium),
            make_module("security", 68, Confidence::High),
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
    fn write_header_emits_21_columns_in_documented_order() {
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.ends_with('\n'));
        let header = s.trim_end_matches('\n');
        let cols: Vec<&str> = header.split(',').collect();
        assert_eq!(cols.len(), 21, "header column count");
        assert_eq!(cols[0], "full_name");
        assert_eq!(cols[1], "url");
        assert_eq!(cols[8], "runtime_seconds");
        assert_eq!(cols[9], "stars_score");
        assert_eq!(cols[18], "security_confidence");
        assert_eq!(cols[19], "top_concern_code");
        assert_eq!(cols[20], "top_concern_module");
    }

    #[test]
    fn write_row_5_modules_round_trips_through_csv_crate() {
        let report = fixture_report();
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        write_row(&report, &mut buf).unwrap();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(buf.as_slice());
        let headers = rdr.headers().unwrap().clone();
        assert_eq!(headers.len(), 21);
        let rows: Vec<csv::StringRecord> = rdr.records().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 1, "exactly one data row");
        let row = &rows[0];
        assert_eq!(row.len(), 21, "data row column count");
        assert_eq!(&row[0], "acme/widget");
        assert_eq!(&row[1], "https://github.com/acme/widget");
        assert_eq!(&row[2], "73");
        assert_eq!(&row[3], "High");
        assert_eq!(&row[4], "Good");
        assert_eq!(&row[5], "standard");
        assert_eq!(&row[6], "1.0.0");
        assert_eq!(&row[8], "12.3");
        // stars=81/High, activity=72/High, maintainers=68/High,
        // adoption=75/Medium, security=68/High
        assert_eq!(&row[9], "81");
        assert_eq!(&row[10], "High");
        assert_eq!(&row[15], "75");
        assert_eq!(&row[16], "Medium");
        assert_eq!(&row[19], "no_packages");
        assert_eq!(&row[20], "adoption");
    }

    #[test]
    fn comma_in_field_is_quoted() {
        let mut report = fixture_report();
        report.repository.full_name = "weird,name/repo".to_string();
        let mut buf = Vec::new();
        write_row(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.starts_with(r#""weird,name/repo","#),
            "comma field must be wrapped in quotes; got: {s}"
        );
        // Must still round-trip cleanly through the csv crate.
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());
        let row = rdr.records().next().unwrap().unwrap();
        assert_eq!(&row[0], "weird,name/repo");
        assert_eq!(row.len(), 21);
    }

    #[test]
    fn double_quote_in_field_is_escaped_by_doubling() {
        let mut report = fixture_report();
        report.top_concerns[0].code = r#"quote"in"code"#.to_string();
        let mut buf = Vec::new();
        write_row(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains(r#""quote""in""code""#),
            "inner quote should be doubled per RFC 4180; got: {s}"
        );
        // Round-trip restores the original string.
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());
        let row = rdr.records().next().unwrap().unwrap();
        assert_eq!(&row[19], r#"quote"in"code"#);
    }

    #[test]
    fn newline_in_field_is_quoted() {
        let mut report = fixture_report();
        report.top_concerns[0].code = "line1\nline2".to_string();
        let mut buf = Vec::new();
        write_row(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"line1\nline2\""));
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());
        let row = rdr.records().next().unwrap().unwrap();
        assert_eq!(&row[19], "line1\nline2");
        assert_eq!(row.len(), 21);
    }

    #[test]
    fn missing_module_emits_empty_cells_and_keeps_21_columns() {
        let mut report = fixture_report();
        // Drop everything except activity — simulates `--modules activity`.
        report.modules.retain(|m| m.module == "activity");

        let mut buf = Vec::new();
        write_row(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());
        let row = rdr.records().next().unwrap().unwrap();
        assert_eq!(row.len(), 21, "row still has 21 columns");

        // stars (cols 9-10), maintainers (13-14), adoption (15-16), security
        // (17-18) all empty; activity (11-12) populated.
        assert_eq!(&row[9], "");
        assert_eq!(&row[10], "");
        assert_eq!(&row[11], "72");
        assert_eq!(&row[12], "High");
        assert_eq!(&row[13], "");
        assert_eq!(&row[14], "");
        assert_eq!(&row[15], "");
        assert_eq!(&row[16], "");
        assert_eq!(&row[17], "");
        assert_eq!(&row[18], "");
    }

    #[test]
    fn no_top_concerns_emits_empty_concern_cells() {
        let mut report = fixture_report();
        report.top_concerns.clear();

        let mut buf = Vec::new();
        write_row(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());
        let row = rdr.records().next().unwrap().unwrap();
        assert_eq!(row.len(), 21);
        assert_eq!(&row[19], "");
        assert_eq!(&row[20], "");
    }

    #[test]
    fn write_creates_file_with_header_plus_one_row() {
        let report = fixture_report();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("acme_widget.csv");
        write(&report, &path).unwrap();
        let s = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("full_name,url,overall_score,"));
        assert!(lines[1].starts_with("acme/widget,https://github.com/acme/widget,73,High,"));
    }

    #[test]
    fn escape_csv_passes_through_simple_strings() {
        assert_eq!(escape_csv("hello"), "hello");
        assert_eq!(escape_csv(""), "");
        assert_eq!(escape_csv("acme/widget"), "acme/widget");
    }

    #[test]
    fn escape_csv_handles_carriage_return() {
        assert_eq!(escape_csv("a\rb"), "\"a\rb\"");
    }
}
