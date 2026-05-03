---
feature: github-api-client
status: accepted
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
related_agents: []
related_scenarios: ["tests/scenarios/github-api-client.md"]
related_runbooks: []
related_docs: ["docs/api-notes.md", "docs/architecture.md#6-caching-and-rate-limits"]
---

# GitHub API client

> ETag-aware, rate-limit-aware HTTP client around `octocrab`. Wraps the few endpoints Phase 1+2 modules need (repo metadata, commits, releases, issues, PRs, contributors, stargazers). Talks to the cache facade so 304s never consume rate-limit budget.

---

## 1. Goal

Every collector reads GitHub through `api::github::Client::*` methods. Each method consults the cache, sends `If-None-Match` when a cached ETag exists, handles `304` by returning the cached body, handles `200` by writing the new body+etag back, and emits a `tracing` event with rate-limit-remaining on every response.

We know it worked when: a wiremock test replays a full module's worth of requests with cached ETags and zero requests reach the mocked-200 handler (all served from cache + 304-validated).

---

## 2. Non-functional requirements

- **Latency:** cache-hit path <2ms; cache-miss + 304 path <100ms; cache-miss + 200 <300ms (per individual request).
- **Concurrency:** all method calls are `Send + Sync`; the limiter coordinates concurrent calls via `tokio::sync::Semaphore` (default 10 in-flight).
- **Rate-limit safety:** when `X-RateLimit-Remaining < 100`, log `warn`; when `< 10`, pause via `tokio::time::sleep` until `X-RateLimit-Reset`.
- **Determinism:** for the same cached state, identical method calls return identical bodies.

---

## 3. Boundaries

### In scope (Phase 1)
- `api::github::Client` constructed from a `reqwest::Client`, an `octocrab::Octocrab`, and a `Cache` + `RateLimiter` pair.
- Methods needed for Activity + Maintainer + Stars + Adoption modules (repo metadata, commits with windows, releases, issues, PRs, contributors, stargazers paginated, optional `stargazers + starred_at` via the `application/vnd.github.star+json` Accept header per `api-notes.md`).
- ETag-aware fetch helper that handles cache + conditional request lifecycle.
- Rate-limit-aware semaphore in `utils/ratelimit.rs`.
- Honors `GITHUB_TOKEN` from `Config`.

### Out of scope (explicit)
- GraphQL endpoints (Phase 1 uses REST exclusively; star date heuristic falls back to "no star date" gracefully per methodology §1).
- App / OAuth installation flows.
- Search API (separate stricter limit; not needed for any v1 module).
- Mutation endpoints (write APIs are never used).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. `Client::get_repo("octocat", "Hello-World")` invoked.
2. Cache lookup for key `github:repos:octocat/Hello-World:metadata` returns `None`.
3. Semaphore permit acquired (1 of 10 in-flight).
4. octocrab issues `GET /repos/octocat/Hello-World`; response 200 + ETag.
5. Cache `put(key, Some(etag), body, ttl=24h)`.
6. Limiter parses `X-RateLimit-Remaining: 4998` (no warn).
7. Body deserialized to `octocrab::models::Repository`, returned to caller.
8. 25 hours later: same call, cache returns expired entry; conditional request `If-None-Match: <etag>`; 304; cache `touch`; same body returned.

---

## 6. Architecture sketch

```
[ Collector ] -> Client::get_repo(...)
                    |
                    v
              [ Cache::get(key) ]
                  |              |
              hit & fresh    miss/expired
                  |              |
              return body   acquire semaphore
                                 |
                            HTTP via octocrab/reqwest
                            with If-None-Match header
                                 |
                            +---304---> Cache::touch(key); return cached body
                            |
                            +---200---> Cache::put; return body
                                 |
                          parse rate-limit headers; log/pause if needed
```

---

## 7. Closed loop

- **Goal metric:** `tests/github_client_etag.rs::roundtrip_304_does_not_increment_rate_limit_consumption` passes (asserts wiremock saw exactly N+1 requests where N were 304s).
- **Where it lives:** CI; MemPalace `collectors/github-api`.
- **Read by:** Reviewer; rate-limit pause events visible in `--debug` logs.
- **Improvement path:** add a metrics endpoint in v1.1 surfacing cumulative remaining-rate over a session.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/github-api-client.md` lists ≥6 scenarios (200, 304, 404, rate-limit pause, conditional fetch with stale cache, parallel fetch coordination).
- [ ] `src/api/github.rs::Client` exposes the methods listed in §3.
- [ ] `src/api/client.rs::build()` returns the shared reqwest client used by both octocrab and any direct `reqwest::get` calls (Scorecard, OSV, deps.dev later).
- [ ] `src/utils/ratelimit.rs::RateLimiter` exposes `acquire().await` (semaphore permit) and `record(&headers)` (parses + pauses if needed).
- [ ] Wiremock integration tests cover the §3 in-scope behaviors.
- [ ] All quality gates green.

---

## 9. Open questions

- None — REST-only choice frozen for v1; GraphQL + star date heuristic deferred to Phase 2's deep stars work via fallback.

---

## 10. Closed questions (history)

- 2026-05-03 — REST or GraphQL for stargazer pagination? — REST for v1 per architecture §15; GraphQL adopted later only if rate-limit pressure demands it. Today's `vnd.github.star+json` Accept header gives us stargazer dates over REST.

---

## 11. References

- `docs/api-notes.md` — quirks per upstream.
- `docs/architecture.md` §6 — caching, ETag, rate limits.
