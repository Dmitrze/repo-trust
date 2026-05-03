# Module Specifications

Per-module input/output contracts, ecosystem-aware adjustments, and edge cases. This document is the contract that module implementations must satisfy.

---

## Star Authenticity

### Inputs
```rust
struct StarsRawData {
    total_stars: u64,
    forks: u64,
    watchers: u64,
    repo_created_at: OffsetDateTime,
    stargazer_sample: Vec<StargazerProfile>,  // size depends on mode
    star_event_timeseries: Vec<DateTime<Utc>>,  // for lockstep z-score
}
```

### Outputs
```rust
struct StarsFeatures {
    total_stars: u64,
    fork_to_star_ratio: f64,
    watcher_to_star_ratio: f64,
    low_activity_share: f64,        // 0.0..1.0
    lockstep_z_score: f64,           // max daily z-score
    sample_size: usize,
    median_stargazer_account_age_days: f64,
}
```

### Ecosystem multipliers

| Primary language | Fork ratio multiplier | Watcher ratio multiplier |
| --- | --- | --- |
| TypeScript / JavaScript | 0.7 | 0.8 |
| Python | 1.0 | 1.0 |
| Go | 1.1 | 1.0 |
| Rust | 0.9 | 0.9 |
| Java / Kotlin | 1.0 | 1.0 |
| Other | 1.0 | 1.0 |

*(These multipliers adjust the healthy-range thresholds, not the score directly.)*

### Edge cases
- **Repo with < 50 stars:** module is auto-skipped; sample size too small to be meaningful.
- **Repo created < 30 days ago:** module is run with `Low confidence` and a caveat about youth.
- **GitHub API does not return `starred_at`** (REST endpoint): we fall back to GraphQL; if both fail, sub-signal 9 (star date == account creation date) is dropped and we lower confidence.
- **Sample size truncated by rate limit:** caveat surfaced; confidence reduced.

---

## Activity Health

### Inputs
```rust
struct ActivityRawData {
    commits_30d: Vec<CommitMeta>,
    commits_90d: Vec<CommitMeta>,
    commits_365d: Vec<CommitMeta>,
    commits_18m_monthly: Vec<u64>,        // 18 monthly bucket counts for variance
    issues_last_90d: Vec<IssueMeta>,      // first response timestamps
    prs_last_90d: Vec<PRMeta>,            // review timestamps
    releases: Vec<ReleaseMeta>,           // ordered by date desc
    contributors_last_90d: HashSet<String>,
}
```

### Outputs
```rust
struct ActivityFeatures {
    commits_last_30d: u64,
    commits_last_90d: u64,
    commits_last_365d: u64,
    days_since_last_commit: u64,
    days_since_last_release: Option<u64>,
    release_count_last_year: u64,
    median_issue_first_response_hours: Option<f64>,
    median_pr_review_hours: Option<f64>,
    active_contributors_last_90d: u64,
    commit_count_variance_18m: f64,
}
```

### Ecosystem-aware adjustments
- **"Long-stable utility" detection:** if `total_releases >= 3` AND `versioned_consistently == true` AND `dependents_count >= 1000`, we down-weight "days since last release" by 50%. A heavily-depended-on UUID library that hasn't released in 18 months is not unhealthy.
- **Archived repos:** module is skipped; the report explicitly states the repo is archived.

---

## Maintainer Health

### Inputs
```rust
struct MaintainerRawData {
    commits_365d_by_author: HashMap<String, u64>,
    pr_reviews_365d_by_reviewer: HashMap<String, u64>,
    contributors_first_180d: HashSet<String>,
    contributors_second_180d: HashSet<String>,
    issue_responses_by_user: HashMap<String, Vec<DurationHours>>,
    has_codeowners: bool,
    has_maintainers_md: bool,
    has_governance_doc: bool,
}
```

### Outputs
```rust
struct MaintainerFeatures {
    active_maintainers_last_year: u64,
    commit_gini: f64,
    review_gini: f64,
    bus_factor_proxy: u64,
    contributor_retention_rate: f64,
    median_maintainer_response_hours: Option<f64>,
    has_codeowners: bool,
    has_governance_doc: bool,
}
```

### Edge cases
- **Solo-maintainer projects:** flagged in evidence; not penalized into "High Risk" alone. Many excellent OSS projects are solo-maintained.
- **Bot commits:** filtered by GitHub user `type == "Bot"` and known bot username patterns (`*-bot`, `dependabot[bot]`, `renovate[bot]`).

---

## Adoption Signals

### Inputs (all federated)
```rust
struct AdoptionRawData {
    github_dependents: Option<u64>,
    deps_dev_packages: Vec<DepsDevPackage>,    // packages this repo publishes
    weekly_downloads_per_system: HashMap<String, u64>,
    awesome_list_mentions: Vec<String>,        // list URLs
    has_readme: bool,
    readme_word_count: usize,
    has_docs_folder: bool,
    has_examples_folder: bool,
}
```

### Outputs
```rust
struct AdoptionFeatures {
    github_dependents: Option<u64>,
    weekly_downloads: Option<u64>,            // sum across systems
    package_systems: Vec<String>,             // npm, crates, pypi, ...
    awesome_list_mentions: u64,
    doc_maturity_score: f64,                  // 0.0..1.0
}
```

### Edge cases
- **No published package:** `weekly_downloads = None`; module reports Medium confidence with a caveat.
- **deps.dev outage:** confidence drops to Low for this module; we use only GitHub-side signals.

---

## Security & Readiness

### Inputs
```rust
struct SecurityRawData {
    scorecard_score: Option<f64>,             // from scorecard.dev API
    scorecard_check_results: Vec<ScorecardCheck>,
    osv_advisories: Vec<OsvAdvisory>,
    files_present: HashSet<String>,           // SECURITY.md, LICENSE, ...
    workflow_files: Vec<String>,              // .github/workflows/*
    releases: Vec<ReleaseMeta>,               // for semver consistency
}
```

### Outputs
```rust
struct SecurityFeatures {
    scorecard_score: Option<f64>,             // 0.0..10.0 from Scorecard
    scorecard_checks_failed: Vec<String>,
    osv_open_advisories: u64,
    has_security_md: bool,
    has_contributing_md: bool,
    has_code_of_conduct: bool,
    has_license: bool,
    has_codeowners: bool,
    has_ci_workflow: bool,
    semver_consistent: bool,
}
```

### Federation policy
- Scorecard score < 30 days old: weight 0.40 of this module.
- Scorecard score 30–90 days old: weight 0.30; lower confidence.
- Scorecard score > 90 days old or absent: ignored; module relies on document presence + CI signals only with Low confidence.
