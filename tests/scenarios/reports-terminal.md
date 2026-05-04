---
feature: reports-terminal
status: accepted
spec: ../../specs/reports-terminal.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Terminal report writer — Scenarios

Link: [`specs/reports-terminal.md`](../../specs/reports-terminal.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | full 5-module report with color; piped no-color |
| Edge cases | 2 | empty caveats; missing module data |
| Snapshot | 1 | insta golden against a fixed report |

---

## Happy path

### S-001: full report renders header + table + sections

**Given** a 5-module `TrustReport` with score 73, category Good, confidence High
**When** `terminal::write(&report, &mut buf, color=true)` is called
**Then** output contains: header line with full_name, score line with category + confidence, module table with 5 rows, "Top strengths" section with ≤3 items, "Top concerns" section with ≤3 items.

### S-002: piped output suppresses ANSI codes

**Given** the same report
**When** `terminal::write(&report, &mut buf, color=false)` is called
**Then** output contains zero `\x1b[` ANSI escape sequences.

---

## Edge cases

### S-101: empty caveats omits the caveats section entirely

**Given** a report with `caveats: vec![]`
**When** `terminal::write` is called
**Then** the output does NOT contain a "Caveats" header.

### S-102: report with one module renders without panic

**Given** a single-module report (e.g. `--modules activity`)
**When** `terminal::write` is called
**Then** the table has exactly one row; no panic.

---

## Snapshot

### S-401: insta snapshot against the fixed octocat/Hello-World inactive baseline

**Given** a known-good `TrustReport` fixture (5-module inactive baseline)
**When** `terminal::write(report, &mut buf, color=false)` is called
**Then** `buf` matches the committed insta snapshot byte-for-byte.
