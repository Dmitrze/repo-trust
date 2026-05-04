---
feature: reports-csv
status: accepted
spec: ../../specs/reports-csv.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# CSV report writer — Scenarios

Link: [`specs/reports-csv.md`](../../specs/reports-csv.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | header + one row; round-trip parse via csv crate (dev-dep) |
| Edge cases | 3 | comma in field; quote in field; missing module → empty cells |
| Snapshot | 1 | insta golden |

---

## Happy path

### S-001: write_header + write_row produces parseable CSV

**Given** a 5-module `TrustReport`
**When** `write_header(buf)` followed by `write_row(&report, buf)` is called
**Then** `buf` contains exactly 2 newline-terminated lines; both parse via `csv::Reader::from_reader(buf.as_bytes())` without error; the header row matches the documented column order; the data row has the same number of columns.

### S-002: round-trip preserves all values

**Given** the same row
**When** parsed via `csv::Reader` and re-serialized
**Then** the round-trip output matches the original byte-for-byte (modulo trailing newline conventions).

---

## Edge cases

### S-101: comma in field is properly quoted

**Given** an evidence rationale containing a comma (used as `top_concern_code` is unlikely but the URL or full_name might in pathological cases)
**When** the row is written
**Then** the offending field is wrapped in double quotes; CSV parsing succeeds.

### S-102: double quote in field is escaped by doubling

**Given** a field value containing `"` (e.g. an unusual top-concern code)
**When** the row is written
**Then** the field is quoted and the inner `"` is doubled per RFC 4180.

### S-103: missing module → empty cells

**Given** a report from `--modules activity` (only 1 module result present)
**When** the row is written
**Then** `activity_score` and `activity_confidence` cells contain values; the other 8 module cells (stars/maintainers/adoption/security × score+confidence) are empty strings; the row still has 21 columns total.

---

## Snapshot

### S-401: insta snapshot of header + one full row

**Given** a fixed `TrustReport` fixture
**When** `write_header` + `write_row` runs into a buffer
**Then** the buffer matches the committed insta snapshot.
