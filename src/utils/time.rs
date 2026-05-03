//! Time helpers, including a serde adapter for ISO 8601.

pub mod iso8601 {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::format_description::well_known::Iso8601;
    use time::OffsetDateTime;

    pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = dt
            .format(&Iso8601::DEFAULT)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OffsetDateTime::parse(&s, &Iso8601::DEFAULT).map_err(serde::de::Error::custom)
    }
}

/// Round a float to 6 decimal places. Used to make floating-point output
/// deterministic per ADR-0007.
#[must_use]
pub fn round6(x: f64) -> f64 {
    (x * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::round6;

    #[test]
    fn round6_works() {
        assert!((round6(0.123_456_789) - 0.123_457).abs() < 1e-9);
        assert!((round6(1.0 / 3.0) - 0.333_333).abs() < 1e-9);
    }
}
