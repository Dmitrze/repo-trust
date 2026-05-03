//! Cache facade — thin wrapper around an r2d2-pooled SQLite connection.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CacheHandle {
    pub path: PathBuf,
    // TODO: r2d2 Pool<SqliteConnectionManager>
}

impl CacheHandle {
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }
}
