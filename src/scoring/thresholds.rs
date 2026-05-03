//! Threshold tables. Versioned with the scoring model.
//!
//! Tables here are the authoritative numeric values referenced by
//! `docs/methodology.md` and `docs/scoring-model.md`. They are also mirrored
//! in `src/config/default.toml` so users can override them — but the
//! defaults live here in code so the binary works even with no config file
//! and so changes are visible in `git log`.

/// Threshold table for the Activity Health module
/// (`docs/methodology.md` §Module 2).
#[derive(Debug, Clone, Copy)]
pub struct ActivityThresholds {
    pub days_since_last_commit_full_credit: f64,
    pub days_since_last_commit_zero: f64,
    pub commits_last_90d_full_credit: f64,
    pub commits_last_90d_zero: f64,
    pub active_contributors_full_credit: f64,
    pub active_contributors_zero: f64,
    pub median_issue_response_full_credit_hours: f64,
    pub median_issue_response_zero_hours: f64,
    pub days_since_last_release_full_credit: f64,
    pub days_since_last_release_zero: f64,
    /// Repo must be at least this many days old before we report `High`
    /// confidence on activity signals.
    pub min_repo_age_for_high_confidence_days: u64,
}

impl ActivityThresholds {
    /// Defaults from `docs/methodology.md` §Module 2 v1.0.
    #[must_use]
    pub const fn v1() -> Self {
        Self {
            days_since_last_commit_full_credit: 14.0,
            days_since_last_commit_zero: 365.0,
            commits_last_90d_full_credit: 30.0,
            commits_last_90d_zero: 0.0,
            active_contributors_full_credit: 4.0,
            active_contributors_zero: 1.0,
            median_issue_response_full_credit_hours: 48.0,
            median_issue_response_zero_hours: 720.0,
            days_since_last_release_full_credit: 90.0,
            days_since_last_release_zero: 730.0,
            min_repo_age_for_high_confidence_days: 180,
        }
    }
}

impl Default for ActivityThresholds {
    fn default() -> Self {
        Self::v1()
    }
}

/// Threshold table for the Maintainer Health module
/// (`docs/methodology.md` §Module 3).
#[derive(Debug, Clone, Copy)]
pub struct MaintainerThresholds {
    /// Bus-factor proxy ≥ this value scores 100. Below, scaled linearly to 0 at 0.
    pub bus_factor_full_credit: u64,
    /// Gini ≤ this is "balanced multi-maintainer".
    pub gini_full_credit: f64,
    /// Gini ≥ this scores 0 ("highly concentrated").
    pub gini_zero: f64,
    /// Retention ≥ this scores 100 (`[0, 1]` rate).
    pub retention_full_credit: f64,
    /// Retention ≤ this scores 0.
    pub retention_zero: f64,
    pub min_repo_age_for_high_confidence_days: u64,
}

impl MaintainerThresholds {
    /// Defaults from `docs/methodology.md` §Module 3 v1.0.
    #[must_use]
    pub const fn v1() -> Self {
        Self {
            bus_factor_full_credit: 5,
            gini_full_credit: 0.40,
            gini_zero: 0.85,
            retention_full_credit: 0.50,
            retention_zero: 0.10,
            min_repo_age_for_high_confidence_days: 180,
        }
    }
}

impl Default for MaintainerThresholds {
    fn default() -> Self {
        Self::v1()
    }
}

/// Linear interpolation: lower input value → higher score (e.g. days
/// since last commit).
#[must_use]
pub fn linear_lower_better(value: f64, full_credit: f64, zero: f64) -> u8 {
    debug_assert!(
        full_credit < zero,
        "for lower-better signals, full_credit < zero"
    );
    if value <= full_credit {
        100
    } else if value >= zero {
        0
    } else {
        let frac = (zero - value) / (zero - full_credit);
        clamp_round(frac * 100.0)
    }
}

/// Linear interpolation: higher input value → higher score (e.g. commits
/// last 90 days, active contributors).
#[must_use]
pub fn linear_higher_better(value: f64, full_credit: f64, zero: f64) -> u8 {
    debug_assert!(
        full_credit > zero,
        "for higher-better signals, full_credit > zero"
    );
    if value >= full_credit {
        100
    } else if value <= zero {
        0
    } else {
        let frac = (value - zero) / (full_credit - zero);
        clamp_round(frac * 100.0)
    }
}

fn clamp_round(x: f64) -> u8 {
    x.round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_better_full_credit() {
        assert_eq!(linear_lower_better(0.0, 14.0, 365.0), 100);
        assert_eq!(linear_lower_better(14.0, 14.0, 365.0), 100);
    }

    #[test]
    fn lower_better_zero() {
        assert_eq!(linear_lower_better(365.0, 14.0, 365.0), 0);
        assert_eq!(linear_lower_better(1000.0, 14.0, 365.0), 0);
    }

    #[test]
    fn lower_better_midpoint() {
        // mid = (14 + 365) / 2 = 189.5 → ~50
        let s = linear_lower_better(189.5, 14.0, 365.0);
        assert!((50..=51).contains(&s), "got {s}");
    }

    #[test]
    fn higher_better_full_credit() {
        assert_eq!(linear_higher_better(30.0, 30.0, 0.0), 100);
        assert_eq!(linear_higher_better(100.0, 30.0, 0.0), 100);
    }

    #[test]
    fn higher_better_zero() {
        assert_eq!(linear_higher_better(0.0, 30.0, 0.0), 0);
    }

    #[test]
    fn higher_better_midpoint() {
        // mid = 15 → 50
        let s = linear_higher_better(15.0, 30.0, 0.0);
        assert_eq!(s, 50);
    }

    #[test]
    fn contributor_scoring_thresholds() {
        // full_credit=4, zero=1
        assert_eq!(linear_higher_better(5.0, 4.0, 1.0), 100);
        assert_eq!(linear_higher_better(4.0, 4.0, 1.0), 100);
        // 2.5 between 1 and 4 → (2.5 - 1) / 3 = 0.5 → 50
        assert_eq!(linear_higher_better(2.5, 4.0, 1.0), 50);
        assert_eq!(linear_higher_better(1.0, 4.0, 1.0), 0);
    }

    #[test]
    fn issue_response_lower_better() {
        // 48h → 100, 720h → 0, 384h ≈ midpoint
        assert_eq!(linear_lower_better(48.0, 48.0, 720.0), 100);
        assert_eq!(linear_lower_better(720.0, 48.0, 720.0), 0);
        let s = linear_lower_better(384.0, 48.0, 720.0);
        assert!((49..=51).contains(&s), "got {s}");
    }
}
