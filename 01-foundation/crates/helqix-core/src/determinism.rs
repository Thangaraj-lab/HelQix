//! Declared reproducibility level of a run (locked principle 4).
//!
//! * [`DeterminismLevel::Bitwise`] — the core runtime: same build, same platform,
//!   same inputs give bit-identical results.
//! * [`DeterminismLevel::Tolerance`] — external solvers: results are compared
//!   within a stated absolute tolerance.
//! * [`DeterminismLevel::Statistical`] — stochastic or quantum backends: single
//!   samples are not comparable; only distributions are.

/// How exactly a run is expected to reproduce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeterminismLevel {
    /// Bit-identical on the same build, platform and inputs.
    Bitwise,
    /// Every value reproduces within `absolute` (same unit as the values).
    Tolerance {
        /// Largest permitted absolute difference.
        absolute: f64,
    },
    /// Only statistical properties reproduce; samples cannot be compared one by one.
    Statistical,
}

/// Invalid determinism declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminismError {
    /// Tolerance must be finite and greater than zero.
    InvalidTolerance,
}

impl core::fmt::Display for DeterminismError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("tolerance must be finite and greater than zero")
    }
}

impl std::error::Error for DeterminismError {}

impl DeterminismLevel {
    /// Declares a tolerance level; rejects zero, negative and non-finite tolerances.
    pub fn tolerance(absolute: f64) -> Result<Self, DeterminismError> {
        if absolute.is_finite() && absolute > 0.0 { Ok(Self::Tolerance { absolute }) } else { Err(DeterminismError::InvalidTolerance) }
    }

    /// Stable label for records and reports.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self { Self::Bitwise => "Bitwise", Self::Tolerance { .. } => "Tolerance", Self::Statistical => "Statistical" }
    }

    /// Whether two equally long series reproduce at this level.
    ///
    /// Returns `None` for [`DeterminismLevel::Statistical`], where sample-by-sample
    /// comparison is meaningless, and for series of different lengths.
    #[must_use]
    pub fn reproduces(&self, original: &[f64], repeat: &[f64]) -> Option<bool> {
        if original.len() != repeat.len() { return None; }
        let pairs = original.iter().zip(repeat);
        match self {
            Self::Bitwise => Some(pairs.into_iter().all(|(a, b)| a.to_bits() == b.to_bits())),
            Self::Tolerance { absolute } => Some(pairs.into_iter().all(|(a, b)| (a - b).abs() <= *absolute)),
            Self::Statistical => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitwise_requires_identical_bits() {
        let level = DeterminismLevel::Bitwise;
        assert_eq!(level.reproduces(&[1.0, 2.0], &[1.0, 2.0]), Some(true));
        assert_eq!(level.reproduces(&[1.0, 2.0], &[1.0, 2.0 + f64::EPSILON * 2.0]), Some(false));
        // 0.0 and -0.0 compare equal but are different bits.
        assert_eq!(level.reproduces(&[0.0], &[-0.0]), Some(false));
        assert_eq!(level.label(), "Bitwise");
    }

    #[test]
    fn tolerance_compares_within_the_bound() {
        let level = DeterminismLevel::tolerance(1e-3).unwrap();
        assert_eq!(level.reproduces(&[1.0, 2.0], &[1.0005, 1.9995]), Some(true));
        assert_eq!(level.reproduces(&[1.0], &[1.002]), Some(false));
        assert_eq!(level.reproduces(&[f64::NAN], &[f64::NAN]), Some(false));
        assert_eq!(level.label(), "Tolerance");
    }

    #[test]
    fn statistical_and_length_mismatch_are_not_decidable() {
        assert_eq!(DeterminismLevel::Statistical.reproduces(&[1.0], &[1.0]), None);
        assert_eq!(DeterminismLevel::Bitwise.reproduces(&[1.0], &[1.0, 2.0]), None);
        assert_eq!(DeterminismLevel::Statistical.label(), "Statistical");
    }

    #[test]
    fn invalid_tolerances_are_rejected() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(DeterminismLevel::tolerance(bad), Err(DeterminismError::InvalidTolerance));
        }
    }
}
