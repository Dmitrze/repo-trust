---
feature: deps-dev-client
status: accepted
spec: ../../specs/deps-dev-client.md
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
---

# deps.dev client — Scenarios

Link: [`specs/deps-dev-client.md`](../../specs/deps-dev-client.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | project_packages 200; package 200 |
| Edge cases | 2 | 404 → empty list; deterministic sort |
| Failure modes | 1 | 5xx upstream |

---

## Happy path

### S-001: project_packages returns sorted Vec

**Given** wiremock returns `200 OK` body `{"packages":[{"system":"GO","name":"github.com/prometheus/prometheus"}]}` for `GET /v3/projects/github.com/prometheus/prometheus/packages`
**When** `Client::project_packages("prometheus", "prometheus")` is called
**Then** the result is `Ok(vec)` with `vec[0].system == "GO"` and `vec[0].name == "github.com/prometheus/prometheus"`.

### S-002: package returns weekly downloads

**Given** wiremock returns `200 OK` body `{"system":"NPM","name":"lodash","weeklyDownloads":"50000000","latestVersion":"4.17.21"}` for `GET /v3/systems/NPM/packages/lodash`
**When** `Client::package("NPM", "lodash")` is called
**Then** the result is `Ok(PackageInfo { weekly_downloads: Some(50_000_000), latest_version: Some("4.17.21".into()), .. })`.

---

## Edge cases

### S-101: 404 on project endpoint returns empty Vec

**Given** wiremock returns `404 Not Found` for `GET /v3/projects/github.com/ghost/ghost/packages`
**When** `Client::project_packages("ghost", "ghost")` is called
**Then** the result is `Ok(Vec::new())` — not an error. Adoption module surfaces this as `no_packages` Neutral evidence.

### S-102: deterministic sort by (system, name)

**Given** wiremock returns 3 packages in order `[(NPM, b), (GO, a), (NPM, a)]`
**When** `Client::project_packages` is called
**Then** the returned Vec is `[(GO, a), (NPM, a), (NPM, b)]` — sorted alphabetically on `(system, name)`.

---

## Failure modes

### S-201: 5xx returns Err

**Given** wiremock returns `503 Service Unavailable`
**When** `Client::project_packages` is called
**Then** the result is `Err`; CLI maps to exit code 7 per architecture §8.
