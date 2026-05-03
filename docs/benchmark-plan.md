# Benchmark Plan

How we validate that the scoring model produces sensible results, and how we'll continue to improve it.

---

## The benchmark set

We maintain a curated set of ≈50 GitHub repositories spanning five expected categories. Each repo's expected category is documented in `examples/benchmark-set.csv`. The set is rotated annually.

| Expected category | Repo count | Selection criteria |
| --- | --- | --- |
| Strong | 12 | Established, multiple maintainers, high adoption, signed releases (e.g. `rust-lang/rust`, `tokio-rs/tokio`, `kubernetes/kubernetes`) |
| Good | 12 | Healthy projects with one or two minor weaknesses |
| Mixed | 12 | Real strengths and real weaknesses (e.g. very active but solo-maintained) |
| Weak | 8 | Significant concerns: archived, abandoned, stale releases |
| High Risk | 6 | Documented examples (with evidence) of fake-star campaigns or deceptive packaging |

The High-Risk set is the most sensitive. We only include repos for which a credible third party (academic paper, news article, GitHub takedown) has published evidence. We **never** add a repo to High Risk based on our own judgment alone.

## Validation procedure

A `cargo run --release --bin benchmark` (or equivalent script in `scripts/run_benchmark.rs`) produces a CSV with the actual score and category for every repo in the benchmark set, against the current scoring version.

### Stability metric
For each scoring-model release, we measure:
- **Category accuracy:** % of repos that landed in their expected category (target: ≥ 80%).
- **Category drift:** for repos whose category changed since the previous release, we record the delta and require an entry in `docs/scoring-model.md` change log explaining it.
- **Score variance:** standard deviation of the absolute score change across the benchmark set (target: ≤ 5 points unless a major release).

### Per-module precision
For the Star Authenticity module specifically, we measure:
- **True positives:** how many of the 6 High-Risk repos are flagged with `low_activity_share ≥ 20%` AND `lockstep_z_score ≥ 5`. Target: ≥ 5/6.
- **False positives:** how many of the 12 Strong repos get flagged. Target: 0.

False positives in Star Authenticity are treated as worse than false negatives. A scoring change that improves recall on High-Risk but introduces even one false positive on Strong is rejected.

## How to propose a benchmark set change

1. Open an issue with the proposed addition / removal.
2. Provide evidence for the expected category (links, prior coverage, repo metadata).
3. Run the current scoring against the proposed repo and attach the JSON.
4. The maintainer adds it after review.

We do **not** "tune to the benchmark set" — the set is for validation, not training. If our scoring suddenly does well on the set after a code change without an obvious mechanism, that is a red flag, not a green one.

## Public reporting

Every scoring-model release includes a benchmark report at `docs/benchmarks/<version>.md` showing:
- The benchmark set contents at release time.
- Category accuracy.
- Per-module precision.
- Notable category changes vs the previous release.

This is part of the release process; CI fails the release if the benchmark report is missing.
