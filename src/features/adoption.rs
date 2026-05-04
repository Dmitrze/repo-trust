//! Adoption Signals features — pure transformation from
//! [`crate::collectors::adoption::AdoptionRawData`] into the
//! normalized struct the scorer consumes.
//!
//! No I/O. Deterministic.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::collectors::adoption::AdoptionRawData;

/// Per-module features produced from raw collected data.
///
/// `weekly_downloads` is `None` when no packages are present *or* when every
/// package's `weekly_downloads` was `None` — the scorer drops the
/// downloads sub-score in that case.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AdoptionFeatures {
    /// Sum of weekly downloads across all packages, or `None` when no
    /// packages are published / deps.dev returned no download data.
    pub weekly_downloads: Option<u64>,
    /// Sorted, de-duplicated package-system list (e.g. `["GO", "NPM"]`).
    pub package_systems: Vec<String>,
    /// Number of distinct package systems.
    pub package_systems_count: u64,
    /// Whether `README.md` (or extension-less `README`) was found at the
    /// repo root.
    pub has_readme: bool,
    /// Word count of the README body (None if no README).
    pub readme_word_count: Option<usize>,
    pub has_docs_dir: bool,
    pub has_examples_dir: bool,
    /// Documentation-maturity score in the `[0.0, 1.0]` range. Combined
    /// from README presence + word-count band + `docs/` + `examples/`.
    pub documentation_maturity_score: f64,
    /// Number of awesome-list mentions for this repo. Day 3: always 0
    /// (loaded from config in v1.1).
    pub awesome_list_mentions: u64,
    pub archived: bool,
    pub deps_dev_error: bool,
}

/// Convert raw collected data into normalized features.
///
/// Documentation-maturity formula (range `[0.0, 1.0]`):
///
/// ```text
/// readme_weight = 0.50  if has_readme && words >= 500
///                = 0.35  if has_readme && words >= 100
///                = 0.20  if has_readme
///                = 0.00  otherwise
/// docs_weight   = 0.30  if has_docs_dir
/// examples_weight = 0.20  if has_examples_dir
/// total = readme_weight + docs_weight + examples_weight  // capped at 1.0
/// ```
///
/// Weights sum to 1.0 when all three are present and the README clears the
/// 500-word bar; this keeps the resulting `documentation_maturity_score`
/// well-defined and bounded.
#[must_use]
pub fn compute(raw: &AdoptionRawData, _now: OffsetDateTime) -> AdoptionFeatures {
    // ─── weekly downloads (sum of present values) ──────────────────────────
    let weekly_downloads = if raw.packages.is_empty() {
        None
    } else {
        let mut had_value = false;
        let mut sum: u64 = 0;
        for p in &raw.packages {
            if let Some(v) = p.weekly_downloads {
                had_value = true;
                sum = sum.saturating_add(v);
            }
        }
        if had_value {
            Some(sum)
        } else {
            None
        }
    };

    // ─── package_systems (sorted, unique) ──────────────────────────────────
    let systems_set: BTreeSet<String> = raw.packages.iter().map(|p| p.system.clone()).collect();
    let package_systems: Vec<String> = systems_set.into_iter().collect();
    let package_systems_count = u64::try_from(package_systems.len()).unwrap_or(0);

    // ─── documentation maturity ────────────────────────────────────────────
    let documentation_maturity_score = doc_maturity(
        raw.has_readme,
        raw.readme_word_count,
        raw.has_docs_dir,
        raw.has_examples_dir,
    );

    AdoptionFeatures {
        weekly_downloads,
        package_systems,
        package_systems_count,
        has_readme: raw.has_readme,
        readme_word_count: raw.readme_word_count,
        has_docs_dir: raw.has_docs_dir,
        has_examples_dir: raw.has_examples_dir,
        documentation_maturity_score: crate::utils::time::round6(documentation_maturity_score),
        awesome_list_mentions: u64::try_from(raw.awesome_list_mentions.len()).unwrap_or(0),
        archived: raw.archived,
        deps_dev_error: raw.deps_dev_error,
    }
}

/// Documentation-maturity formula. See [`compute`] for the weights.
#[must_use]
pub fn doc_maturity(
    has_readme: bool,
    word_count: Option<usize>,
    has_docs_dir: bool,
    has_examples_dir: bool,
) -> f64 {
    let readme: f64 = if has_readme {
        match word_count.unwrap_or(0) {
            n if n >= 500 => 0.50,
            n if n >= 100 => 0.35,
            _ => 0.20,
        }
    } else {
        0.0
    };
    let docs: f64 = if has_docs_dir { 0.30 } else { 0.0 };
    let examples: f64 = if has_examples_dir { 0.20 } else { 0.0 };
    (readme + docs + examples).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::deps_dev::PackageInfo;

    fn pkg(system: &str, name: &str, downloads: Option<u64>) -> PackageInfo {
        PackageInfo {
            system: system.into(),
            name: name.into(),
            weekly_downloads: downloads,
            latest_version: None,
        }
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap()
    }

    #[test]
    fn doc_maturity_full_signal_caps_at_one() {
        // README ≥ 500 words + docs/ + examples/ = 0.50 + 0.30 + 0.20 = 1.0
        let s = doc_maturity(true, Some(800), true, true);
        assert!((s - 1.0).abs() < 1e-9, "expected 1.0, got {s}");
    }

    #[test]
    fn doc_maturity_no_readme_anchors_zero_signal() {
        let s = doc_maturity(false, None, false, false);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn doc_maturity_short_readme_only_yields_low() {
        // README <100 words, no other signal → 0.20
        let s = doc_maturity(true, Some(20), false, false);
        assert!((s - 0.20).abs() < 1e-9, "expected 0.20, got {s}");
    }

    #[test]
    fn weekly_downloads_sums_across_packages() {
        let raw = AdoptionRawData {
            packages: vec![
                pkg("NPM", "left-pad", Some(1_000)),
                pkg("CARGO", "serde", Some(2_500)),
                pkg("PYPI", "requests", None), // None ignored
            ],
            ..AdoptionRawData::default()
        };
        let f = compute(&raw, now());
        assert_eq!(f.weekly_downloads, Some(3_500));
    }

    #[test]
    fn weekly_downloads_none_when_all_packages_have_none() {
        let raw = AdoptionRawData {
            packages: vec![pkg("NPM", "a", None), pkg("CARGO", "b", None)],
            ..AdoptionRawData::default()
        };
        let f = compute(&raw, now());
        assert_eq!(f.weekly_downloads, None);
    }

    #[test]
    fn package_systems_sorted_and_unique() {
        let raw = AdoptionRawData {
            packages: vec![
                pkg("NPM", "x", Some(10)),
                pkg("CARGO", "y", Some(20)),
                pkg("NPM", "z", Some(30)), // dup system
                pkg("GO", "w", Some(40)),
            ],
            ..AdoptionRawData::default()
        };
        let f = compute(&raw, now());
        assert_eq!(f.package_systems, vec!["CARGO", "GO", "NPM"]);
        assert_eq!(f.package_systems_count, 3);
    }
}
