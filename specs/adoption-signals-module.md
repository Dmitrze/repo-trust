---
feature: adoption-signals-module
status: accepted
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
related_agents: []
related_scenarios: ["tests/scenarios/adoption-signals-module.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-4--adoption-signals", "docs/module-specs.md#adoption-signals"]
---

# Adoption Signals module

> Fourth end-to-end module. Federates deps.dev for repo→packages mapping and weekly downloads, queries GitHub for documentation maturity (README presence + word count + `docs/`/`examples/` folders), checks an awesome-list mention list (default empty). Graceful degradation: no published package → Medium confidence + caveat; deps.dev unavailable → Low confidence + caveat.

---

## 1. Goal

`AdoptionModule::run(ctx)` returns `(ModuleResult, Vec<EvidenceItem>)` answering "is this repository actually used in the wild?". We know it worked when: a repo with one package + 100k weekly downloads + README + docs/ scores ≥75; a repo with no published package falls back to documentation-only with Medium confidence and a `no_packages` caveat.

---

## 2. Non-functional requirements

- **API budget:** ≤15 calls in Standard mode (1 deps.dev project mapping + N package metadata + 4 GitHub doc-presence + 1 README content fetch).
- **Determinism:** same inputs → same `(score, sub_scores, evidence)` byte-for-byte.
- **Conservative posture:** absence of a package is a `no_packages` caveat, not a `Concerning` verdict (per `methodology.md` §Module 4: "We don't penalize this — we simply lower the module's confidence to Medium and surface the absence as a caveat").

---

## 3. Boundaries

### In scope (Day 3)
- `src/collectors/adoption.rs::collect` — federates deps.dev `project_packages` + per-package downloads, GitHub doc-presence (`README.md`, `docs/`, `examples/`, `README` fallback), README content fetch via `GET /repos/{owner}/{repo}/readme` (200 → base64-decode body and word-count).
- `src/features/adoption.rs::compute` — sums downloads across packages; doc maturity = present-flag set + README word count bucket; awesome-list mentions count.
- `src/scoring/adoption.rs::score` — sub-scores: `weekly_downloads` (logarithmic banding), `package_systems_count`, `documentation_maturity`, `awesome_list_mentions`. Federation evidence + caveats.
- `src/modules/adoption.rs::run` — wires the three stages.
- ≥10 unit tests on scorer + 1 wiremock integration test covering one-package + zero-package paths.

### Out of scope (Day 3)
- GitHub `/dependents` HTML scraping — brittle, opt-in via a flag in v1.1 if user demand justifies it. Day-3 spec: omit gracefully (no value reported).
- Docker Hub pulls — Phase 2.
- Custom awesome-lists from a file — supported by config but loaded as empty default in Day 3; users override via `~/.repo-trust/config.toml` `[adoption] awesome_lists = ["..."]`.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. `cli::scan::execute` builds context, picks `AdoptionModule`.
2. Collector calls `deps_dev::Client::project_packages("prometheus", "prometheus")` → `[{system: "GO", name: "github.com/prometheus/prometheus"}]`.
3. For each package, calls `deps_dev::Client::package(...)` → `weekly_downloads = Some(100_000)`.
4. Concurrently fetches `/repos/.../.../readme` (200, base64-decoded body) and probes for `docs/` + `examples/` directories.
5. Features layer builds `AdoptionFeatures` (downloads sum, package_systems = ["GO"], doc_maturity_score, awesome_list_mentions = 0).
6. Scorer maps to sub-scores and arithmetic-mean.
7. Confidence: High when downloads + doc maturity both present; Medium when no packages; Low when deps.dev errored.
8. Evidence: ≥3 items including `weekly_downloads`, `package_systems`, `documentation_maturity`.

For the no-packages scenario:
- `project_packages` returns empty Vec.
- Features `weekly_downloads = None`, `package_systems = []`.
- Scorer drops the downloads sub-score; emits `no_packages` Neutral evidence.
- Confidence drops to Medium.

---

## 6. Architecture sketch

```
[ deps.dev Client ] -----► project_packages(owner, repo) → Vec<PackageRef>
                          (per-package) package(system, name) → PackageInfo
[ GitHub Client ]   -----► /repos/{owner}/{repo}/readme + file_exists for docs/ + examples/
                                                  |
                                                  v
                                  AdoptionCollector::collect → AdoptionRawData
                                                  |
                                                  v
                                  AdoptionFeatures::compute → AdoptionFeatures
                                                  |
                                                  v
                                  scoring::adoption::score → (ModuleResult, Vec<EvidenceItem>)
                                                  |
                                                  v
                                       AdoptionModule::run returns
```

---

## 7. Closed loop

- **Goal metric:** ≥10 scorer unit tests + 1 wiremock integration test green.
- **Where it lives:** CI; MemPalace `modules/adoption`.
- **Read by:** Reviewer; Day 5 benchmark sweep against prometheus/prometheus + rust-lang/cargo.
- **Improvement path:** if benchmark shows downloads thresholds mis-classify mid-popularity packages, tune the logarithmic bands in v1.1.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/adoption-signals-module.md` lists ≥6 scenarios.
- [ ] `src/collectors/adoption.rs::collect` implemented (deps.dev federation + GitHub README + dir probes).
- [ ] `src/features/adoption.rs::compute` implemented.
- [ ] `src/modules/adoption.rs::run` returns real data (no `bail!`).
- [ ] `src/scoring/adoption.rs::score` exposes ≥3 sub-scores; federation policy applied.
- [ ] ≥10 unit tests on scorer.
- [ ] 1 wiremock integration test (one-package happy path + zero-package fallback).
- [ ] No package → `Medium` confidence + `no_packages` evidence (NOT a `Concerning` verdict).
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Should `weekly_downloads` thresholds be linear, logarithmic, or tier-based? Day 3 v1: logarithmic banding (0 → 0, 1k → 25, 10k → 50, 100k → 75, 1M+ → 100). Tune in v1.1 if benchmark says.

---

## 10. Closed questions (history)

- 2026-05-05 — should we use `GET /repos/.../.../dependents` (HTML scrape)? — No; brittle and rate-limit-burning. Defer to v1.1 if user demand surfaces.

---

## 11. References

- `docs/methodology.md` §Module 4 — Adoption Signals.
- `docs/module-specs.md` §Adoption Signals.
- `docs/api-notes.md` §deps.dev.
- ADR-0005 — Federate, don't replicate.
- ADR-0011, ADR-0012.
