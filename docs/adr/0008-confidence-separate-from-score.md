# 0008 — Confidence is independent of score

## Status
Accepted (May 2026)

## Context

A naive composite score collapses two distinct signals: **how good** the repo is and **how sure we are** about that judgment. Conflating them produces problems:
- A repo with sparse data and a high score looks identical to a well-measured repo with a high score.
- Users can't tell when to trust the score vs when to look closer.
- Comparing two scores becomes meaningless if confidence differs.

This pattern shows up in mature scoring systems: credit ratings include a confidence band, weather forecasts include a probability, election forecasts separate "point estimate" from "uncertainty."

## Decision

Every score in Repo Trust is reported alongside a **confidence band**: `Low / Medium / High`. Confidence is a separate axis, computed independently from the score itself.

### Per-module confidence
Determined by:
- **Data completeness** — fraction of expected inputs collected.
- **Sample size** — for sample-based modules, the size of the sampled population vs configured target.
- **Cross-signal agreement** — when multiple sub-signals agree, confidence rises.
- **Staleness** — cached-data age relative to repo activity.

### Aggregate confidence
The overall report confidence is the **minimum** of any module that contributed >10% of the final score.

### How it's surfaced
- Terminal output: `Trust Score: 73 / 100 · Category: Good · Confidence: Medium`.
- JSON: `"overall_confidence": "Medium"` and `"confidence": "..."` per module.
- Markdown report: a column in the module table.
- A high-score-low-confidence repo is rendered with a visual cue (⚠️) and an explicit caveat.

## Consequences

### Easier
- Users can compare two scores fairly: a Score 80 / High vs Score 80 / Low are not the same statement.
- Partial-data scans degrade gracefully (lower confidence, score is still reported).
- We are honest about what we don't know; this is the entire epistemic posture of the project.
- Reports are defensible: "the score is 73 with Low confidence because the stargazer sample was 50 of a target 200" is a real answer.

### Harder
- Two-axis output is more cognitive load than a single number; we mitigate via clear visual hierarchy in reports.
- Aggregate-confidence calculation must be carefully calibrated; we publish the formula and validate it on the benchmark set.

## Alternatives considered

### Single number: confidence-weighted score
**Why considered:** Simpler.

**Why rejected:** Conflates the two signals. A user can't recover the underlying confidence from the weighted number.

### Confidence interval (e.g. "73 ± 8")
**Why considered:** Most rigorous statistically.

**Why rejected:** Implies probabilistic semantics we can't actually justify. Heuristic scoring doesn't yield calibrated intervals.

### Five-band confidence
**Why considered:** More granular.

**Why rejected:** Marginal information gain; harder for users to internalize. Three bands match how humans naturally bucket uncertainty.
