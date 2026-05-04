---
feature: reports-markdown
status: accepted
spec: ../../specs/reports-markdown.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Markdown report writer — Scenarios

Link: [`specs/reports-markdown.md`](../../specs/reports-markdown.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | 5-module report; round-trip via pulldown-cmark |
| Edge cases | 2 | escaped pipes in evidence value; empty top_concerns |
| Snapshot | 1 | insta golden against the fixed inactive baseline |

---

## Happy path

### S-001: 5-module report produces complete markdown

**Given** a 5-module `TrustReport`
**When** `markdown_report::write(&report, path)` is called
**Then** the file contains: H1 header with `Trust Report — {full_name}`, summary table with 7 rows, one `## {Module Name}` section per module, sub-scores table, evidence table, "Top Strengths" + "Top Concerns" sections, "Methodology" footer.

### S-002: output round-trips through pulldown-cmark without warnings

**Given** the produced markdown
**When** parsed via `pulldown_cmark::Parser`
**Then** parsing produces zero warnings (well-formed GFM).

---

## Edge cases

### S-101: pipe character in evidence value is escaped

**Given** an evidence item with `value: serde_json::Value::String("a|b".into())`
**When** the evidence table renders
**Then** the `|` is escaped as `\|` so it doesn't break the table column boundary.

### S-102: empty top_concerns omits the section entirely

**Given** a report with `top_concerns: vec![]`
**When** the writer runs
**Then** the markdown does NOT contain a `## Top Concerns` header.

---

## Snapshot

### S-401: insta snapshot against the inactive fixture

**Given** a known-good 5-module `TrustReport` fixture
**When** `markdown_report::write` is invoked into a buffer
**Then** the output matches the committed insta snapshot.
