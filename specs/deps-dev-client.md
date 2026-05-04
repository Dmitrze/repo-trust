---
feature: deps-dev-client
status: accepted
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
related_agents: []
related_scenarios: ["tests/scenarios/deps-dev-client.md"]
related_runbooks: []
related_docs: ["docs/api-notes.md#deps-dev"]
---

# deps.dev client

> Federated read-only client over `https://api.deps.dev/v3`. Two endpoints in scope: project→packages mapping and per-package metadata (incl. weekly downloads). Powers the Adoption Signals module's federation per ADR-0005.

---

## 1. Goal

`api::deps_dev::Client::project_packages(owner, repo)` returns the list of published packages a GitHub repo maps to. `Client::package(system, name)` returns one package's metadata including `weekly_downloads` where deps.dev provides it. The Adoption module aggregates downloads across the returned package list.

We know it works when: the wiremock fixture for `prometheus/prometheus` returns one package mapping with non-zero downloads, parsed into a `Vec<PackageRef>` and a `PackageInfo`.

---

## 2. Non-functional requirements

- **TTL:** 24h per `architecture.md` §6.3.
- **Latency:** cache-hit <2ms; cache-miss <500ms.
- **Errors typed:** distinguish `Ok(None)` for repos with no published packages (200 + empty list) from `Err` for true failures.
- **Determinism:** parsed list sorted by `(system, name)` for deterministic JSON output.

---

## 3. Boundaries

### In scope (Day 3)
- `Client::project_packages(owner: &str, repo: &str) -> Result<Vec<PackageRef>>` over `GET /v3/projects/{kind}/{repo}/packages` (kind="github.com", repo="owner/repo").
- `Client::package(system: &str, name: &str) -> Result<PackageInfo>` over `GET /v3/systems/{system}/packages/{name}`.
- DTOs: `PackageRef { system, name }`, `PackageInfo { system, name, weekly_downloads: Option<u64>, latest_version: Option<String> }`.
- ETag/cache via existing `storage::Cache`; cache keys `deps_dev:projects:{owner}/{repo}:packages` and `deps_dev:systems:{system}:{name}`.
- 5xx → `Err(DepsDevError::Other { status, body })`. 404 on the project endpoint → `Ok(Vec::new())` (no packages mapped).
- Wiremock tests for happy path, 404 → empty, malformed JSON, 5xx, deterministic sort.

### Out of scope (Day 3)
- `/v3alpha/` endpoints (we use stable v3).
- Batched queries.
- Authentication (deps.dev is fully public).
- Direct repo→commit mapping (only repo→packages).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. Adoption collector calls `deps_dev::Client::project_packages("prometheus", "prometheus")`.
2. Cache miss → `GET /v3/projects/github.com/prometheus/prometheus/packages`.
3. Response 200 with body `{"packages":[{"system":"GO","name":"github.com/prometheus/prometheus"},...]}`.
4. Cache stores raw body with 24h TTL; client returns sorted `Vec<PackageRef>`.
5. Adoption iterates the result and calls `package(system, name)` per entry; downloads summed.

For the empty-list scenario: 200 with `{"packages":[]}` or 404 → `Ok(Vec::new())`. Adoption surfaces this as `no packages found` Neutral evidence + Medium confidence.

---

## 6. Architecture sketch

```
[ Adoption collector ]
       |
       v
project_packages(owner, repo)
       |
       cache miss → GET /v3/projects/github.com/{owner}/{repo}/packages
       |               200 → parse + sort + return
       |               404 → return Ok(Vec::new())
       |               5xx → return Err
       v
package(system, name) per entry
       |
       cache miss → GET /v3/systems/{system}/packages/{name}
       |               200 → parse + return
       v
return PackageInfo with downloads
```

---

## 7. Closed loop

- **Goal metric:** `tests/deps_dev_client_integration.rs` passes ≥5 cases.
- **Where it lives:** CI; MemPalace `collectors/deps-dev`.
- **Read by:** Reviewer; Day 5 real-API validation against `prometheus/prometheus` and `rust-lang/cargo`.
- **Improvement path:** if deps.dev adds richer signals (transitive dep counts), extend `PackageInfo` and bump the `SCORING_VERSION`.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/deps-dev-client.md` lists ≥5 scenarios.
- [ ] `src/api/deps_dev.rs::Client` exposes `project_packages` + `package` + `with_base_url`.
- [ ] DTOs (`PackageRef`, `PackageInfo`) carry only the fields Adoption uses.
- [ ] `src/api/mod.rs` re-exports as `DepsDevClient`.
- [ ] ETag-aware fetch via existing cache facade; 24h TTL.
- [ ] 5+ wiremock tests cover happy path, 404 → empty, malformed JSON, 5xx, deterministic sort, cache hit.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-05 — use `/v3` or `/v3alpha`? — `/v3` per spec; `/v3alpha` reserved for unstable additions we don't need.
- 2026-05-05 — query downloads per-version? — No, package-level aggregate is what Adoption needs; per-version queries are a v1.1 follow-up.

---

## 11. References

- `docs/api-notes.md` §deps.dev.
- ADR-0005 — Federate, don't replicate.
- ADR-0012 — `RepositoryContext` runtime handles.
