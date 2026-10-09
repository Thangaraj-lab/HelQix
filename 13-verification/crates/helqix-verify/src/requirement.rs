//! Explicit, verifiable engineering requirements for HELQIX Layer 13.
//!
//! A [`Requirement`] states a condition that executed engineering reality
//! must satisfy — e.g. "altitude must remain at or above 100 m". It is a
//! pure definition value, exactly like Foundation's `Constraint`: it never
//! evaluates anything. Evaluation belongs to [`crate::constraint_check`].
//!
//! ```text
//! Requirement (this module) ──▶ constraint_check::check() ──▶ VerificationResult
//!   what must hold                 pure comparison               PASS / FAIL / INCONCLUSIVE
//! ```
//!
//! # Reused canonical contracts (no duplicates created here)
//!
//! - Identity: [`Identifier`] (Foundation validated grammar). The subject of
//!   a requirement is a [`SubjectTarget`] built on the same tagged-reference
//!   vocabulary Layer 10 uses (`EntityRef::Component`,
//!   `EntityRef::Observation`, …), so verification never invents a second id
//!   system.
//! - Values and units: [`Quantity`] / [`Unit`] (Foundation). A requirement
//!   bound always carries its unit; there is deliberately no bare-number
//!   bound, because unitless thresholds are the classic source of silent
//!   dimensional errors.
//! - Inclusive-bound semantics mirror Foundation `Constraint`
//!   (`min <= value <= max`) so the two layers agree at boundaries.
//!
//! # What this module does NOT do
//!
//! - It does not resolve the referenced component/property against any
//!   registry (that would couple Layer 13 to Layer 04 internals).
//! - It does not evaluate observations (that is `constraint_check`).
//! - It does not know about domains: every concept arrives through
//!   caller-supplied identifiers and Foundation units.
//!
//! # Determinism
//!
//! Construction validates fields in a fixed order (operator/bound shape →
//! finiteness → dimension agreement → ordering after unit normalization →
//! tolerance sign), so identical input always yields the identical value or
//! the identical error. No clocks, randomness, or registries are involved.

use std::error::Error;
use std::fmt;

use helqix_core::error::{NonFiniteKind, UnitError};
use helqix_core::id::Identifier;
use helqix_core::quantity::Quantity;
use helqix_core::units::Unit;
use helqix_events::event::EntityRef;

/// The kind of thing a requirement applies to.
///
/// Wraps the canonical Layer 10 [`EntityRef`] tagged reference. The most
/// common targets are `EntityRef::Component` (a structural member whose
/// runtime property is observed) and `EntityRef::Observation` (a directly
/// reported measurement stream), but any validated reference is accepted so
/// Layer 13 stays domain-independent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectTarget(EntityRef);

impl SubjectTarget {
    /// Targets a component by validated identifier text.
    ///
    /// # Errors
    ///
    /// Returns [`RequirementError::InvalidIdentifier`] when `text` violates
    /// the Foundation grammar.
    pub fn component(text: &str) -> Result<Self, RequirementError> {
        EntityRef::component(text)
            .map(Self)
            .map_err(|_| RequirementError::InvalidIdentifier { field: "target" })
    }

    /// Targets an observation/metric stream by validated identifier text.
    ///
    /// # Errors
    ///
    /// As for [`SubjectTarget::component`].
    pub fn observation(text: &str) -> Result<Self, RequirementError> {
        EntityRef::observation(text)
            .map(Self)
            .map_err(|_| RequirementError::InvalidIdentifier { field: "target" })
    }

    /// Wraps an already-validated canonical entity reference.
    #[must_use]
    pub const fn from_entity_ref(reference: EntityRef) -> Self {
        Self(reference)
    }

    /// Returns the underlying canonical reference.
    #[must_use]
    pub const fn reference(&self) -> &EntityRef {
        &self.0
    }

    /// Returns the referenced identifier.
    #[must_use]
    pub fn identifier(&self) -> &Identifier {
        self.0.id()
    }
}

impl fmt::Display for SubjectTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`", self.0.kind_name(), self.0.id())
    }
}

/// Comparison direction connecting an observed value with the requirement
/// bound.
///
/// Every operator maps onto Foundation `Constraint`'s inclusive-interval
/// semantics (`min <= value <= max`): `AtMost(b)` is the interval
/// `(-inf, b]`, `AtLeast(b)` is `[b, +inf)`, `Within(lo, hi)` is `[lo, hi]`.
/// Boundary equality therefore always satisfies the requirement, matching
/// how Layer 01 defines constraint containment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operator {
    /// observed `<=` upper bound.
    AtMost,
    /// observed `<` upper bound (strict).
    LessThan,
    /// observed `>=` lower bound.
    AtLeast,
    /// observed `>` lower bound (strict).
    GreaterThan,
    /// lower bound `<=` observed `<=` upper bound (inclusive interval).
    Within,
    /// |observed − nominal| `<=` tolerance.
    Approximately,
}

impl Operator {
    /// Returns the stable machine-readable name used in explanations.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AtMost => "at_most",
            Self::LessThan => "less_than",
            Self::AtLeast => "at_least",
            Self::GreaterThan => "greater_than",
            Self::Within => "within",
            Self::Approximately => "approximately",
        }
    }

    /// Renders the operator symbol used in human-readable explanations
    /// (e.g. `temperature <= 100 K`).
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::AtMost => "<=",
            Self::LessThan => "<",
            Self::AtLeast => ">=",
            Self::GreaterThan => ">",
            Self::Within => "in [..]",
            Self::Approximately => "~",
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The expected numeric condition of a requirement, always unit-carrying.
///
/// Bounds are stored as [`Quantity`] values, so dimensional checking and
/// unit conversion reuse Foundation semantics exclusively. Constructing a
/// `Bound` validates finiteness and (for intervals) ordering *after* unit
/// normalization.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    /// Single upper bound (used with [`Operator::AtMost`] /
    /// [`Operator::LessThan`]).
    Upper(Quantity),
    /// Single lower bound (used with [`Operator::AtLeast`] /
    /// [`Operator::GreaterThan`]).
    Lower(Quantity),
    /// Inclusive interval `[low, high]` (used with [`Operator::Within`]).
    Interval {
        /// Inclusive lower limit.
        low: Quantity,
        /// Inclusive upper limit.
        high: Quantity,
    },
    /// Nominal value plus non-negative symmetric tolerance (used with
    /// [`Operator::Approximately`]). Equivalent to the inclusive interval
    /// `[nominal − tol, nominal + tol]` in the nominal's unit.
    Tolerance {
        /// Central/nominal value.
        nominal: Quantity,
        /// Non-negative tolerance expressed in the nominal's unit.
        tolerance: Quantity,
    },
}

impl Bound {
    /// Validates internal consistency of the bound: finite values, matching
    /// dimensions across sub-quantities, ordered interval bounds, and a
    /// non-negative tolerance.
    ///
    /// # Errors
    ///
    /// See [`RequirementError`] variants `NonFiniteValue`,
    /// `IncompatibleUnits`, `InvertedBounds`, `NegativeTolerance`.
    pub fn validate(&self) -> Result<(), RequirementError> {
        match self {
            Self::Upper(q) | Self::Lower(q) => {
                require_finite(q.value())?;
                Ok(())
            }
            Self::Interval { low, high } => {
                require_finite(low.value())?;
                require_finite(high.value())?;
                same_dimension(low.unit(), high.unit(), "interval bounds")?;
                // Normalize into the low bound's unit before ordering, so
                // `km` vs `m` cannot silently invert the interval.
                let high_in_low = high
                    .value_in(low.unit())
                    .map_err(RequirementError::UnitConversion)?;
                if high_in_low < low.value() {
                    return Err(RequirementError::InvertedBounds {
                        min: low.value(),
                        max: high_in_low,
                        unit: low.unit().symbol().to_owned(),
                    });
                }
                Ok(())
            }
            Self::Tolerance { nominal, tolerance } => {
                require_finite(nominal.value())?;
                require_finite(tolerance.value())?;
                same_dimension(tolerance.unit(), nominal.unit(), "tolerance")?;
                let tol_in_nominal = tolerance
                    .value_in(nominal.unit())
                    .map_err(RequirementError::UnitConversion)?;
                if tol_in_nominal < 0.0 {
                    return Err(RequirementError::NegativeTolerance {
                        tolerance: tol_in_nominal,
                        unit: nominal.unit().symbol().to_owned(),
                    });
                }
                Ok(())
            }
        }
    }

    /// The comparison unit carried by this bound (all sub-quantities share
    /// it after validation).
    #[must_use]
    pub fn unit(&self) -> &Unit {
        match self {
            Self::Upper(q) | Self::Lower(q) => q.unit(),
            Self::Interval { low, .. } => low.unit(),
            Self::Tolerance { nominal, .. } => nominal.unit(),
        }
    }
}

/// Consequence weight of a requirement violation.
///
/// Severity describes engineering impact, not evaluation logic: the verdict
/// is computed identically for every severity. It is metadata that lets
/// downstream reporting (Layer 21 UI, Layer 22 API) prioritize findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Violations invalidate the engineering objective.
    Critical,
    /// Violations must be resolved before acceptance.
    Major,
    /// Violations should be resolved but may be waived with justification.
    Minor,
    /// Informational tracking only.
    Advisory,
}

impl Severity {
    /// Stable machine-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Advisory => "advisory",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An explicit engineering condition that can be verified.
///
/// Conceptually:
///
/// ```text
/// target.property OPERATOR BOUND   (severity)
/// ```
///
/// Example: "altitude must remain above 100 m" is
/// `Requirement::at_least(id("r17"), …, SubjectTarget::component("aircraft")?,
/// id("altitude"), quantity(100.0, "m")?, Severity::Critical)`.
///
/// A requirement is immutable, deterministic, inspectable, and fully typed:
/// identity, target, property, operator, and bound each use the canonical
/// Foundation/Layer 10 types described in the module documentation.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    id: Identifier,
    description: String,
    target: SubjectTarget,
    property: Identifier,
    operator: Operator,
    bound: Bound,
    severity: Severity,
}

impl Requirement {
    /// Creates and validates a requirement.
    ///
    /// Validation order is fixed: operator/bound shape agreement first, then
    /// bound internal consistency ([`Bound::validate`]).
    ///
    /// # Errors
    ///
    /// Returns [`RequirementError`] when the bound shape does not match the
    /// operator or the bound fails validation.
    pub fn new(
        id: Identifier,
        description: impl Into<String>,
        target: SubjectTarget,
        property: Identifier,
        operator: Operator,
        bound: Bound,
        severity: Severity,
    ) -> Result<Self, RequirementError> {
        check_shape(operator, &bound)?;
        bound.validate()?;

        Ok(Self {
            id,
            description: description.into(),
            target,
            property,
            operator,
            bound,
            severity,
        })
    }

    /// Convenience: `target.property <= limit`.
    ///
    /// # Errors
    ///
    /// Propagates [`RequirementError`] from [`Requirement::new`].
    pub fn at_most(
        id: Identifier,
        description: impl Into<String>,
        target: SubjectTarget,
        property: Identifier,
        limit: Quantity,
        severity: Severity,
    ) -> Result<Self, RequirementError> {
        Self::new(
            id,
            description,
            target,
            property,
            Operator::AtMost,
            Bound::Upper(limit),
            severity,
        )
    }

    /// Convenience: `target.property >= floor`.
    ///
    /// # Errors
    ///
    /// Propagates [`RequirementError`] from [`Requirement::new`].
    pub fn at_least(
        id: Identifier,
        description: impl Into<String>,
        target: SubjectTarget,
        property: Identifier,
        floor: Quantity,
        severity: Severity,
    ) -> Result<Self, RequirementError> {
        Self::new(
            id,
            description,
            target,
            property,
            Operator::AtLeast,
            Bound::Lower(floor),
            severity,
        )
    }

    /// Convenience: `low <= target.property <= high` (inclusive, mirroring
    /// Foundation `Constraint`).
    ///
    /// # Errors
    ///
    /// Propagates [`RequirementError`] from [`Requirement::new`], including
    /// inverted bounds *after* unit normalization.
    pub fn within(
        id: Identifier,
        description: impl Into<String>,
        target: SubjectTarget,
        property: Identifier,
        low: Quantity,
        high: Quantity,
        severity: Severity,
    ) -> Result<Self, RequirementError> {
        Self::new(
            id,
            description,
            target,
            property,
            Operator::Within,
            Bound::Interval { low, high },
            severity,
        )
    }

    /// Convenience: `|target.property − nominal| <= tolerance`.
    ///
    /// # Errors
    ///
    /// Propagates [`RequirementError`] from [`Requirement::new`]; negative
    /// tolerances are rejected.
    pub fn approximately(
        id: Identifier,
        description: impl Into<String>,
        target: SubjectTarget,
        property: Identifier,
        nominal: Quantity,
        tolerance: Quantity,
        severity: Severity,
    ) -> Result<Self, RequirementError> {
        Self::new(
            id,
            description,
            target,
            property,
            Operator::Approximately,
            Bound::Tolerance { nominal, tolerance },
            severity,
        )
    }

    /// Canonical requirement identity.
    #[must_use]
    pub const fn id(&self) -> &Identifier {
        &self.id
    }

    /// Human-readable statement of the condition.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The subject the requirement applies to.
    #[must_use]
    pub const fn target(&self) -> &SubjectTarget {
        &self.target
    }

    /// The observed property the requirement constrains.
    #[must_use]
    pub const fn property(&self) -> &Identifier {
        &self.property
    }

    /// Comparison direction.
    #[must_use]
    pub const fn operator(&self) -> Operator {
        self.operator
    }

    /// Expected condition (unit-carrying).
    #[must_use]
    pub const fn bound(&self) -> &Bound {
        &self.bound
    }

    /// Violation consequence weight.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// The unit in which comparisons are performed.
    ///
    /// Observations are converted into this unit exactly once per check
    /// (see [`crate::constraint_check`]).
    #[must_use]
    pub fn comparison_unit(&self) -> &Unit {
        self.bound.unit()
    }

    /// Renders the condition as a compact, deterministic explanation string,
    /// e.g. `motor.temperature <= 100 K`.
    #[must_use]
    pub fn display_condition(&self) -> String {
        let target = format!("{}.{}", self.target.identifier(), self.property);
        match &self.bound {
            Bound::Upper(q) => {
                format!("{target} {} {}", self.operator.symbol(), format_quantity(q))
            }
            Bound::Lower(q) => {
                format!("{target} {} {}", self.operator.symbol(), format_quantity(q))
            }
            Bound::Interval { low, high } => format!(
                "{target} in [{}, {}]",
                format_quantity(low),
                format_quantity(high)
            ),
            Bound::Tolerance { nominal, tolerance } => format!(
                "{target} ~= {} +/- {}",
                format_quantity(nominal),
                format_quantity(tolerance)
            ),
        }
    }
}

fn format_quantity(quantity: &Quantity) -> String {
    format!("{} {}", quantity.value(), quantity.unit().symbol())
}

/// Ensures the bound shape matches the operator's arity.
fn check_shape(operator: Operator, bound: &Bound) -> Result<(), RequirementError> {
    let ok = match operator {
        Operator::AtMost | Operator::LessThan => matches!(bound, Bound::Upper(_)),
        Operator::AtLeast | Operator::GreaterThan => matches!(bound, Bound::Lower(_)),
        Operator::Within => matches!(bound, Bound::Interval { .. }),
        Operator::Approximately => matches!(bound, Bound::Tolerance { .. }),
    };

    if ok {
        Ok(())
    } else {
        Err(RequirementError::ShapeMismatch {
            operator: operator.as_str(),
            bound: bound_kind(bound),
        })
    }
}

const fn bound_kind(bound: &Bound) -> &'static str {
    match bound {
        Bound::Upper(_) => "upper",
        Bound::Lower(_) => "lower",
        Bound::Interval { .. } => "interval",
        Bound::Tolerance { .. } => "tolerance",
    }
}

fn require_finite(value: f64) -> Result<(), RequirementError> {
    match NonFiniteKind::classify(value) {
        Some(kind) => Err(RequirementError::NonFiniteValue(kind)),
        None => Ok(()),
    }
}

fn same_dimension(a: &Unit, b: &Unit, context: &str) -> Result<(), RequirementError> {
    if a.dimension() == b.dimension() {
        Ok(())
    } else {
        Err(RequirementError::IncompatibleUnits {
            context: context.to_owned(),
            expected: a.symbol().to_owned(),
            found: b.symbol().to_owned(),
        })
    }
}

/// Errors produced while constructing or validating a [`Requirement`].
///
/// Each variant names a distinct engineering defect; none collapse into a
/// generic string so callers can react programmatically.
#[derive(Debug, Clone, PartialEq)]
pub enum RequirementError {
    /// Identifier text violated the Foundation grammar.
    InvalidIdentifier {
        /// Which field failed ("target", …).
        field: &'static str,
    },
    /// The bound shape does not match the operator's arity.
    ShapeMismatch {
        /// Offending operator name.
        operator: &'static str,
        /// Offending bound kind name.
        bound: &'static str,
    },
    /// A bound value was NaN or infinite (delegated to Foundation's
    /// classification).
    NonFiniteValue(NonFiniteKind),
    /// Two quantities inside one bound measure different dimensions.
    IncompatibleUnits {
        /// Where the mismatch occurred.
        context: String,
        /// Expected unit symbol.
        expected: String,
        /// Found unit symbol.
        found: String,
    },
    /// Interval bounds were inverted after unit normalization.
    InvertedBounds {
        /// Normalized minimum.
        min: f64,
        /// Normalized maximum.
        max: f64,
        /// Unit symbol used for normalization.
        unit: String,
    },
    /// A tolerance bound was negative.
    NegativeTolerance {
        /// Rejected tolerance value.
        tolerance: f64,
        /// Unit symbol of the nominal value.
        unit: String,
    },
    /// A Foundation unit conversion failed during validation.
    UnitConversion(UnitError),
}

impl fmt::Display for RequirementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field } => {
                write!(f, "invalid {field} identifier (Foundation grammar)")
            }
            Self::ShapeMismatch { operator, bound } => write!(
                f,
                "operator `{operator}` expects a `{bound}`-shaped bound"
            ),
            Self::NonFiniteValue(kind) => write!(f, "requirement bound is not finite: {kind}"),
            Self::IncompatibleUnits {
                context,
                expected,
                found,
            } => write!(
                f,
                "incompatible units in requirement {context}: expected `{expected}`, found `{found}`"
            ),
            Self::InvertedBounds { min, max, unit } => write!(
                f,
                "requirement interval bounds are inverted after unit normalization: {min} > {max} ({unit})"
            ),
            Self::NegativeTolerance { tolerance, unit } => write!(
                f,
                "requirement tolerance is negative: {tolerance} ({unit})"
            ),
            Self::UnitConversion(error) => {
                write!(f, "requirement unit conversion failed: {error}")
            }
        }
    }
}

impl Error for RequirementError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnitConversion(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).expect("test identifier must be valid")
    }

    fn unit(symbol: &str) -> Unit {
        Unit::parse(symbol).expect("test unit must be registered")
    }

    fn quantity(value: f64, symbol: &str) -> Quantity {
        Quantity::new(value, unit(symbol)).expect("test quantity must be finite")
    }

    fn target() -> SubjectTarget {
        SubjectTarget::component("aircraft").expect("valid target")
    }

    #[test]
    fn creates_valid_minimum_altitude_requirement() {
        let requirement = Requirement::at_least(
            id("r17"),
            "Altitude must remain above 100 m",
            target(),
            id("altitude"),
            quantity(100.0, "m"),
            Severity::Critical,
        )
        .expect("valid requirement");

        assert_eq!(requirement.id().as_str(), "r17");
        assert_eq!(requirement.operator(), Operator::AtLeast);
        assert_eq!(requirement.comparison_unit().symbol(), "m");
        assert_eq!(requirement.severity(), Severity::Critical);
        assert_eq!(requirement.property().as_str(), "altitude");
    }

    #[test]
    fn rejects_target_with_invalid_identifier() {
        let error = SubjectTarget::component("9bad").unwrap_err();
        assert_eq!(error, RequirementError::InvalidIdentifier { field: "target" });
    }

    #[test]
    fn rejects_operator_bound_shape_mismatch() {
        let error = Requirement::new(
            id("r"),
            "",
            target(),
            id("altitude"),
            Operator::AtMost,
            Bound::Lower(quantity(100.0, "m")),
            Severity::Major,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            RequirementError::ShapeMismatch {
                operator: "at_most",
                bound: "lower"
            }
        ));
    }

    #[test]
    fn foundation_rejects_non_finite_bound_values() {
        // Quantity itself refuses NaN/infinity, so a requirement can never
        // be constructed around a non-finite threshold.
        assert!(Quantity::new(f64::NAN, unit("m")).is_err());
        assert!(Quantity::new(f64::INFINITY, unit("m")).is_err());
    }

    #[test]
    fn rejects_inverted_interval() {
        let error = Requirement::within(
            id("r"),
            "",
            target(),
            id("altitude"),
            quantity(200.0, "m"),
            quantity(100.0, "m"),
            Severity::Major,
        )
        .unwrap_err();

        assert!(matches!(error, RequirementError::InvertedBounds { .. }));
    }

    #[test]
    fn detects_inversion_after_unit_normalization() {
        // Same-dimension, different-unit interval: 2000 m .. 1 km is
        // inverted once normalized into metres (2000 > 1000).
        let err = Requirement::within(
            id("r"),
            "",
            target(),
            id("d"),
            quantity(2000.0, "m"),
            quantity(1.0, "km"),
            Severity::Major,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            RequirementError::InvertedBounds { min: 2000.0, max: 1000.0, .. }
        ));

        // The convertible-but-consistent version succeeds.
        let ok = Requirement::within(
            id("r"),
            "",
            target(),
            id("d"),
            quantity(1.0, "km"),
            quantity(2000.0, "m"),
            Severity::Major,
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn rejects_mixed_dimensions_in_interval() {
        let error = Requirement::within(
            id("r"),
            "",
            target(),
            id("x"),
            quantity(0.0, "m"),
            quantity(10.0, "s"),
            Severity::Major,
        )
        .unwrap_err();

        assert!(matches!(error, RequirementError::IncompatibleUnits { .. }));
    }

    #[test]
    fn rejects_negative_tolerance() {
        let error = Requirement::approximately(
            id("r"),
            "",
            target(),
            id("x"),
            quantity(10.0, "m"),
            quantity(-1.0, "m"),
            Severity::Major,
        )
        .unwrap_err();

        assert!(matches!(error, RequirementError::NegativeTolerance { .. }));
    }

    #[test]
    fn tolerance_accepts_convertible_units() {
        // 10 m ± 100 cm is dimensionally consistent.
        let requirement = Requirement::approximately(
            id("r"),
            "",
            target(),
            id("x"),
            quantity(10.0, "m"),
            quantity(100.0, "cm"),
            Severity::Minor,
        )
        .expect("convertible tolerance must be accepted");

        assert_eq!(requirement.operator(), Operator::Approximately);
    }

    #[test]
    fn display_condition_is_compact_and_deterministic() {
        let requirement = Requirement::at_most(
            id("r1"),
            "",
            SubjectTarget::component("motor").unwrap(),
            id("temperature"),
            quantity(100.0, "K"),
            Severity::Critical,
        )
        .unwrap();

        assert_eq!(
            requirement.display_condition(),
            "motor.temperature <= 100 K"
        );
    }

    #[test]
    fn construction_is_deterministic() {
        let first = Requirement::at_least(
            id("r"),
            "d",
            target(),
            id("altitude"),
            quantity(100.0, "m"),
            Severity::Critical,
        )
        .unwrap();
        let second = first.clone();
        assert_eq!(first, second);
    }

    #[test]
    fn observation_targets_are_supported() {
        let target = SubjectTarget::observation("telemetry_stream")
            .expect("observation target");
        assert_eq!(target.identifier().as_str(), "telemetry_stream");
    }
}
