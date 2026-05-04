---
feature: reports-csv
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/reports-csv.md"]
related_runbooks: []
related_docs: []
---

# CSV report writer

> One row per repo. Suitable for batch-mode runs that pipe into spreadsheet tools, BI dashboards, or downstream automation. Header row required; columns documented in this spec and the produced output.

---

## 1. Goal

`reports::csv_report::write(report, writer)` appends one CSV row representing a single `TrustReport`. `write_header(writer)` emits the column header. Used by `cli::scan::execute` (single row per scan) and `cli::batch::execute` (multi-row append).

We know it works when: a 5-module report serialised via this writer round-trips through `csv` parsing without quoting issues, and the column header matches the documented order exactly.

---

## 2. Non-functional requirements

- **No new runtime crates:** plain `writeln!` formatting with explicit double-quoting + comma escaping. (We considered the `csv` crate; rejected — adds 5+ transitive deps for a one-row writer.)
- **RFC 4180-ish:** quote any field containing comma, quote, or newline; double-up internal quotes.
- **Determinism:** byte-identical output for the same `TrustReport` modulo `snapshot_at` + `runtime_seconds`.
- **Append-safe:** `write_header` is separate from `write_row` so batch mode can emit one header + N rows.

---

## 3. Boundaries

### In scope (Day 4)
- `src/reports/csv_report.rs::write_header(w: &mut impl Write)` — writes the documented header row.
- `src/reports/csv_report.rs::write_row(report: &TrustReport, w: &mut impl Write)` — writes one row per `TrustReport`.
- `src/reports/csv_report.rs::write(report: &TrustReport, path: &Path)` — convenience: header + one row to file.
- Column order (fixed, documented):
  ```
  full_name,
  url,
  overall_score,
  overall_confidence,
  category,
  mode,
  scoring_version,
  snapshot_at,
  runtime_seconds,
  stars_score, stars_confidence,
  activity_score, activity_confidence,
  maintainers_score, maintainers_confidence,
  adoption_score, adoption_confidence,
  security_score, security_confidence,
  top_concern_code,
  top_concern_module
  ```
- Modules whose result is missing get empty cells (e.g. when the user passed `--skip-modules stars`).
- Top-concern code/module = first item from `report.top_concerns` (or empty if no concerns).
- ≥4 unit tests + 1 insta snapshot.

### Out of scope
- TSV variant.
- Multi-row-per-repo (one row per evidence item) export — separate spec if requested.
- BOM / encoding flags.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

```
$ repo-trust scan acme/widget --format csv --output ./reports
$ cat ./reports/acme_widget.csv
full_name,url,overall_score,overall_confidence,category,mode,scoring_version,snapshot_at,runtime_seconds,stars_score,stars_confidence,activity_score,activity_confidence,maintainers_score,maintainers_confidence,adoption_score,adoption_confidence,security_score,security_confidence,top_concern_code,top_concern_module
acme/widget,https://github.com/acme/widget,73,High,Good,standard,1.0.0,2026-05-04T10:23:45Z,12.3,81,High,72,High,68,High,75,Medium,68,High,no_packages,adoption
```

For batch mode (Day 5):
```
$ repo-trust batch repos.txt --format csv --output ./reports
$ cat ./reports/batch.csv  # one header + N rows
```

---

## 6. Architecture sketch

```
write(report, path):
  open file
  write_header(file)
  write_row(report, file)

write_row(report, w):
  let cols = build_columns(report);    // Vec<String>
  let escaped = cols.into_iter().map(escape_csv);
  writeln!(w, "{}", escaped.join(","));

escape_csv(s): if needs quote -> quote + escape + close
```

---

## 7. Closed loop

- **Goal metric:** insta snapshot match; round-trip through `csv::Reader` in a unit test parses every column correctly.
- **Where it lives:** CI; MemPalace `ops/distribution`.
- **Read by:** batch-mode users + downstream automation.
- **Improvement path:** if downstream consumers ask for additional columns, add at the END of the row (header version bump).

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/reports-csv.md` lists ≥4 scenarios.
- [ ] `src/reports/csv_report.rs` implements `write_header` + `write_row` + `write`.
- [ ] No new runtime crates (the `csv` crate is allowed in dev-dependencies for round-trip tests if useful).
- [ ] Column order matches this spec exactly.
- [ ] ≥4 unit tests covering: simple row, quote escaping, comma in field, newline in field, missing-module cells.
- [ ] 1 insta snapshot test against a fixed `TrustReport`.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-04 — pull the `csv` crate? — No for runtime, optional for dev. Plain string formatting keeps the dependency surface minimal and the column order completely under our control.

---

## 11. References

- RFC 4180 (CSV).
- `docs/architecture.md` §5 — TrustReport shape.
