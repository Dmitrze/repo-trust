# API Notes

Quirks, rate limits, and gotchas for the upstream APIs we federate.

---

## GitHub REST + GraphQL

### Rate limits
- **Unauthenticated:** 60 requests / hour. Useless for anything beyond a single quick scan.
- **Authenticated (PAT):** 5,000 requests / hour. The default for individual users.
- **GitHub App / OAuth installation:** higher limits available.
- **Search API:** separate, stricter limit (30 requests / minute).

### Critical mitigations
1. **ETag-aware fetching.** Every request sends `If-None-Match: <cached-etag>`. A `304 Not Modified` response **does not consume rate-limit budget** for authenticated requests — this is the single most important optimization.
2. **Conditional pagination.** For large stargazer sets, we paginate via REST `Link` headers; each page is independently cached with its own ETag.
3. **Rate-limit headers.** Every response includes `X-RateLimit-Remaining` and `X-RateLimit-Reset`. We pause when remaining < 100 and resume after the reset window.

### Gotchas
- The REST `GET /repos/{owner}/{repo}/stargazers` endpoint returns a list with `starred_at` only if you set `Accept: application/vnd.github.star+json`. Without it, you only get user records, not star timestamps. Easy to miss.
- The `dependents` count is **not** in the REST API. It only appears on the HTML `Used by` page. We currently scrape it from the page; deps.dev provides a more reliable cross-reference where available.
- GitHub deduplicates renames (`old/repo → new/repo`) via HTTP 301. We follow once and use the canonical name in the report.
- The `contributors` endpoint returns up to 500 entries. For repos with > 500 contributors, the long tail is invisible to this endpoint.

## deps.dev

### Endpoint overview
- Base: `https://api.deps.dev/v3/`
- `/systems/{system}/packages/{name}/versions/{version}` — package metadata, OSV advisories, OpenSSF Scorecard score (federated through deps.dev), license, dependencies.
- `/projects/{project_url}` — maps a repo URL to one or more published packages.

### Rate limits
Not publicly documented as of May 2026. We use generous defaults (1 req/s, with backoff on `429`).

### Gotchas
- Repo-to-package mapping is not always 1:1. A monorepo (e.g. `microsoft/TypeScript-Node-Starter`) may have zero or many packages.
- Some ecosystems' download counts are not present (e.g. private package registries are unsupported).

## scorecard.dev

### Endpoint
`GET https://api.scorecard.dev/projects/github.com/{owner}/{repo}` returns the JSON Scorecard report. Updated weekly via Scorecard's BigQuery batch run.

### Caveats
- Coverage: only the top ≈1M repos by activity are scored. Long-tail repos return 404.
- Freshness: a 404 may simply mean "not yet scored," not "doesn't exist."

## OSV.dev

### Endpoint
`POST https://api.osv.dev/v1/query` with body `{"package": {"name": ..., "ecosystem": ...}, "version": ...}` returns the list of advisories affecting that package version.

### Caveats
- Some ecosystems (e.g. private NuGet feeds) are not covered.
- Withdrawn advisories are still returned by default; we filter them client-side.

## General federation policy

- Aggressive caching with TTLs documented in `docs/architecture.md` §6.3.
- Graceful degradation: if any one upstream is unavailable, the affected module reports Low confidence but does not fail the run.
- Per-upstream client modules in `src/api/` keep adapter logic isolated so a future API change is a localized fix.
