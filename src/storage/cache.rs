//! Cache facade — r2d2-pooled SQLite connection with ETag-aware CRUD.
//!
//! See `docs/architecture.md` §6 and `specs/cache-layer.md`.
//!
//! The facade exposes three logical caches:
//! - **`api_cache`** — raw upstream API responses keyed by request URL
//!   fragment, with ETags and TTLs.
//! - **`features`** — computed feature snapshots per `(repo, module,
//!   scoring_ver)`.
//! - **`reports`** — history of every `TrustReport` produced, latest-first.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::migrations;

/// Schema version embedded in every `api_cache` row. Bumped when the cached
/// body structure changes for a given upstream key class.
const SCHEMA_VER: &str = "1";

/// Pooled handle to the local SQLite cache. Cheap to clone (Arc inside r2d2).
#[derive(Debug, Clone)]
pub struct Cache {
    pool: Pool<SqliteConnectionManager>,
    path: PathBuf,
}

/// One row from `api_cache`, deserialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    pub etag: Option<String>,
    pub body: Vec<u8>,
    #[serde(with = "crate::utils::time::iso8601")]
    pub fetched_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub schema_ver: String,
}

impl CachedEntry {
    /// Returns `true` when the entry has passed its `expires_at` (or never had
    /// one). Callers use this to decide whether to send a conditional request.
    #[must_use]
    pub fn is_stale(&self) -> bool {
        match self.expires_at {
            Some(exp) => exp < OffsetDateTime::now_utc(),
            None => true,
        }
    }
}

/// Aggregate counts surfaced by `repo-trust cache info`.
#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub path: PathBuf,
    pub api_rows: u64,
    pub feature_rows: u64,
    pub report_rows: u64,
    pub bytes_on_disk: u64,
}

/// One row per `(repo, mode, scoring_ver)` returned by
/// [`Cache::list_all_reports`]. The web viewer's index page renders these.
#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub repo: String,
    pub mode: String,
    pub scoring_ver: String,
    pub computed_at: OffsetDateTime,
}

impl Cache {
    /// Open or create the cache at `path`. Runs all pending migrations and
    /// (on Unix) tightens the file permissions to 0600.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create cache parent dir {parent:?}"))?;
        }
        let manager = SqliteConnectionManager::file(&path);
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .with_context(|| format!("opening cache pool at {path:?}"))?;

        // Run migrations on a single dedicated connection.
        {
            let mut conn = pool.get().context("checkout connection for migration")?;
            migrations::migrations()
                .to_latest(&mut conn)
                .context("running cache migrations")?;
        }

        // Lock down perms on Unix; no-op on Windows.
        Self::tighten_permissions(&path)?;

        Ok(Self { pool, path })
    }

    /// Path to the on-disk SQLite file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Look up an `api_cache` entry by its key.
    pub fn get(&self, key: &str) -> Result<Option<CachedEntry>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT etag, fetched_at, expires_at, body_json, schema_ver
                 FROM api_cache
                 WHERE cache_key = ?1",
                params![key],
                |row| {
                    let etag: Option<String> = row.get(0)?;
                    let fetched_at: String = row.get(1)?;
                    let expires_at: Option<String> = row.get(2)?;
                    let body: String = row.get(3)?;
                    let schema_ver: String = row.get(4)?;
                    Ok((etag, fetched_at, expires_at, body, schema_ver))
                },
            )
            .optional()?;
        let Some((etag, fetched_at, expires_at, body, schema_ver)) = row else {
            return Ok(None);
        };
        Ok(Some(CachedEntry {
            etag,
            body: body.into_bytes(),
            fetched_at: parse_iso(&fetched_at)?,
            expires_at: match expires_at {
                Some(s) => Some(parse_iso(&s)?),
                None => None,
            },
            schema_ver,
        }))
    }

    /// Insert or replace an `api_cache` entry. `body` must be valid UTF-8
    /// (we store JSON only, never binary blobs).
    pub fn put(&self, key: &str, etag: Option<&str>, body: &[u8], ttl: Duration) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        let expires_at = now + ttl;
        let body = std::str::from_utf8(body).context("cache body must be UTF-8 JSON")?;
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO api_cache (cache_key, etag, fetched_at, expires_at, body_json, schema_ver)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(cache_key) DO UPDATE SET
                etag        = excluded.etag,
                fetched_at  = excluded.fetched_at,
                expires_at  = excluded.expires_at,
                body_json   = excluded.body_json,
                schema_ver  = excluded.schema_ver",
            params![
                key,
                etag,
                format_iso(now),
                format_iso(expires_at),
                body,
                SCHEMA_VER,
            ],
        )?;
        Ok(())
    }

    /// Update only the `fetched_at` column for the given key. Called after
    /// a successful `304 Not Modified` response.
    pub fn touch(&self, key: &str) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE api_cache SET fetched_at = ?1 WHERE cache_key = ?2",
            params![format_iso(now), key],
        )?;
        Ok(())
    }

    /// Remove a single `api_cache` entry by exact key.
    pub fn delete_by_key(&self, key: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM api_cache WHERE cache_key = ?1", params![key])?;
        Ok(())
    }

    /// Remove every `api_cache` entry whose key is prefixed by
    /// `github:repos:{repo}:`. Used by `--refresh`.
    pub fn delete_by_repo(&self, repo: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let prefix = format!("github:repos:{repo}:%");
        let n = conn.execute(
            "DELETE FROM api_cache WHERE cache_key LIKE ?1",
            params![prefix],
        )?;
        Ok(n)
    }

    /// Remove all rows from `api_cache`. Returns the number of rows
    /// deleted.
    pub fn clear_api_cache(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute("DELETE FROM api_cache", [])?;
        Ok(n)
    }

    /// Remove all rows from every cache table. Returns
    /// `(api_cache_rows, features_rows, reports_rows)`.
    pub fn clear_all(&self) -> Result<(usize, usize, usize)> {
        let conn = self.pool.get()?;
        let api = conn.execute("DELETE FROM api_cache", [])?;
        let features = conn.execute("DELETE FROM features", [])?;
        let reports = conn.execute("DELETE FROM reports", [])?;
        Ok((api, features, reports))
    }

    /// Remove `api_cache` rows whose `expires_at` is in the past. Returns
    /// the number of rows deleted. Used by `cache prune`.
    pub fn prune_expired(&self) -> Result<usize> {
        let now = OffsetDateTime::now_utc();
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM api_cache WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![format_iso(now)],
        )?;
        Ok(n)
    }

    /// Aggregate row counts and on-disk size, surfaced by `cache info`.
    pub fn info(&self) -> Result<CacheInfo> {
        let conn = self.pool.get()?;
        let api_rows = count(&conn, "api_cache")?;
        let feature_rows = count(&conn, "features")?;
        let report_rows = count(&conn, "reports")?;
        let bytes_on_disk = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        Ok(CacheInfo {
            path: self.path.clone(),
            api_rows,
            feature_rows,
            report_rows,
            bytes_on_disk,
        })
    }

    // ─── Feature snapshots ────────────────────────────────────────────────

    /// Upsert a feature snapshot.
    pub fn put_feature(
        &self,
        repo: &str,
        module: &str,
        scoring_ver: &str,
        body: &[u8],
    ) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        let body = std::str::from_utf8(body).context("feature body must be UTF-8 JSON")?;
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO features (repo, module, scoring_ver, computed_at, body_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(repo, module, scoring_ver) DO UPDATE SET
                computed_at = excluded.computed_at,
                body_json   = excluded.body_json",
            params![repo, module, scoring_ver, format_iso(now), body],
        )?;
        Ok(())
    }

    /// Look up a feature snapshot by its full key.
    pub fn get_feature(
        &self,
        repo: &str,
        module: &str,
        scoring_ver: &str,
    ) -> Result<Option<Vec<u8>>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT body_json FROM features
                 WHERE repo = ?1 AND module = ?2 AND scoring_ver = ?3",
                params![repo, module, scoring_ver],
                |r| {
                    let s: String = r.get(0)?;
                    Ok(s.into_bytes())
                },
            )
            .optional()?;
        Ok(row)
    }

    // ─── Reports ──────────────────────────────────────────────────────────

    /// Record a freshly computed report. Append-only.
    pub fn put_report(&self, repo: &str, mode: &str, scoring_ver: &str, body: &[u8]) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        let body = std::str::from_utf8(body).context("report body must be UTF-8 JSON")?;
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO reports (repo, mode, scoring_ver, computed_at, body_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo, mode, scoring_ver, format_iso(now), body],
        )?;
        Ok(())
    }

    /// Most recent report for `(repo, mode, scoring_ver)`.
    pub fn latest_report(
        &self,
        repo: &str,
        mode: &str,
        scoring_ver: &str,
    ) -> Result<Option<Vec<u8>>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT body_json FROM reports
                 WHERE repo = ?1 AND mode = ?2 AND scoring_ver = ?3
                 ORDER BY computed_at DESC LIMIT 1",
                params![repo, mode, scoring_ver],
                |r| {
                    let s: String = r.get(0)?;
                    Ok(s.into_bytes())
                },
            )
            .optional()?;
        Ok(row)
    }

    /// One row per `(repo, mode, scoring_ver)` — the most recent
    /// `computed_at` for each tuple. Sorted newest-first. Used by the web
    /// viewer's index page.
    pub fn list_all_reports(&self) -> Result<Vec<ReportSummary>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT repo, mode, scoring_ver, MAX(computed_at) AS latest
             FROM reports
             GROUP BY repo, mode, scoring_ver
             ORDER BY latest DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let repo: String = r.get(0)?;
                let mode: String = r.get(1)?;
                let scoring_ver: String = r.get(2)?;
                let computed_at: String = r.get(3)?;
                Ok((repo, mode, scoring_ver, computed_at))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for (repo, mode, scoring_ver, computed_at) in rows {
            out.push(ReportSummary {
                repo,
                mode,
                scoring_ver,
                computed_at: parse_iso(&computed_at)?,
            });
        }
        Ok(out)
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    #[cfg(unix)]
    fn tighten_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .with_context(|| format!("stat cache file {path:?}"))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("chmod 0600 cache file {path:?}"))?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn tighten_permissions(_path: &Path) -> Result<()> {
        Ok(())
    }
}

fn count(conn: &rusqlite::Connection, table: &str) -> Result<u64> {
    let q = format!("SELECT COUNT(*) FROM {table}");
    let n: i64 = conn.query_row(&q, [], |r| r.get(0))?;
    Ok(u64::try_from(n).unwrap_or(0))
}

fn parse_iso(s: &str) -> Result<OffsetDateTime> {
    use time::format_description::well_known::Iso8601;
    OffsetDateTime::parse(s, &Iso8601::DEFAULT)
        .with_context(|| format!("parse ISO 8601 timestamp from cache: {s}"))
}

fn format_iso(t: OffsetDateTime) -> String {
    use time::format_description::well_known::Iso8601;
    t.format(&Iso8601::DEFAULT)
        .expect("OffsetDateTime always formats to ISO 8601")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn fresh_cache() -> (Cache, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        let cache = Cache::open(&path).expect("open cache");
        (cache, dir)
    }

    #[test]
    fn put_then_get_round_trip() {
        let (cache, _dir) = fresh_cache();
        cache
            .put(
                "github:repos:octocat/Hello-World:metadata",
                Some("etag-1"),
                br#"{"full_name":"octocat/Hello-World"}"#,
                Duration::from_secs(60),
            )
            .unwrap();
        let entry = cache
            .get("github:repos:octocat/Hello-World:metadata")
            .unwrap()
            .expect("entry should exist");
        assert_eq!(entry.etag.as_deref(), Some("etag-1"));
        assert_eq!(
            entry.body,
            br#"{"full_name":"octocat/Hello-World"}"#.to_vec()
        );
        assert!(!entry.is_stale(), "entry should not be stale immediately");
    }

    #[test]
    fn get_missing_key_returns_none() {
        let (cache, _dir) = fresh_cache();
        assert!(cache.get("never-stored").unwrap().is_none());
    }

    #[test]
    fn touch_advances_fetched_at() {
        let (cache, _dir) = fresh_cache();
        cache
            .put("k", Some("e"), b"{}", Duration::from_secs(60))
            .unwrap();
        let before = cache.get("k").unwrap().unwrap().fetched_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.touch("k").unwrap();
        let after = cache.get("k").unwrap().unwrap().fetched_at;
        assert!(after > before, "fetched_at should advance after touch");
    }

    #[test]
    fn put_replaces_existing_etag_and_body() {
        let (cache, _dir) = fresh_cache();
        cache
            .put("k", Some("e1"), b"{\"v\":1}", Duration::from_secs(60))
            .unwrap();
        cache
            .put("k", Some("e2"), b"{\"v\":2}", Duration::from_secs(60))
            .unwrap();
        let entry = cache.get("k").unwrap().unwrap();
        assert_eq!(entry.etag.as_deref(), Some("e2"));
        assert_eq!(entry.body, br#"{"v":2}"#.to_vec());
    }

    #[test]
    fn ttl_expired_entry_is_stale() {
        let (cache, _dir) = fresh_cache();
        // Zero TTL ⇒ expires_at == now ⇒ stale on next read.
        cache.put("k", None, b"{}", Duration::from_secs(0)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let entry = cache.get("k").unwrap().unwrap();
        assert!(entry.is_stale(), "zero-TTL entry should be stale");
    }

    #[test]
    fn delete_by_repo_removes_only_matching_keys() {
        let (cache, _dir) = fresh_cache();
        cache
            .put(
                "github:repos:octocat/Hello-World:metadata",
                None,
                b"{}",
                Duration::from_secs(60),
            )
            .unwrap();
        cache
            .put(
                "github:repos:octocat/Hello-World:commits",
                None,
                b"{}",
                Duration::from_secs(60),
            )
            .unwrap();
        cache
            .put(
                "github:repos:other/repo:metadata",
                None,
                b"{}",
                Duration::from_secs(60),
            )
            .unwrap();
        let n = cache.delete_by_repo("octocat/Hello-World").unwrap();
        assert_eq!(n, 2);
        assert!(cache
            .get("github:repos:octocat/Hello-World:metadata")
            .unwrap()
            .is_none());
        assert!(cache
            .get("github:repos:other/repo:metadata")
            .unwrap()
            .is_some());
    }

    #[test]
    fn put_and_get_feature_snapshot() {
        let (cache, _dir) = fresh_cache();
        cache
            .put_feature("octocat/Hello-World", "activity", "1.0.0", b"{\"x\":1}")
            .unwrap();
        let body = cache
            .get_feature("octocat/Hello-World", "activity", "1.0.0")
            .unwrap()
            .expect("feature should exist");
        assert_eq!(body, br#"{"x":1}"#.to_vec());
    }

    #[test]
    fn reports_history_returns_latest() {
        let (cache, _dir) = fresh_cache();
        cache
            .put_report("r", "standard", "1.0.0", b"{\"v\":1}")
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache
            .put_report("r", "standard", "1.0.0", b"{\"v\":2}")
            .unwrap();
        let body = cache
            .latest_report("r", "standard", "1.0.0")
            .unwrap()
            .unwrap();
        assert_eq!(body, br#"{"v":2}"#.to_vec(), "latest write should win");
    }

    #[test]
    fn list_all_reports_groups_by_tuple_and_orders_newest_first() {
        let (cache, _dir) = fresh_cache();
        // Two writes for acme/widget — only the latest computed_at should
        // surface for that tuple.
        cache
            .put_report("acme/widget", "standard", "1.0.0", b"{\"v\":1}")
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache
            .put_report("acme/widget", "standard", "1.0.0", b"{\"v\":2}")
            .unwrap();
        // Older write for octocat/Hello-World.
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache
            .put_report("octocat/Hello-World", "standard", "1.0.0", b"{}")
            .unwrap();
        let rows = cache.list_all_reports().unwrap();
        assert_eq!(rows.len(), 2, "tuple-grouped, not row-counted");
        // Newest first.
        assert_eq!(rows[0].repo, "octocat/Hello-World");
        assert_eq!(rows[1].repo, "acme/widget");
    }

    #[test]
    fn info_counts_rows() {
        let (cache, _dir) = fresh_cache();
        cache
            .put("k1", None, b"{}", Duration::from_secs(60))
            .unwrap();
        cache.put_feature("r", "activity", "1.0.0", b"{}").unwrap();
        cache.put_report("r", "standard", "1.0.0", b"{}").unwrap();
        let info = cache.info().unwrap();
        assert_eq!(info.api_rows, 1);
        assert_eq!(info.feature_rows, 1);
        assert_eq!(info.report_rows, 1);
        assert!(info.bytes_on_disk > 0);
    }

    #[test]
    fn open_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c/cache.db");
        let cache = Cache::open(&nested).unwrap();
        assert!(nested.exists(), "cache file should be created");
        assert_eq!(cache.path(), &nested);
    }

    #[cfg(unix)]
    #[test]
    fn cache_file_is_chmod_0600() {
        use std::os::unix::fs::PermissionsExt;
        let (cache, _dir) = fresh_cache();
        let mode = std::fs::metadata(cache.path())
            .unwrap()
            .permissions()
            .mode();
        // Mask off the file-type bits; only compare permission bits.
        assert_eq!(
            mode & 0o777,
            0o600,
            "cache file should be 0600, got {:o}",
            mode & 0o777
        );
    }

    #[test]
    fn clear_api_cache_removes_only_api_rows() {
        let (cache, _dir) = fresh_cache();
        cache
            .put("k1", None, b"{}", Duration::from_secs(60))
            .unwrap();
        cache
            .put("k2", None, b"{}", Duration::from_secs(60))
            .unwrap();
        cache.put_feature("r", "activity", "1.0.0", b"{}").unwrap();
        let n = cache.clear_api_cache().unwrap();
        assert_eq!(n, 2);
        assert_eq!(cache.info().unwrap().api_rows, 0);
        assert_eq!(cache.info().unwrap().feature_rows, 1);
    }

    #[test]
    fn clear_all_removes_every_table() {
        let (cache, _dir) = fresh_cache();
        cache
            .put("k", None, b"{}", Duration::from_secs(60))
            .unwrap();
        cache.put_feature("r", "activity", "1.0.0", b"{}").unwrap();
        cache.put_report("r", "standard", "1.0.0", b"{}").unwrap();
        let (api, features, reports) = cache.clear_all().unwrap();
        assert_eq!(api, 1);
        assert_eq!(features, 1);
        assert_eq!(reports, 1);
        let info = cache.info().unwrap();
        assert_eq!(info.api_rows, 0);
        assert_eq!(info.feature_rows, 0);
        assert_eq!(info.report_rows, 0);
    }

    #[test]
    fn prune_expired_removes_only_stale_rows() {
        let (cache, _dir) = fresh_cache();
        // Fresh entry (1h TTL).
        cache
            .put("fresh", None, b"{}", Duration::from_secs(3600))
            .unwrap();
        // Expired entry (zero TTL).
        cache
            .put("stale", None, b"{}", Duration::from_secs(0))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let n = cache.prune_expired().unwrap();
        assert_eq!(n, 1);
        assert!(cache.get("fresh").unwrap().is_some());
        assert!(cache.get("stale").unwrap().is_none());
    }

    #[test]
    fn reopening_cache_preserves_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        {
            let cache = Cache::open(&path).unwrap();
            cache
                .put("k", Some("e"), b"{}", Duration::from_secs(60))
                .unwrap();
        }
        let reopened = Cache::open(&path).unwrap();
        let entry = reopened.get("k").unwrap().unwrap();
        assert_eq!(entry.etag.as_deref(), Some("e"));
    }
}
