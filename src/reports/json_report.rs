//! JSON report writer. Emits the frozen schema (see `docs/architecture.md` §5).

use anyhow::Result;

use crate::models::TrustReport;

pub fn write(report: &TrustReport, path: &std::path::Path) -> Result<()> {
    let s = serde_json::to_string_pretty(report)?;
    std::fs::write(path, s)?;
    Ok(())
}
