# /tests/scenarios — Behavioral Scenarios

> **Scenarios are how we describe success in concrete, agent-iterable form.** Specs say what a feature is. Scenarios say what "working" looks like in actual cases. Implementations are iterated against scenarios until they pass.
>
> See `AI_NATIVE_CONSTITUTION.md` (principle 5) and `docs/AI_NATIVE_PLAYBOOK.md` section 6.

---

## What goes here

One file per feature, mirroring `specs/<feature>.md` one-to-one. If a spec exists, a scenarios file exists.

Named `<kebab-case-feature>.md` — same name as the spec.

---

## What scenarios are (and aren't)

**Scenarios are:**
- Concrete cases described in a Given / When / Then shape.
- Mixed: happy paths, edge cases, failure modes, accessibility cases, performance cases.
- Detailed enough that an LLM judge can decide pass/fail unambiguously.
- The artifact agents iterate against when generating code.

**Scenarios are not:**
- Pseudocode for the implementation.
- A duplication of the spec's prose.
- Vague ("the system should work well").
- Minimal ("just one happy path"). At least 5; usually 10-20.

**Scenarios are not unit tests** — unit tests in `src/` and Edge Function code live next to the code they cover. Scenarios are higher-level: they describe behavior at the feature boundary, often spanning multiple modules.

---

## Format

Start from `_TEMPLATE.md`. Every scenarios file has:

1. **Spec link** at the top — the matching `/specs/<feature>.md`.
2. **Sections** grouping scenarios: happy paths, edge cases, failure modes, performance, accessibility, security.
3. **Each scenario** in this shape:

```
### S-<NNN>: <short descriptive title>

**Given** <preconditions>
**When** <action>
**Then** <observable outcome>

Notes:
- (optional) implementation hints, eval set inclusion, severity, etc.
```

The `S-<NNN>` IDs are stable: once a scenario is numbered, the number never changes. New scenarios get the next free number. Removed scenarios are marked deprecated, not deleted.

---

## Categories of scenarios

Most feature scenario files cover at least:

- **Happy path** — the typical user flow. 2-3 of these.
- **Edge cases** — boundary inputs (empty, very long, unicode, wrong locale, simultaneous requests). 3-7 of these.
- **Failure modes** — third-party API down, network timeout, malformed input, auth missing. 3-7 of these.
- **Accessibility** — keyboard navigation, screen reader, mobile viewport. 2-3 of these.
- **Performance** — latency under load, throughput limits. 1-3 of these (where relevant).
- **Security / privacy** — RLS isolation, PII redaction, secret-leak prevention. 2-4 of these.

Not every feature needs every category, but the spec's non-functional requirements drive which categories must be covered.

---

## How agents use this directory

Claude Code uses scenarios as the iteration target:

1. Read the spec in `/specs/<feature>.md`.
2. Read the scenarios in `/tests/scenarios/<feature>.md`.
3. Generate or update implementation.
4. Run automated tests (unit, E2E, evals) tied to the scenarios.
5. Iterate until all scenarios pass at the threshold defined in the spec.
6. For LLM features: run the Braintrust eval suite; ensure judge mean >= threshold.

When a scenario fails, the agent does not paper over it with a special-case fix. It either updates the implementation, updates the scenario (if the scenario was wrong), or escalates to the user (if the spec needs to change). All three paths are valid; silent suppression is not.

---

## How humans use this directory

- Before asking an agent to implement, write 5-10 scenarios first — it sharpens the spec.
- During review, scan the scenarios file for missing categories. "Did we test the failure modes?" is a routine review question.
- When triaging a bug: add a new scenario that reproduces the bug *before* fixing it. The scenario stays in the file forever as a regression check.

---

## Relationship to existing test infrastructure

Scenarios in this directory are the source of truth. They are realized in code as:

- **Vitest unit tests** in `src/**/*.test.ts(x)` for component-level behavior.
- **Playwright E2E tests** in `e2e/` for cross-page flows.
- **Braintrust eval suites** in `evals/` for LLM-driven scenarios.

A scenario can map to multiple realizations (same scenario in unit + E2E + eval). The scenario file is what humans read; the test code is what runs in CI.

---

## Lifecycle

Scenarios live with the feature. When a feature is deprecated, its scenarios file is marked `deprecated` in the frontmatter but retained for historical reference.

When a scenario is intentionally removed (e.g., the underlying behavior is no longer required), it is marked `deprecated` inline rather than deleted.

---

*Maintainer: the founder. The DRI for any individual scenarios file is the same as the DRI on the matching spec.*
