-- Initial Repo Trust cache schema.
-- See docs/architecture.md §6.1 and specs/cache-layer.md.

-- Raw upstream API responses, keyed by request URL fragment.
-- Values like "github:repos:octocat/Hello-World:metadata".
CREATE TABLE IF NOT EXISTS api_cache (
    cache_key   TEXT PRIMARY KEY,
    etag        TEXT,
    fetched_at  TEXT NOT NULL,    -- ISO 8601 UTC
    expires_at  TEXT,             -- ISO 8601 UTC; NULL means no soft TTL
    body_json   TEXT NOT NULL,
    schema_ver  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_cache_fetched ON api_cache(fetched_at);

-- Per-(repo, module, scoring_ver) feature snapshots, idempotent upsert.
CREATE TABLE IF NOT EXISTS features (
    repo        TEXT NOT NULL,
    module      TEXT NOT NULL,
    scoring_ver TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    body_json   TEXT NOT NULL,
    PRIMARY KEY (repo, module, scoring_ver)
);

-- History of every report computed; latest one wins via ORDER BY computed_at DESC.
CREATE TABLE IF NOT EXISTS reports (
    repo        TEXT NOT NULL,
    mode        TEXT NOT NULL,
    scoring_ver TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    body_json   TEXT NOT NULL,
    PRIMARY KEY (repo, mode, scoring_ver, computed_at)
);

CREATE INDEX IF NOT EXISTS idx_reports_repo ON reports(repo);
