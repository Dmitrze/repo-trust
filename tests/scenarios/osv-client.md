---
feature: osv-client
status: accepted
spec: ../../specs/osv-client.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# OSV.dev client — Scenarios

Link: [`specs/osv-client.md`](../../specs/osv-client.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | empty response; populated response |
| Edge cases | 2 | withdrawn-filter; deterministic sort |
| Failure modes | 1 | 5xx upstream |

---

## Happy path

### S-001: empty response returns empty Vec

**Given** wiremock returns `200 OK` body `{"vulns":[]}` for `POST /v1/query`
**When** `Client::query({name:"safe-pkg", ecosystem:"npm", version:"1.0.0"})` is called
**Then** the result is `Ok(Vec::new())`.

### S-002: populated response parses + filters withdrawn

**Given** wiremock returns `200 OK` with two advisories: `{id:"GHSA-aaaa", withdrawn:null}` and `{id:"GHSA-bbbb", withdrawn:"2025-06-01T00:00:00Z"}`
**When** `Client::query` is called
**Then** the result is a `Vec` of length 1 containing only `GHSA-aaaa`.

---

## Edge cases

### S-101: deterministic sort by id

**Given** wiremock returns 3 non-withdrawn advisories in order `[GHSA-cccc, GHSA-aaaa, GHSA-bbbb]`
**When** `Client::query` is called
**Then** the returned `Vec` is sorted alphabetically: `[GHSA-aaaa, GHSA-bbbb, GHSA-cccc]`.

### S-102: cache hit serves without network

**Given** the cache contains a fresh entry for `osv:npm:lodash:4.17.20`
**When** `Client::query({name:"lodash", ecosystem:"npm", version:"4.17.20"})` is called
**Then** the wiremock mock receives zero requests; the result matches the cached parsed advisories.

---

## Failure modes

### S-201: 5xx returns Err

**Given** wiremock returns `503 Service Unavailable`
**When** `Client::query` is called
**Then** the result is `Err`; CLI maps to exit code 7 per architecture §8.
