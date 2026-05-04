---
feature: web-viewer
status: accepted
spec: ../../specs/web-viewer.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Web viewer — Scenarios

Link: [`specs/web-viewer.md`](../../specs/web-viewer.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | / lists reports; /reports/{o}/{n} renders |
| Edge cases | 3 | empty cache; nonexistent report 404; static asset served |
| Security | 2 | localhost-only by default; /scans 405 without --allow-scan |

---

## Happy path

### S-001: GET / lists cached reports newest-first

**Given** the SQLite cache contains 2 reports for `acme/widget` and 1 for `octocat/Hello-World`
**When** the user hits `GET /`
**Then** the response is 200 HTML containing both repo names; `acme/widget` appears before `octocat/Hello-World` (or whichever is newest by `computed_at`).

### S-002: GET /reports/{owner}/{name} renders module cards

**Given** a cached report for `acme/widget` with 5 modules
**When** the user hits `GET /reports/acme/widget`
**Then** the response is 200 HTML containing `acme/widget`, the overall score, all 5 module names, and ≥3 evidence items.

---

## Edge cases

### S-101: empty cache → / renders with friendly empty-state

**Given** an empty cache
**When** the user hits `GET /`
**Then** the response is 200 with an empty-state message ("No reports yet. Run `repo-trust scan owner/repo` first.").

### S-102: nonexistent report → 404

**Given** no cached report for `ghost/ghost`
**When** the user hits `GET /reports/ghost/ghost`
**Then** the response is 404 with a clear message.

### S-103: static asset is served from rust-embed

**Given** the binary embeds `static/style.css`
**When** the user hits `GET /static/style.css`
**Then** the response is 200 `text/css`.

---

## Security

### S-501: default --bind is 127.0.0.1:8765

**Given** `repo-trust serve` invoked with no `--bind`
**When** the server starts
**Then** the bound address is `127.0.0.1:8765`; a `tracing::info!` event records this.

### S-502: POST /scans returns 405 without --allow-scan

**Given** the server started without `--allow-scan`
**When** the user submits `POST /scans` with `{repo: "acme/widget", mode: "standard"}`
**Then** the response is 405 Method Not Allowed; nothing happens server-side.
