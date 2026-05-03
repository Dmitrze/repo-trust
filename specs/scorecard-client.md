---
feature: scorecard-client
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/scorecard-client.md"]
related_runbooks: []
related_docs: ["docs/api-notes.md#scorecard-dev", "docs/methodology.md#module-5--security--readiness"]
---

# scorecard.dev client

> Thin federated read-only client. Hits `https://api.scorecard.dev/projects/github.com/{owner}/{repo}`, returns the latest score + per-check results. 404 is a "not yet scored" signal, not an error — the security module degrades to Low confidence + doc-presence-only sub-scores when no Scorecard exists.

---

## 1. Goal

`api::scorecard::Client::get(owner, repo)` returns `Ok(Some(report))` for repos with a Scorecard score, `Ok(None)` for repos with no score, `Err` for true network/parse failures. Powers the highest-weighted sub-signal of the Security & Readiness module.

We know it works when: the security module's wiremock test gets a real-shaped Scorecard JSON for `prometheus/prometheus` and a 404 fallback for `octocat/Hello-World` (which is too small for Scorecard's batch run).

---

## 2. Non-functional requirements

- **TTL:** 7 days per `architecture.md` §6.3 (Scorecard runs weekly).
- **Latency:** cache-hit <2ms; cache-miss <500ms (single GET, no pagination).
- **Errors are typed:** distinguish 404 (not scored) from 4xx/5xx (real failure). The security module only treats 404 as "fall back to doc-presence-only".
- **Determinism:** for the same upstream JSON the parsed `ScorecardReport` is byte-stable.

---

## 3. Boundaries

### In scope (Day 2)
- `Client::get(owner, repo) -> Result<Option<ScorecardReport>>`.
- `ScorecardReport` DTO with `score: f64` (0.0-10.0), `check_results: Vec<CheckResult>`, `date: OffsetDateTime` (the Scorecard run date), `repo_url: String`.
- `CheckResult { name: String, score: i32 (-1..=10), reason: String, documentation: Documentation }`.
- ETag-aware fetch via the existing `storage::Cache`.
- Wiremock tests for 200 + 404 paths.

### Out of scope
- Running Scorecard ourselves (we federate — see ADR-0005).
- Re-implementing Scorecard's checks.
- Authenticated requests (Scorecard is fully public).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. Security module calls `scorecard::Client::get("prometheus", "prometheus")`.
2. Cache miss → GET `https://api.scorecard.dev/projects/github.com/prometheus/prometheus`.
3. Response 200 + `ETag: "x"` + JSON body; cache stores with 7-day TTL.
4. Parsed to `ScorecardReport` with `score = 8.7`, ~18 check results.
5. Returned to caller.
6. 1 hour later: same call → cache hit (still fresh) → returned immediately, no network.
7. 8 days later: same call → cache stale → conditional `If-None-Match: "x"` → server returns 304 → cache TTL refreshed.

For 404 scenario: `Client::get` returns `Ok(None)`; the security module logs a Neutral evidence item ("Scorecard has not yet scored this repository") and falls back to doc-presence-only with Low confidence.

---

## 6. Architecture sketch

```
[ Security collector ] --> Client::get("prometheus", "prometheus")
                                 |
                            cache lookup (key: "scorecard:projects/github.com/prometheus/prometheus")
                                 |
                              hit fresh ─────────────► return parsed ScorecardReport
                                 |
                              hit stale ─► If-None-Match ─► 304 → refresh TTL → return
                                                       └─► 200 → store + parse
                                 |
                              miss ─► GET ─► 200 → store + parse → return Some
                                            └► 404 → return Ok(None)
                                            └► other → Err
```

---

## 7. Closed loop

- **Goal metric:** `tests/scorecard_client_integration.rs::s001_200_returns_report` and `s002_404_returns_none` pass.
- **Where it lives:** CI; MemPalace `collectors/scorecard`.
- **Read by:** Reviewer; Day 5 real-API validation.
- **Improvement path:** if Scorecard adds new check shapes the deserializer will surface unknown fields — bump TTL or schema accordingly.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/scorecard-client.md` lists ≥4 scenarios.
- [ ] `src/api/scorecard.rs::Client::get` implemented, returns `Result<Option<ScorecardReport>>`.
- [ ] `ScorecardReport` + `CheckResult` typed DTOs.
- [ ] ETag-aware fetch reusing the existing cache facade.
- [ ] Wiremock tests cover 200, 304, 404, and a malformed-JSON 5xx fallback.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-04 — should we cache parsed ScorecardReport vs raw JSON? — Cache raw JSON; parse on read so a future schema bump only needs a deserializer change, not a cache wipe.

---

## 11. References

- `docs/api-notes.md` §scorecard.dev.
- `docs/methodology.md` §Module 5 — Security & Readiness — Federation: Scorecard.
- ADR-0005 — Federate, don't replicate.
