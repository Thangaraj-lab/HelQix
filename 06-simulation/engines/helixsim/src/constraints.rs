//! Constraint evaluation against simulation state (Layer 06 / HELIXSIM).
//!
//! ## Ownership boundary
//!
//! Layer 01 ([`helqix_core::Constraint`]) defines the *engineering* constraint
//! semantics: a named, inclusive scalar interval `min <= x <= max` attached to
//! a reference. Layer 03 collects those into constraint sets. **This module
//! does not duplicate that model** — it consumes canonical `Constraint` values
//! and answers the simulation-time question: *does the measured value satisfy
//! the constraint?*
//!
//! Conceptually each constraint is an inequality predicate:
//!
//! ```text
//! g(x) = max(x − max_bound, min_bound − x, 0) <= 0
//! ```
//!
//! Evaluation is read-only. Violations are *reported*, never silently
//! corrected: HELIXSIM has no authority to move state back inside bounds
//! because Layer 04 owns authoritative state and any correction policy would
//! have to be an explicit engineering decision recorded upstream.
//!
//! ## Tolerance semantics
//!
//! The Foundation contract defines closed bounds with **no tolerance band**,
//! so exact-boundary values (`x == min`, `x == max`) are satisfied. If a
//! deployment later introduces tolerance in Layer 03, evaluation here must be
//! updated to match that contract rather than inventing slack locally.
//!
//! Non-finite measurements are reported as violations — consistent with
//! `Constraint::contains`, which refuses NaN/infinity — because a non-finite
//! reading can never prove satisfaction of a finite interval.

use std::fmt;

use helqix_core::{Constraint, Identifier};

/// The result of evaluating one constraint against one measured value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintOutcome {
    /// The measurement lies inside the inclusive interval.
    Satisfied {
        /// Constraint name.
        name: Identifier,
        /// Measured value.
        value: f64,
        /// Signed margin: `>= 0` when satisfied; distance to the nearest
        /// bound (`min(value − min, max − value)`).
        margin: f64,
    },
    /// The measurement lies outside the inclusive interval.
    Violated {
        /// Constraint name.
        name: Identifier,
        /// Measured value.
        value: f64,
        /// Which bound was exceeded.
        bound: ViolatedBound,
        /// Positive distance beyond the violated bound.
        amount: f64,
    },
}

impl ConstraintOutcome {
    /// Returns `true` for [`Self::Satisfied`].
    #[must_use]
    pub const fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied { .. })
    }

    /// The constraint name under evaluation.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        match self {
            Self::Satisfied { name, .. } | Self::Violated { name, .. } => name,
        }
    }
}

/// Which side of the interval a violation crossed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolatedBound {
    /// Value below the inclusive minimum.
    BelowMin,
    /// Value above the inclusive maximum.
    AboveMax,
    /// Value was NaN or infinite — provably unsatisfiable.
    NonFinite,
}

/// A single measured quantity paired with the constraint that governs it.
///
/// Measurement pairing (which sensor/state field feeds which constraint) is
/// resolved by callers — typically the kernel from Layer 04 state fields —
/// keeping this module free of any state-storage knowledge.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitoredValue<'a> {
    constraint: &'a Constraint,
    value: f64,
}

impl<'a> MonitoredValue<'a> {
    /// Pairs a constraint with its current measurement.
    #[must_use]
    pub const fn new(constraint: &'a Constraint, value: f64) -> Self {
        Self { constraint, value }
    }

    /// Returns the governing constraint.
    #[must_use]
    pub const fn constraint(&self) -> &'a Constraint {
        self.constraint
    }

    /// Returns the measured value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }
}

/// Evaluates one constraint against one value.
///
/// Deterministic pure function of `(constraint, value)`. Boundary values are
/// satisfied (inclusive bounds per the Foundation contract).
#[must_use]
pub fn evaluate(constraint: &Constraint, value: f64) -> ConstraintOutcome {
    let name = constraint.name().clone();
    if !value.is_finite() {
        return ConstraintOutcome::Violated {
            name,
            value,
            bound: ViolatedBound::NonFinite,
            amount: f64::NAN,
        };
    }
    if value < constraint.min() {
        return ConstraintOutcome::Violated {
            name,
            value,
            bound: ViolatedBound::BelowMin,
            amount: constraint.min() - value,
        };
    }
    if value > constraint.max() {
        return ConstraintOutcome::Violated {
            name,
            value,
            bound: ViolatedBound::AboveMax,
            amount: value - constraint.max(),
        };
    }
    // Satisfied: margin is distance to the nearer bound, both finite.
    let margin = (value - constraint.min()).min(constraint.max() - value);
    ConstraintOutcome::Satisfied {
        name,
        value,
        margin,
    }
}

/// Aggregated report over an ordered collection of monitored values.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintReport {
    outcomes: Vec<ConstraintOutcome>,
}

impl ConstraintReport {
    /// Evaluates every pair in **slice order** (deterministic: no hashing,
    /// sorting or parallel reduction influences the outcome list).
    #[must_use]
    pub fn evaluate_all(monitored: &[MonitoredValue<'_>]) -> Self {
        Self {
            outcomes: monitored
                .iter()
                .map(|m| evaluate(m.constraint(), m.value()))
                .collect(),
        }
    }

    /// Individual outcomes, in input order.
    #[must_use]
    pub fn outcomes(&self) -> &[ConstraintOutcome] {
        &self.outcomes
    }

    /// `true` when every constraint is satisfied.
    #[must_use]
    pub fn all_satisfied(&self) -> bool {
        self.outcomes.iter().all(ConstraintOutcome::is_satisfied)
    }

    /// Names of violated constraints, in input order.
    #[must_use]
    pub fn violations(&self) -> Vec<&Identifier> {
        self.outcomes
            .iter()
            .filter(|o| !o.is_satisfied())
            .map(ConstraintOutcome::name)
            .collect()
    }

    /// Number of evaluated constraints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    /// Whether the report evaluated nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced while preparing constraint evaluation inputs.
///
/// Note these guard *input plumbing*, not physics: the Foundation
/// [`Constraint`] type validates its own bounds at construction, so an
/// already-constructed constraint cannot carry NaN bounds.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintEvalError {
    /// A measured value required to be finite was NaN or infinite.
    NonFiniteMeasurement {
        /// Name of the constraint whose measurement failed.
        name: String,
    },
    /// A structural validation failure surfaced from Foundation constraints.
    InvalidConstraint(helqix_core::ConstraintError),
}

impl fmt::Display for ConstraintEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteMeasurement { name } => {
                write!(f, "measurement for constraint `{name}` is not finite")
            }
            Self::InvalidConstraint(error) => write!(f, "invalid constraint: {error}"),
        }
    }
}

impl std::error::Error for ConstraintEvalError {}

impl From<helqix_core::ConstraintError> for ConstraintEvalError {
    fn from(error: helqix_core::ConstraintError) -> Self {
        Self::InvalidConstraint(error)
    }
}

/// Strict variant of [`evaluate`] that converts non-finite measurements into
/// errors instead of violations. Used where a pipeline wants to distinguish
/// "the physics broke" from "the operating limit was exceeded".
///
/// # Errors
///
/// [`ConstraintEvalError::NonFiniteMeasurement`] when `value` is NaN/infinite.
pub fn evaluate_strict(
    constraint: &Constraint,
    value: f64,
) -> Result<ConstraintOutcome, ConstraintEvalError> {
    if !value.is_finite() {
        return Err(ConstraintEvalError::NonFiniteMeasurement {
            name: constraint.name().to_string(),
        });
    }
    Ok(evaluate(constraint, value))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::{ConstraintReference, ReferenceKind};

    fn bounded(name: &str, min: f64, max: f64) -> Constraint {
        Constraint::new(
            Identifier::parse(name).unwrap(),
            ConstraintReference::new(ReferenceKind::Property, Identifier::parse("prop").unwrap()),
            min,
            max,
        )
        .unwrap()
    }

    #[test]
    fn interior_value_satisfied_with_margin() {
        let c = bounded("vmax", 0.0, 100.0);
        match evaluate(&c, 40.0) {
            ConstraintOutcome::Satisfied { margin, .. } => assert_eq!(margin, 40.0),
            other => panic!("expected satisfied, got {other:?}"),
        }
    }

    #[test]
    fn exact_boundary_is_satisfied_inclusive_bounds() {
        let c = bounded("vmax", 0.0, 100.0);
        assert!(evaluate(&c, 0.0).is_satisfied());
        assert!(evaluate(&c, 100.0).is_satisfied());
        // Margin at a boundary is exactly zero.
        match evaluate(&c, 100.0) {
            ConstraintOutcome::Satisfied { margin, .. } => assert_eq!(margin, 0.0),
            other => panic!("expected satisfied, got {other:?}"),
        }
    }

    #[test]
    fn outside_value_violates_correct_bound_and_amount() {
        let c = bounded("vmax", 0.0, 100.0);
        match evaluate(&c, 130.0) {
            ConstraintOutcome::Violated { bound, amount, .. } => {
                assert_eq!(bound, ViolatedBound::AboveMax);
                assert_eq!(amount, 30.0);
            }
            other => panic!("expected violation, got {other:?}"),
        }
        match evaluate(&c, -5.0) {
            ConstraintOutcome::Violated { bound, amount, .. } => {
                assert_eq!(bound, ViolatedBound::BelowMin);
                assert_eq!(amount, 5.0);
            }
            other => panic!("expected violation, got {other:?}"),
        }
    }

    #[test]
    fn nonfinite_measurement_is_a_violation_not_a_pass() {
        let c = bounded("vmax", 0.0, 100.0);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            match evaluate(&c, bad) {
                ConstraintOutcome::Violated {
                    bound: ViolatedBound::NonFinite,
                    ..
                } => {}
                other => panic!("expected non-finite violation, got {other:?}"),
            }
        }
        // Strict mode turns the same input into an error.
        assert!(matches!(
            evaluate_strict(&c, f64::NAN),
            Err(ConstraintEvalError::NonFiniteMeasurement { .. })
        ));
        assert!(evaluate_strict(&c, 50.0).unwrap().is_satisfied());
    }

    #[test]
    fn multiple_constraints_reported_in_input_order() {
        let a = bounded("alpha", 0.0, 1.0);
        let b = bounded("beta", -10.0, 10.0);
        let g = bounded("gamma", 5.0, 6.0);
        let monitored = [
            MonitoredValue::new(&a, 0.5),
            MonitoredValue::new(&b, 11.0), // violated
            MonitoredValue::new(&g, 5.5),
        ];
        let report = ConstraintReport::evaluate_all(&monitored);
        assert_eq!(report.len(), 3);
        assert!(!report.all_satisfied());
        let names: Vec<String> = report.violations().iter().map(|n| n.to_string()).collect();
        assert_eq!(names, vec!["beta".to_owned()]);
        assert_eq!(report.outcomes()[0].name().as_str(), "alpha");
        assert_eq!(report.outcomes()[2].name().as_str(), "gamma");
    }

    #[test]
    fn evaluation_is_deterministic_pure_function() {
        let c = bounded("vmax", 0.0, 100.0);
        let first = evaluate(&c, 99.999);
        for _ in 0..100 {
            assert_eq!(evaluate(&c, 99.999), first);
        }
    }

    #[test]
    fn empty_report_is_trivially_satisfied() {
        let report = ConstraintReport::evaluate_all(&[]);
        assert!(report.is_empty());
        assert!(report.all_satisfied());
        assert!(report.violations().is_empty());
    }

    #[test]
    fn degenerate_point_constraint_accepts_only_exact_value() {
        let c = bounded("setpoint", 3.0, 3.0);
        assert!(evaluate(&c, 3.0).is_satisfied());
        // NOTE: 3.0 + f64::EPSILON rounds back to exactly 3.0 in IEEE-754
        // (ulp(3.0) == 2·EPSILON), so it is satisfied. The next representable
        // float above 3.0 must violate.
        assert!(!evaluate(&c, 3.0_f64.next_up()).is_satisfied());
    }
}
