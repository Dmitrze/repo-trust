# 0004 — No machine learning in v1

## Status
Accepted (May 2026)

## Context

Fake-star detection is the headline novelty of the Star Authenticity module. There is genuine machine-learning research on the topic (random forests, gradient boosting, even fine-tuned LLMs trained on GHArchive corpora). Using ML could plausibly improve detection precision and recall.

We also have a competing concern: **explainability**. Every score must be defensible. False positives in fake-star flagging can harm real maintainers and are treated as worse than false negatives.

## Decision

v1 uses a **transparent, heuristic-driven, weighted-evidence model** with documented thresholds. **No ML classifiers**.

This follows the approach validated by:
- Dagster's `fake-star-detector` (2023) — heuristic-based, explainable.
- StarScout (ICSE 2026) — heuristic-driven with a small statistical layer for lockstep timing detection.

## Consequences

### Easier
- Every score is auditable: a user can read `docs/methodology.md` and replay the logic by hand.
- Methodology disputes are productive: "the threshold for low-activity-stargazer share should be 25%, not 20%, because ___" is concrete.
- We can publish the full algorithm without giving away a moat. The moat is the methodology rigor itself, not the model weights.
- Model versioning is just a TOML file; no model artifacts to ship.
- Reproducibility is automatic.

### Harder
- We accept lower precision/recall than a tuned ML model could achieve.
- We must do more thoughtful work on threshold calibration (handled by the benchmark plan in `docs/benchmark-plan.md`).

### Trade-offs explicitly accepted
- We accept worse classification metrics in exchange for full transparency and contributor accessibility.

## Future work

A future major version (v2 or later) may introduce a **transparent, audited** ML layer for confidence band adjustment only — never for the score itself. Any such addition will require an ADR and a methodology review.

We will not bolt on a black-box LLM scoring layer. That would erase our differentiation against Snyk Advisor and Socket.dev, both of which lean on opaque scoring.

## Alternatives considered

### Random forest on stargazer features
**Why considered:** Probably the highest-precision option; supported by literature.

**Why rejected:** Even tree-based models are difficult to explain to a maintainer whose repo got a low score. Threshold tweaks become opaque retraining cycles. Reproducibility requires shipping the model weights.

### Fine-tuned LLM judge
**Why considered:** Could capture nuanced patterns.

**Why rejected:** Worst case for explainability and reproducibility. Inference cost. Dependency on a hosted API.

### Hybrid: heuristics for score, ML for confidence only
**Why considered:** Best of both.

**Why rejected for v1:** We don't have the benchmark data yet to train a confidence model fairly. Defer to v2.
