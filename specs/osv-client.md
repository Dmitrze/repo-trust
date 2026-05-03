---
feature: osv-client
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/osv-client.md"]
related_runbooks: []
related_docs: ["docs/api-notes.md#osv-dev"]
---

# OSV.dev client

> Federated query client over `POST https://api.osv.dev/v1/query`. Takes package coordinates `(name, ecosystem, version)`, returns the list of OSV advisories affecting that version. Withdrawn advisories are filtered client-side. Day 2 wires the client; the Security module's actual OSV usage is enabled Day 3 once the Adoption module gives us a `(repo → packages)` map via deps.dev.

---

## 1. Goal

`api::osv::Client::query(coords) -> Result<Vec<OsvAdvisory>>` returns the open (non-withdrawn) advisories for the given package version. The Security module aggregates counts across the repo's published packages once Day 3 wires deps.dev mapping; for Day 2 the client is unit/wiremock-tested but the Security collector skips OSV calls (no package list available yet) and the security feature `osv_open_advisories` defaults to 0 with a `defer_to_phase_3` caveat.

We know it works when: wiremock returns a fixture body with one advisory, the client parses it; wiremock returns a withdrawn advisory, the client filters it out; wiremock returns `{}`, the client returns an empty Vec.

---

## 2. Non-functional requirements

- **TTL:** 6 hours per `architecture.md` §6.3.
- **Latency:** cache-hit <2ms; cache-miss <500ms.
- **Withdrawn filter:** server-side via the `withdrawn` field on each advisory.
- **Determinism:** advisories sorted by `id` before return.

---

## 3. Boundaries

### In scope (Day 2)
- `Client::query(PackageCoords) -> Vec<OsvAdvisory>` over `POST /v1/query`.
- DTOs: `PackageCoords { name, ecosystem, version }`, `OsvAdvisory { id, summary, modified: OffsetDateTime, withdrawn: Option<OffsetDateTime>, severity: Vec<Severity>, affected_versions: Vec<String> }`.
- ETag/cache via existing cache facade with key `osv:{ecosystem}:{name}:{version}`.
- Filter out advisories where `withdrawn.is_some()`.
- Wiremock tests for empty response, populated response, and withdrawn-filtering.

### Out of scope (Day 2)
- `POST /v1/querybatch` for multiple packages at once — Day 3.
- OSV's "querybysource" repo-URL endpoint — not in the v1 OSV API.
- Severity score normalization (we surface OSV's CVSS strings as-is).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. (Day 3) Security collector receives `(repo → [npm:lodash 4.17.20, crates:reqwest 0.11.0])` from Adoption.
2. For each package, calls `osv::Client::query(coords)`.
3. Cache miss → `POST /v1/query {"package":{"name":"lodash","ecosystem":"npm"},"version":"4.17.20"}`.
4. Response 200 with `{"vulns":[{...}]}`; cache stores body with 6h TTL.
5. Parsed to `Vec<OsvAdvisory>`; entries with `withdrawn` set are dropped; remaining sorted by `id`.
6. Returned to caller.

---

## 6. Architecture sketch

```
[ Security collector ] --> Client::query({name:"lodash", ecosystem:"npm", version:"4.17.20"})
                                 |
                            cache key = "osv:npm:lodash:4.17.20"
                                 |
                              hit fresh ─► return parsed Vec
                                 |
                              miss/stale ─► POST /v1/query ─► 200 → store + parse + filter withdrawn → return
                                                          └─► other → Err
```

---

## 7. Closed loop

- **Goal metric:** `tests/osv_client_integration.rs` passes for empty/populated/withdrawn-filtered/error fixtures.
- **Where it lives:** CI; MemPalace `collectors/osv`.
- **Read by:** Reviewer.
- **Improvement path:** if OSV adds new severity formats we extend the deserializer.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/osv-client.md` lists ≥4 scenarios.
- [ ] `src/api/osv.rs::Client::query` implemented.
- [ ] DTOs serialize/deserialize the public OSV schema.
- [ ] Withdrawn filter applied; stable sort by `id`.
- [ ] Wiremock tests for empty, populated, withdrawn-filtered, server-error.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Day 3 may want batched `/v1/querybatch` for repos with many packages. Defer until adoption module benchmarks make it necessary.

---

## 10. Closed questions (history)

- 2026-05-04 — query by source repo URL? — Not supported by `/v1/query`; the OSV-schema field `affected.repo` exists but is not a queryable index. Mapping happens via deps.dev (Day 3).

---

## 11. References

- `docs/api-notes.md` §OSV.dev.
- ADR-0005 — Federate, don't replicate.
