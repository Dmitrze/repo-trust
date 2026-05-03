## What & why

<!-- One-paragraph description of what this PR changes and why. Link to the issue or discussion if one exists. -->

Closes #

---

## Type of change

- [ ] feat — user-visible behavior change (new feature, new flag, new module)
- [ ] fix — bug fix
- [ ] docs — documentation only
- [ ] refactor — internal restructuring, no behavior change
- [ ] test — adding or improving tests
- [ ] chore — dependencies, CI, tooling
- [ ] perf — performance improvement

## Scope of change

- [ ] Touches scoring weights or thresholds (requires `docs/scoring-model.md` bump)
- [ ] Changes the JSON report schema (requires `schema_version` bump)
- [ ] Adds a new runtime crate (justify in description)
- [ ] Adds a new public function or CLI flag (requires docs)
- [ ] Adds a new module (requires ADR)

---

## Verification

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] If applicable: `cargo insta review` ran and snapshot changes are intentional
- [ ] If applicable: regression test added for the bug being fixed
- [ ] Manual smoke test against a real repo (paste command + summary of output below)

```
# Paste the command and a short excerpt of the output here.
```

---

## Risks

<!-- What could go wrong because of this change? E.g. "changes confidence calculation; could shift scores by ±2 across the benchmark set". If you don't know of any risks, write "None known." -->

---

## For reviewers

- [ ] Reviewed against the relevant `docs/methodology.md` section
- [ ] Determinism preserved (no new `HashMap` ordering dependencies in serialized output)
- [ ] No new outbound network calls beyond documented APIs
- [ ] No secrets or tokens in the diff
