---
feature: github-api-client
status: accepted
spec: ../../specs/github-api-client.md
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
---

# GitHub API client — Scenarios

Link: [`specs/github-api-client.md`](../../specs/github-api-client.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | 200 cache miss; 304 cache hit |
| Edge cases | 3 | rate-limit pause; conditional fetch with stale cache; stargazer pagination |
| Failure modes | 2 | 404; 401 (bad token) |
| Performance | 1 | parallel fetch coordination |

---

## Happy path

### S-001: 200 OK populates cache

**Given** an empty cache and a wiremock returning `200 OK + ETag: "abc"` for `GET /repos/octocat/Hello-World`
**When** `Client::get_repo("octocat", "Hello-World")` is called
**Then** the response is parsed; cache contains an entry with key `github:repos:octocat/Hello-World:metadata`, etag `Some("abc")`, body matching the mock body, `expires_at ≈ now + 24h`.

### S-002: 304 Not Modified reuses cached body

**Given** cache contains entry with etag `"abc"` and body `B`; wiremock returns `304 Not Modified` when `If-None-Match: "abc"` is sent
**When** `Client::get_repo` is called after the cache entry is past its `expires_at`
**Then** the request includes the conditional header; the response is `304`; `Cache::touch` is called; the returned body equals `B`.

---

## Edge cases

### S-101: rate-limit pause when remaining < 10

**Given** wiremock returns `200 OK + X-RateLimit-Remaining: 5 + X-RateLimit-Reset: now+30s`
**When** `Client::get_repo` is called
**Then** the next call to `Client::*` blocks (via `RateLimiter::acquire`) until the reset time is reached; a `tracing::warn` event fires with reason="rate-limit pause".

### S-102: conditional fetch on stale cache without forced refresh

**Given** cache entry exists with `expires_at = now - 1s`, etag `"abc"`
**When** `Client::get_repo` is called (no `--refresh`)
**Then** a single conditional `GET` is issued; depending on response, either body is reused (304) or replaced (200).

### S-103: stargazer pagination follows Link headers

**Given** wiremock returns `Link: <...&page=2>; rel="next"` on page 1 and no `next` on page 2
**When** `Client::list_stargazers("octocat/Hello-World", limit=200)` is called
**Then** two HTTP calls are made; concatenated stargazers are returned; each page is independently cached with its own ETag.

---

## Failure modes

### S-201: 404 Not Found returns typed error

**Given** wiremock returns `404 Not Found`
**When** `Client::get_repo("ghost", "ghost")` is called
**Then** the result is `Err(GithubError::NotFound)`; CLI maps this to exit code 2 per architecture §8.

### S-202: 401 Unauthorized surfaces auth failure

**Given** wiremock returns `401`
**When** any `Client::*` method is called with a bad token
**Then** the result is `Err(GithubError::Unauthorized)`; CLI maps to exit code 3 per architecture §8.

---

## Performance

### S-401: parallel calls are coordinated by semaphore

**Given** semaphore configured with 10 permits; wiremock returns 200 OK for any `/repos/.*/.*` path
**When** 20 concurrent `Client::get_repo` calls are launched via `JoinSet`
**Then** at most 10 are in-flight simultaneously (verified by wiremock request log); all 20 eventually succeed.
