# Test fixtures

This directory holds cached API responses used by the snapshot tests in
`tests/golden_outputs.rs`. Each subdirectory corresponds to a single
fixture repository:

```
tests/fixtures/
  octocat-Hello-World/
    github_repo.json           # GET /repos/octocat/Hello-World
    github_stargazers_p1.json  # first page of stargazers
    deps_dev.json              # deps.dev project endpoint
    scorecard.json             # scorecard.dev response
    osv.json                   # OSV query response
```

Fixtures are loaded by the test harness into a `wiremock::MockServer`.
The scan is then executed against the mock server's URL, and the
resulting `TrustReport` is snapshotted via `insta`.

## Updating fixtures

Do not edit fixture JSON by hand unless the upstream API genuinely
changed. Use `scripts/update_fixtures.rs` (planned) to regenerate from
live APIs against a known repo.
