//! Snapshot tests for deterministic JSON output (see ADR-0007).
//!
//! Once the scan pipeline is wired, this file replays a fixture of cached
//! API responses through `wiremock` and snapshots the resulting JSON via
//! `insta`. Snapshot review is part of the PR process; CI fails on drift.
//!
//! For now this file is a stub that documents the intended approach.

#[test]
#[ignore = "snapshot pipeline not yet wired; placeholder until Phase 1 lands"]
fn placeholder_for_golden_output_test() {
    // TODO once scan is wired:
    //
    //   let mock_server = wiremock::MockServer::start().await;
    //   register_fixtures(&mock_server, "tests/fixtures/octocat-Hello-World/").await;
    //   let output = run_scan_against(&mock_server, "octocat/Hello-World").await;
    //   let report: TrustReport = serde_json::from_str(&output)?;
    //   insta::assert_json_snapshot!(report, {
    //       ".snapshot_at" => "[timestamp]",
    //       ".runtime_seconds" => "[runtime]",
    //   });
}
