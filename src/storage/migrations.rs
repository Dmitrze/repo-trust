//! Schema migrations.
//!
//! The current set is one initial migration that defines the three tables
//! described in `docs/architecture.md` §6.1. New migrations append to the
//! `Vec` returned by [`migrations`]; `rusqlite_migration` runs them in order
//! the first time each connection is opened.

use rusqlite_migration::{Migrations, M};

/// Build the migration set for the SQLite cache.
///
/// New migrations append to this vector. Order matters and is checked by
/// `rusqlite_migration` against the schema version stored in the file.
#[must_use]
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("migrations/0001_initial.sql"))])
}

#[cfg(test)]
mod tests {
    use super::migrations;
    use rusqlite::Connection;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let m = migrations();
        m.to_latest(&mut conn).expect("first apply");
        // Idempotent — second call is a no-op and must not error.
        m.to_latest(&mut conn).expect("second apply (idempotent)");

        // All three tables exist.
        for table in ["api_cache", "features", "reports"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} missing after migration");
        }
    }
}
