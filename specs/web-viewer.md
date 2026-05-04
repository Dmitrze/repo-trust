---
feature: web-viewer
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/web-viewer.md"]
related_runbooks: []
related_docs: ["docs/architecture.md#12-localhost-web-viewer"]
---

# Localhost web viewer (`repo-trust serve`)

> `axum` app on `localhost:8765` that renders cached `TrustReport`s as HTML. Single-binary preserved via `rust-embed` for static assets and compile-time `askama` templates. Localhost-only by default; `--bind 0.0.0.0` documented as risky per `architecture.md` §12.

---

## 1. Goal

`repo-trust serve` starts an HTTP server that lets a user browse cached reports, see a single report's full module breakdown, and trigger a re-scan from the UI. Same JSON exposed via `GET /api/reports/{owner}/{name}` for downstream tooling.

We know it works when: after running `repo-trust scan acme/widget`, hitting `http://localhost:8765/` lists the report; clicking through to `/reports/acme/widget` shows the module cards + evidence; `GET /api/reports/acme/widget` returns the cached JSON byte-for-byte.

---

## 2. Non-functional requirements

- **Single-binary preserved:** all templates compile-time via `askama` (already in `[features] web` dep set), all static assets embedded via `rust-embed` (also already in deps).
- **Localhost only by default:** `--bind 127.0.0.1:8765` default; `--bind 0.0.0.0` requires explicit flag and emits `tracing::warn!` on startup.
- **Read-only by default:** `POST /scans` is implemented but only when `--allow-scan` is also set; otherwise that route returns 405. Documented as a guardrail since "anyone with localhost access can trigger arbitrary scans" (browser attacks via DNS rebinding etc.).
- **No new runtime crates:** `axum`, `tower-http`, `askama`, `rust-embed` already in `[features] web` set.
- **No telemetry:** zero outbound calls beyond the existing federated APIs.

---

## 3. Boundaries

### In scope (Day 4)
- `src/cli/serve.rs::execute(args)` replaces the `bail!` stub. Builds `axum::Router`, binds to the configured address, serves until SIGINT.
- `src/web/mod.rs` and `src/web/routes.rs` — Router definition + handlers:
  - `GET /` → list of all reports (newest first by `computed_at`) — render via `askama` template.
  - `GET /reports/{owner}/{name}` → single report view (module cards, evidence list, caveats).
  - `GET /api/reports/{owner}/{name}` → JSON (latest report from cache).
  - `POST /scans` (when `--allow-scan`) → triggers `cli::scan::execute` with the body's `repo` + `mode`; redirects to the rendered report.
  - `GET /static/*` → embedded CSS/JS via `rust-embed`.
- `src/web/templates/` — askama templates: `base.html`, `index.html`, `report.html`.
- `src/web/static/` — minimal CSS (one file, ≤200 lines), no JS required for v1.
- `--bind <addr>` and `--allow-scan` CLI flags.
- ≥4 unit tests on individual handlers + 1 wiremock-driven integration test.

### Out of scope (Day 4)
- Authentication (we are localhost-only).
- WebSockets / live re-scan progress (Day 5+).
- Activity-timeline charts (Day 5+).
- Multi-user persistent sessions.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. User runs `repo-trust scan acme/widget --mode standard` (Day 3 path); report cached in SQLite.
2. User runs `repo-trust serve`; logs `Serving on http://127.0.0.1:8765`.
3. User opens `http://localhost:8765/`; sees a list with `acme/widget — Score 73 — Good — 12s ago`.
4. User clicks; sees the full module breakdown rendered from the cached `TrustReport` JSON.
5. User does `curl http://localhost:8765/api/reports/acme/widget`; receives the same cached JSON byte-for-byte.
6. (With `--allow-scan`) User submits the form on `/`; backend triggers a new scan; redirects to the rendered new report.

---

## 6. Architecture sketch

```
[ axum::Router ]
  / ───────────────────► handlers::index(State<Cache>) → askama::index.html
  /reports/{o}/{n} ─────► handlers::report(State<Cache>, Path) → askama::report.html
  /api/reports/{o}/{n} ─► handlers::report_json(State<Cache>, Path) → JSON
  /static/* ────────────► tower_http::services::ServeDir over rust-embed
  /scans (POST) ────────► handlers::scan(State<Cache + Clients>, Form) → 303 redirect
```

State carries `Arc<storage::Cache>` and (for /scans) the API client bundle.

---

## 7. Closed loop

- **Goal metric:** integration test hits all 4 routes and asserts response bodies; manual visual check on Day 5.
- **Where it lives:** CI; MemPalace `ops/distribution`.
- **Read by:** users who prefer GUI to terminal.
- **Improvement path:** Day 5+ may add charts via `chart.js` (loaded from `static/`) if the value is clear.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/web-viewer.md` lists ≥5 scenarios.
- [ ] `src/cli/serve.rs::execute` replaces the `bail!` stub.
- [ ] Templates compile under `cargo build --features web`.
- [ ] Static assets embedded via `rust-embed`.
- [ ] `--bind` defaults to `127.0.0.1:8765`; `--bind 0.0.0.0` emits a warn-level log.
- [ ] `POST /scans` returns 405 unless `--allow-scan` set.
- [ ] ≥4 handler unit tests + 1 integration test using `axum::Router::oneshot`.
- [ ] CHANGELOG entry.
- [ ] `cargo build --no-default-features` (without `web` feature) still succeeds and produces a binary without `serve`.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-04 — should `/scans` be enabled by default? — No, behind `--allow-scan` to mitigate DNS-rebinding browser attacks targeting localhost.

---

## 11. References

- `docs/architecture.md` §12 — Localhost web viewer.
- `axum` 0.7 docs.
- `askama` 0.12 docs.
- `rust-embed` 8.x docs.
