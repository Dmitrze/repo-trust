---
feature: scorecard-client
status: accepted
spec: ../../specs/scorecard-client.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# scorecard.dev client — Scenarios

Link: [`specs/scorecard-client.md`](../../specs/scorecard-client.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 1 | 200 with valid report |
| Edge cases | 2 | 304 revalidation; 404 not-yet-scored |
| Failure modes | 2 | malformed JSON; 5xx transient |

---

## Happy path

### S-001: 200 returns parsed ScorecardReport

**Given** wiremock returns `200 OK` + `ETag: "x"` + a valid Scorecard JSON body for `prometheus/prometheus`
**When** `Client::get("prometheus", "prometheus")` is called
**Then** the result is `Ok(Some(report))` with `report.score ≈ 8.7`, ≥10 `check_results` entries, and the cache contains an entry under `scorecard:projects/github.com/prometheus/prometheus`.

---

## Edge cases

### S-101: cache hit serves without network

**Given** the cache contains a fresh entry for `prometheus/prometheus`
**When** `Client::get` is called within the 7-day TTL
**Then** the wiremock mock receives **zero** requests; the parsed report matches the cached body.

### S-102: 404 returns Ok(None)

**Given** wiremock returns `404 Not Found` for `octocat/Hello-World`
**When** `Client::get("octocat", "Hello-World")` is called
**Then** the result is `Ok(None)`. No error is raised — the security module treats this as "not yet scored" and falls back to doc-presence-only with Low confidence.

---

## Failure modes

### S-201: 5xx is a real error

**Given** wiremock returns `500 Internal Server Error`
**When** `Client::get` is called
**Then** the result is `Err`. The CLI maps this to exit code 7 (network/upstream failure) per architecture §8.

### S-202: malformed JSON returns parse error

**Given** wiremock returns `200 OK` with body `not valid json {`
**When** `Client::get` is called
**Then** the result is `Err` whose chain mentions "parse ScorecardReport". Cache is not poisoned with the unparseable body.
