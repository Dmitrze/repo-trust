---
feature: reports-terminal
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/reports-terminal.md"]
related_runbooks: []
related_docs: ["docs/architecture.md#5-data-model"]
---

# Terminal report writer

> Default user-facing output. Colored, table-driven, fits in a single 80-col terminal screen for the typical report. Always shown unless `--quiet`. Never blocks a non-terminal stdout (when piped, ANSI codes are suppressed via `console::Term::stdout().is_term()`).

---

## 1. Goal

`reports::terminal::write(report, writer, color: bool)` prints a human-friendly summary of a `TrustReport` to the writer. Format: header (repo + score + category + confidence), per-module table, top-3 strengths + top-3 concerns, caveats. Color-coded by category band when `color = true`.

We know it works when: `cargo run -- scan octocat/Hello-World --mode standard` produces a colorized terminal block ≤30 lines that a developer can read in one screen, and `cargo run ... | cat` produces the same content with ANSI codes suppressed.

---

## 2. Non-functional requirements

- **No allocations on the hot path** beyond the table builder + format strings.
- **Color discipline:** Strong=green, Good=cyan, Mixed=yellow, Weak=orange (`console::Color::Color256(214)`), HighRisk=red. Confidence Low=dim, Medium=normal, High=bold.
- **Width:** target ≤80 cols; comfy-table preset `UTF8_BORDERS_ONLY` keeps it readable on narrow terminals.
- **Determinism:** for the same `TrustReport` + same `color` flag, `write()` produces byte-identical output.

---

## 3. Boundaries

### In scope (Day 4)
- `src/reports/terminal.rs::write(report: &TrustReport, w: &mut impl Write, color: bool) -> std::io::Result<()>`.
- Header block (3 lines): `repo full_name`, `Score: NN  Category: X  Confidence: Y`, `Mode: standard | Scoring: 1.0.0 | Snapshot: <iso8601>`.
- Module table via `comfy_table` with columns: `Module`, `Score`, `Confidence`, `Top Sub-Score`, `Missing Data`.
- "Top strengths" + "Top concerns" sections rendering 3 evidence items each (sourced from `report.top_strengths` / `top_concerns` already populated by the aggregator).
- Caveats section (only when `report.caveats` non-empty).
- ≥6 unit tests + 1 insta snapshot.

### Out of scope
- Interactive UI (no TUI navigation; pure non-interactive print).
- Sparklines / inline charts (Day 5+).
- Theming via config — Day 5 polish if requested.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

```
$ repo-trust scan octocat/Hello-World --mode standard
octocat/Hello-World — https://github.com/octocat/Hello-World
Score: 12  Category: HighRisk  Confidence: High
Mode: standard | Scoring: 1.0.0 | Snapshot: 2026-05-04T10:23:45Z

┌──────────────┬───────┬────────────┬────────────────────────────┬───────────────┐
│ Module       │ Score │ Confidence │ Top Sub-Score              │ Missing Data  │
├──────────────┼───────┼────────────┼────────────────────────────┼───────────────┤
│ stars        │   45  │ Medium     │ low_activity_share=85       │               │
│ activity     │    8  │ High       │ commits_last_90d=0          │ no_releases   │
│ maintainers  │   25  │ Low        │ bus_factor_proxy=25         │               │
│ adoption     │   30  │ Medium     │ documentation_maturity=60   │ no_packages   │
│ security     │   55  │ Medium     │ documentation_presence=80   │               │
└──────────────┴───────┴────────────┴────────────────────────────┴───────────────┘

Top strengths
  ✔ [security] documentation_presence — 4/5 expected docs present (LICENSE/CONTRIBUTING/CODE_OF_CONDUCT/CODEOWNERS)
  ✔ [stars] watcher_to_star_ratio — watcher/star ratio = 0.0162; ecosystem-adjusted threshold ≥ 0.0050

Top concerns
  ✖ [activity] no_commits_in_window — The 18-month commit window contained zero commits.
  ✖ [maintainers] solo_maintainer — Only one human author committed in the last year.
  ✖ [adoption] no_readme — README endpoint returned 404.

Caveats
  - osv_deferred_to_phase_3
  - recency_biased_sample
```

---

## 6. Architecture sketch

```
write(report, writer, color):
  1. header_block(report) -> writer
  2. module_table(report.modules) via comfy_table with optional color per row
  3. evidence_section("Top strengths", report.top_strengths, ✔)
  4. evidence_section("Top concerns",  report.top_concerns,  ✖)
  5. if !report.caveats.empty(): caveats_section(report.caveats)
```

---

## 7. Closed loop

- **Goal metric:** insta snapshot test against a fixture report; manual visual review on Day 5 benchmark sweep.
- **Where it lives:** CI; MemPalace `ops/distribution`.
- **Read by:** every CLI user — terminal output is the default.
- **Improvement path:** Day 5+ may add `--theme dark|light` if user demand surfaces.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/reports-terminal.md` lists ≥4 scenarios.
- [ ] `src/reports/terminal.rs::write` implemented; replaces stub.
- [ ] Color suppression via `console::Term::stdout().is_term()` when stdout is piped.
- [ ] ≥6 unit tests on the row formatter, header formatter, and color suppression.
- [ ] 1 insta snapshot test against a fixed `TrustReport` (with `--no-color` so the snapshot is plain).
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-04 — sparklines or histograms for activity timeline? — No, Day 5+ if any. Day 4 keeps to the table+evidence shape.

---

## 11. References

- `docs/architecture.md` §5 — TrustReport shape.
- `comfy_table` 7.x docs.
- `console` 0.15 docs.
