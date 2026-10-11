//! Structured error vocabulary for HELQIX Core.
//!
//! This module defines every failure the Foundation layer can report. It is
//! deliberately limited to Foundation concepts: identifiers, units, quantities,
//! properties, component definitions, behaviour contracts, interfaces,
//! constraints, invariants and serialization. Domain-specific failures
//! (motors, batteries, physics, simulation, UI, AI, APIs) belong to later
//! crates, which wrap [`CoreError`] rather than extending it.
//!
//! Design rules:
//! - No dependencies beyond `std`.
//! - Errors are plain data: `Clone`, `PartialEq`, `Send`, `Sync`, `'static`.
//!   No `Box<dyn Error>` payloads, so equality and cloning stay deterministic.
//! - Reasons are enums with typed fields; free-form `String`s only carry
//!   offending input or the rendered name of the definition being validated.
//!   Names are `String` (not identifier types) because `error.rs` sits below
//!   `id.rs`: identifier parsing returns [`IdentifierError`], so this module
//!   must not depend on identifier types.
//! - [`CoreError`] is transparent: its `Display` forwards to the inner error and
//!   its `source()` forwards to the inner error's `source()`, so error-chain
//!   reporters never print the same message twice.

use std::error::Error;
use std::fmt;

/// Result alias used throughout HELQIX Core.
pub type CoreResult<T> = Result<T, CoreError>;

// ---------------------------------------------------------------------------
// Shared vocabulary
// ---------------------------------------------------------------------------

/// Classification of a floating point value that is not finite.
///
/// Stored instead of the raw `f64` so errors keep a total equality
/// (`NaN != NaN` would otherwise make identical errors compare unequal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonFiniteKind {
    Nan,
    PositiveInfinity,
    NegativeInfinity,
}

impl NonFiniteKind {
    /// Classifies `value`, returning `None` when it is finite.
    #[must_use]
    pub fn classify(value: f64) -> Option<Self> {
        if value.is_nan() {
            Some(Self::Nan)
        } else if value == f64::INFINITY {
            Some(Self::PositiveInfinity)
        } else if value == f64::NEG_INFINITY {
            Some(Self::NegativeInfinity)
        } else {
            None
        }
    }
}

impl fmt::Display for NonFiniteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Nan => "NaN",
            Self::PositiveInfinity => "positive infinity",
            Self::NegativeInfinity => "negative infinity",
        })
    }
}

/// The kind of named member a definition can reference or declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
    Property,
    Interface,
    Behaviour,
}

impl fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Property => "property",
            Self::Interface => "interface",
            Self::Behaviour => "behaviour",
        })
    }
}

// ---------------------------------------------------------------------------
// Identifier
// ---------------------------------------------------------------------------

/// An identifier failed validation.
///
/// `position` values are zero-based indices counted in Unicode scalar values
/// (`char`s), not bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IdentifierError {
    /// The identifier is the empty string.
    Empty,
    /// The identifier exceeds the maximum permitted length (in `char`s).
    TooLong { length: usize, max: usize },
    /// The first character is not permitted at the start of an identifier.
    InvalidStart { character: char },
    /// A character outside the permitted set was found.
    InvalidCharacter { character: char, position: usize },
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("identifier is empty"),
            Self::TooLong { length, max } => write!(
                f,
                "identifier is {length} characters long, maximum is {max}"
            ),
            Self::InvalidStart { character } => {
                write!(f, "identifier cannot start with {character:?}")
            }
            Self::InvalidCharacter {
                character,
                position,
            } => write!(
                f,
                "identifier contains invalid character {character:?} at position {position}"
            ),
        }
    }
}

impl Error for IdentifierError {}

// ---------------------------------------------------------------------------
// Unit
// ---------------------------------------------------------------------------

/// A unit symbol or unit relationship was invalid.
///
/// Dimensions are carried as their rendered form because the dimension type
/// lives in a later Foundation module that depends on this one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnitError {
    /// The unit symbol is the empty string.
    EmptySymbol,
    /// The symbol is syntactically invalid; `position` is a zero-based `char` index.
    MalformedSymbol { symbol: String, position: usize },
    /// The symbol is well-formed but not registered.
    UnknownSymbol { symbol: String },
    /// Two units that must share a dimension do not.
    DimensionMismatch { expected: String, found: String },
}

impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySymbol => f.write_str("unit symbol is empty"),
            Self::MalformedSymbol { symbol, position } => write!(
                f,
                "unit symbol `{symbol}` is malformed at position {position}"
            ),
            Self::UnknownSymbol { symbol } => write!(f, "unknown unit symbol `{symbol}`"),
            Self::DimensionMismatch { expected, found } => write!(
                f,
                "dimension mismatch: expected `{expected}`, found `{found}`"
            ),
        }
    }
}

impl Error for UnitError {}

// ---------------------------------------------------------------------------
// Quantity
// ---------------------------------------------------------------------------

/// A numeric quantity failed validation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum QuantityError {
    /// The value is NaN or infinite.
    NonFinite(NonFiniteKind),
    /// A finite value lies outside the permitted inclusive range.
    OutOfRange { value: f64, min: f64, max: f64 },
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(kind) => write!(f, "quantity is not finite: {kind}"),
            Self::OutOfRange { value, min, max } => write!(
                f,
                "value {value} is outside the permitted range [{min}, {max}]"
            ),
        }
    }
}

impl Error for QuantityError {}

// ---------------------------------------------------------------------------
// Property
// ---------------------------------------------------------------------------

/// A property declaration or assignment was invalid.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PropertyError {
    /// The same property name was declared twice.
    DuplicateName { name: String },
    /// A value was supplied for a property that was never declared.
    Undeclared { name: String },
    /// A required property has no value.
    MissingRequired { name: String },
    /// The property's value failed quantity validation. The cause is exposed
    /// through [`Error::source`], not repeated in `Display`.
    InvalidValue { name: String, source: QuantityError },
}

impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName { name } => write!(f, "property `{name}` is declared more than once"),
            Self::Undeclared { name } => write!(f, "property `{name}` is not declared"),
            Self::MissingRequired { name } => {
                write!(f, "required property `{name}` has no value")
            }
            Self::InvalidValue { name, .. } => {
                write!(f, "property `{name}` has an invalid value")
            }
        }
    }
}

impl Error for PropertyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidValue { source, .. } => Some(source as &(dyn Error + 'static)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Component definition
// ---------------------------------------------------------------------------

/// A component definition was structurally invalid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ComponentError {
    /// The component declares two members of the same kind with the same name.
    DuplicateMember {
        component: String,
        kind: ReferenceKind,
        name: String,
    },
    /// The component references a member that it does not declare.
    UnresolvedReference {
        component: String,
        kind: ReferenceKind,
        name: String,
    },
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMember {
                component,
                kind,
                name,
            } => write!(
                f,
                "component `{component}` declares {kind} `{name}` more than once"
            ),
            Self::UnresolvedReference {
                component,
                kind,
                name,
            } => write!(
                f,
                "component `{component}` references undeclared {kind} `{name}`"
            ),
        }
    }
}

impl Error for ComponentError {}

// ---------------------------------------------------------------------------
// Behaviour contract
// ---------------------------------------------------------------------------

/// A behaviour contract was invalid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BehaviourError {
    /// The contract states nothing (no clauses).
    EmptyContract { behaviour: String },
    /// A clause refers to a member the contract cannot resolve.
    UnresolvedReference {
        behaviour: String,
        kind: ReferenceKind,
        name: String,
    },
}

impl fmt::Display for BehaviourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContract { behaviour } => {
                write!(f, "behaviour contract `{behaviour}` has no clauses")
            }
            Self::UnresolvedReference {
                behaviour,
                kind,
                name,
            } => write!(
                f,
                "behaviour contract `{behaviour}` references undeclared {kind} `{name}`"
            ),
        }
    }
}

impl Error for BehaviourError {}

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

/// Why two interfaces cannot be connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncompatibilityReason {
    DimensionMismatch,
    DirectionMismatch,
}

impl fmt::Display for IncompatibilityReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DimensionMismatch => "dimension mismatch",
            Self::DirectionMismatch => "direction mismatch",
        })
    }
}

/// An interface definition or connection was invalid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InterfaceError {
    /// The interface declares the same member name twice.
    DuplicateMember { interface: String, name: String },
    /// Two interfaces cannot be connected.
    Incompatible {
        from: String,
        to: String,
        reason: IncompatibilityReason,
    },
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMember { interface, name } => write!(
                f,
                "interface `{interface}` declares member `{name}` more than once"
            ),
            Self::Incompatible { from, to, reason } => write!(
                f,
                "interface `{from}` cannot connect to `{to}`: {reason}"
            ),
        }
    }
}

impl Error for InterfaceError {}

// ---------------------------------------------------------------------------
// Constraint
// ---------------------------------------------------------------------------

/// A constraint *definition* was invalid.
///
/// Evaluating a valid constraint against runtime state is not a Foundation
/// concern and is intentionally absent.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConstraintError {
    /// The lower bound is greater than the upper bound (both finite).
    InvertedBounds { constraint: String, min: f64, max: f64 },
    /// A bound is NaN or infinite.
    NonFiniteBound {
        constraint: String,
        bound: NonFiniteKind,
    },
    /// The constraint refers to a member that is not declared.
    UnresolvedReference {
        constraint: String,
        kind: ReferenceKind,
        name: String,
    },
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvertedBounds {
                constraint,
                min,
                max,
            } => write!(
                f,
                "constraint `{constraint}` has lower bound {min} greater than upper bound {max}"
            ),
            Self::NonFiniteBound { constraint, bound } => write!(
                f,
                "constraint `{constraint}` has a non-finite bound: {bound}"
            ),
            Self::UnresolvedReference {
                constraint,
                kind,
                name,
            } => write!(
                f,
                "constraint `{constraint}` references undeclared {kind} `{name}`"
            ),
        }
    }
}

impl Error for ConstraintError {}

// ---------------------------------------------------------------------------
// Invariant
// ---------------------------------------------------------------------------

/// An internal Foundation invariant was broken.
///
/// This signals a defect in the caller or in HELQIX itself, never ordinary bad
/// input. `invariant` is a compile-time name (for example
/// `"component.members_sorted"`) so it can be matched and grepped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvariantViolation {
    pub invariant: &'static str,
    pub detail: String,
}

impl InvariantViolation {
    #[must_use]
    pub fn new(invariant: &'static str, detail: impl Into<String>) -> Self {
        Self {
            invariant,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invariant `{}` violated: {}", self.invariant, self.detail)
    }
}

impl Error for InvariantViolation {}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Whether a failure happened while writing or reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerializationDirection {
    Serialize,
    Deserialize,
}

impl fmt::Display for SerializationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Serialize => "serialize",
            Self::Deserialize => "deserialize",
        })
    }
}

/// One-based position inside serialized input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// A (de)serialization failure.
///
/// Foundation does not depend on any serialization library. Later crates that
/// use one (serde_json, etc.) convert its error into this type, copying the
/// format name, location and message; the original error object is not kept so
/// this type stays `Clone + PartialEq`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SerializationError {
    pub direction: SerializationDirection,
    /// Static format name, for example `"json"`.
    pub format: &'static str,
    pub location: Option<SourceLocation>,
    pub message: String,
}

impl SerializationError {
    #[must_use]
    pub fn new(
        direction: SerializationDirection,
        format: &'static str,
        location: Option<SourceLocation>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            direction,
            format,
            location,
            message: message.into(),
        }
    }
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to {} {}", self.direction, self.format)?;
        if let Some(location) = self.location {
            write!(f, " at {location}")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl Error for SerializationError {}

// ---------------------------------------------------------------------------
// CoreError
// ---------------------------------------------------------------------------

/// Any failure produced by HELQIX Core.
///
/// Later crates should wrap this type (for example with a `#[from] CoreError`
/// variant) instead of extending it. It is `non_exhaustive`, so downstream
/// matches need a wildcard arm and new categories are not breaking changes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CoreError {
    Identifier(IdentifierError),
    Unit(UnitError),
    Quantity(QuantityError),
    Property(PropertyError),
    Component(ComponentError),
    Behaviour(BehaviourError),
    Interface(InterfaceError),
    Constraint(ConstraintError),
    Invariant(InvariantViolation),
    Serialization(SerializationError),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(e) => fmt::Display::fmt(e, f),
            Self::Unit(e) => fmt::Display::fmt(e, f),
            Self::Quantity(e) => fmt::Display::fmt(e, f),
            Self::Property(e) => fmt::Display::fmt(e, f),
            Self::Component(e) => fmt::Display::fmt(e, f),
            Self::Behaviour(e) => fmt::Display::fmt(e, f),
            Self::Interface(e) => fmt::Display::fmt(e, f),
            Self::Constraint(e) => fmt::Display::fmt(e, f),
            Self::Invariant(e) => fmt::Display::fmt(e, f),
            Self::Serialization(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl Error for CoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Identifier(e) => e.source(),
            Self::Unit(e) => e.source(),
            Self::Quantity(e) => e.source(),
            Self::Property(e) => e.source(),
            Self::Component(e) => e.source(),
            Self::Behaviour(e) => e.source(),
            Self::Interface(e) => e.source(),
            Self::Constraint(e) => e.source(),
            Self::Invariant(e) => e.source(),
            Self::Serialization(e) => e.source(),
        }
    }
}

macro_rules! impl_from_for_core_error {
    ($($source:ty => $variant:ident),+ $(,)?) => {
        $(
            impl From<$source> for CoreError {
                fn from(error: $source) -> Self {
                    Self::$variant(error)
                }
            }
        )+
    };
}

impl_from_for_core_error! {
    IdentifierError => Identifier,
    UnitError => Unit,
    QuantityError => Quantity,
    PropertyError => Property,
    ComponentError => Component,
    BehaviourError => Behaviour,
    InterfaceError => Interface,
    ConstraintError => Constraint,
    InvariantViolation => Invariant,
    SerializationError => Serialization,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync_static<T: Send + Sync + 'static>() {}

    #[test]
    fn core_error_is_send_sync_and_static() {
        assert_send_sync_static::<CoreError>();
    }

    #[test]
    fn classify_distinguishes_every_non_finite_kind() {
        assert_eq!(NonFiniteKind::classify(f64::NAN), Some(NonFiniteKind::Nan));
        assert_eq!(
            NonFiniteKind::classify(f64::INFINITY),
            Some(NonFiniteKind::PositiveInfinity)
        );
        assert_eq!(
            NonFiniteKind::classify(f64::NEG_INFINITY),
            Some(NonFiniteKind::NegativeInfinity)
        );
        assert_eq!(NonFiniteKind::classify(0.0), None);
        assert_eq!(NonFiniteKind::classify(f64::MAX), None);
        assert_eq!(NonFiniteKind::classify(f64::MIN_POSITIVE), None);
    }

    #[test]
    fn non_finite_errors_with_nan_compare_equal() {
        let a = QuantityError::NonFinite(NonFiniteKind::Nan);
        let b = QuantityError::NonFinite(NonFiniteKind::classify(f64::NAN).unwrap());
        assert_eq!(a, b);
    }

    #[test]
    fn from_wraps_each_category_in_its_own_variant() {
        assert!(matches!(
            CoreError::from(IdentifierError::Empty),
            CoreError::Identifier(IdentifierError::Empty)
        ));
        assert!(matches!(
            CoreError::from(UnitError::EmptySymbol),
            CoreError::Unit(UnitError::EmptySymbol)
        ));
        assert!(matches!(
            CoreError::from(QuantityError::NonFinite(NonFiniteKind::Nan)),
            CoreError::Quantity(_)
        ));
        assert!(matches!(
            CoreError::from(PropertyError::Undeclared { name: "x".into() }),
            CoreError::Property(_)
        ));
        assert!(matches!(
            CoreError::from(ComponentError::DuplicateMember {
                component: "c".into(),
                kind: ReferenceKind::Property,
                name: "p".into(),
            }),
            CoreError::Component(_)
        ));
        assert!(matches!(
            CoreError::from(BehaviourError::EmptyContract {
                behaviour: "b".into()
            }),
            CoreError::Behaviour(_)
        ));
        assert!(matches!(
            CoreError::from(InterfaceError::DuplicateMember {
                interface: "i".into(),
                name: "m".into(),
            }),
            CoreError::Interface(_)
        ));
        assert!(matches!(
            CoreError::from(ConstraintError::NonFiniteBound {
                constraint: "k".into(),
                bound: NonFiniteKind::Nan,
            }),
            CoreError::Constraint(_)
        ));
        assert!(matches!(
            CoreError::from(InvariantViolation::new("a.b", "broken")),
            CoreError::Invariant(_)
        ));
        assert!(matches!(
            CoreError::from(SerializationError::new(
                SerializationDirection::Serialize,
                "json",
                None,
                "boom"
            )),
            CoreError::Serialization(_)
        ));
    }

    #[test]
    fn core_error_display_is_transparent() {
        let inner = UnitError::UnknownSymbol {
            symbol: "furlong".to_owned(),
        };
        let wrapped = CoreError::from(inner.clone());
        assert_eq!(wrapped.to_string(), inner.to_string());
        assert!(wrapped.source().is_none());
    }

    #[test]
    fn property_error_exposes_quantity_cause_as_source_without_duplicating_it() {
        let cause = QuantityError::NonFinite(NonFiniteKind::Nan);
        let error = CoreError::from(PropertyError::InvalidValue {
            name: "mass".to_owned(),
            source: cause.clone(),
        });

        let source = error.source().expect("InvalidValue must expose its cause");
        assert_eq!(source.downcast_ref::<QuantityError>(), Some(&cause));
        assert_eq!(error.to_string(), "property `mass` has an invalid value");
        assert!(!error.to_string().contains("NaN"));
    }

    #[test]
    fn property_variants_without_cause_have_no_source() {
        assert!(PropertyError::DuplicateName { name: "a".into() }
            .source()
            .is_none());
        assert!(PropertyError::MissingRequired { name: "a".into() }
            .source()
            .is_none());
    }

    #[test]
    fn identifier_messages_carry_character_and_position() {
        let error = IdentifierError::InvalidCharacter {
            character: '-',
            position: 3,
        };
        assert_eq!(
            error.to_string(),
            "identifier contains invalid character '-' at position 3"
        );
        assert_eq!(IdentifierError::Empty.to_string(), "identifier is empty");
        assert_eq!(
            IdentifierError::TooLong { length: 70, max: 64 }.to_string(),
            "identifier is 70 characters long, maximum is 64"
        );
        assert_eq!(
            IdentifierError::InvalidStart { character: '9' }.to_string(),
            "identifier cannot start with '9'"
        );
    }

    #[test]
    fn out_of_range_reports_value_and_bounds() {
        let error = QuantityError::OutOfRange {
            value: 12.5,
            min: 0.0,
            max: 10.0,
        };
        assert_eq!(
            error.to_string(),
            "value 12.5 is outside the permitted range [0, 10]"
        );
    }

    #[test]
    fn definition_errors_name_the_definition_and_member() {
        let component = ComponentError::UnresolvedReference {
            component: "pump".to_owned(),
            kind: ReferenceKind::Interface,
            name: "inlet".to_owned(),
        };
        assert_eq!(
            component.to_string(),
            "component `pump` references undeclared interface `inlet`"
        );

        let behaviour = BehaviourError::UnresolvedReference {
            behaviour: "start".to_owned(),
            kind: ReferenceKind::Property,
            name: "speed".to_owned(),
        };
        assert_eq!(
            behaviour.to_string(),
            "behaviour contract `start` references undeclared property `speed`"
        );

        let interface = InterfaceError::Incompatible {
            from: "out".to_owned(),
            to: "in".to_owned(),
            reason: IncompatibilityReason::DimensionMismatch,
        };
        assert_eq!(
            interface.to_string(),
            "interface `out` cannot connect to `in`: dimension mismatch"
        );
    }

    #[test]
    fn constraint_errors_report_bounds_and_non_finite_kind() {
        let inverted = ConstraintError::InvertedBounds {
            constraint: "range".to_owned(),
            min: 5.0,
            max: 1.0,
        };
        assert_eq!(
            inverted.to_string(),
            "constraint `range` has lower bound 5 greater than upper bound 1"
        );

        let non_finite = ConstraintError::NonFiniteBound {
            constraint: "range".to_owned(),
            bound: NonFiniteKind::PositiveInfinity,
        };
        assert_eq!(
            non_finite.to_string(),
            "constraint `range` has a non-finite bound: positive infinity"
        );
    }

    #[test]
    fn invariant_violation_keeps_name_and_detail() {
        let violation = InvariantViolation::new("component.members_sorted", "found `b` before `a`");
        assert_eq!(violation.invariant, "component.members_sorted");
        assert_eq!(
            violation.to_string(),
            "invariant `component.members_sorted` violated: found `b` before `a`"
        );
    }

    #[test]
    fn serialization_error_formats_with_and_without_location() {
        let without = SerializationError::new(
            SerializationDirection::Serialize,
            "json",
            None,
            "unsupported value",
        );
        assert_eq!(
            without.to_string(),
            "failed to serialize json: unsupported value"
        );

        let with = SerializationError::new(
            SerializationDirection::Deserialize,
            "json",
            Some(SourceLocation { line: 4, column: 17 }),
            "expected `,`",
        );
        assert_eq!(
            with.to_string(),
            "failed to deserialize json at line 4, column 17: expected `,`"
        );
    }

    #[test]
    fn errors_are_deterministic_values() {
        let error = CoreError::from(ConstraintError::InvertedBounds {
            constraint: "c".to_owned(),
            min: 2.0,
            max: 1.0,
        });
        assert_eq!(error.clone(), error);
        assert_eq!(error.to_string(), error.clone().to_string());
    }
}

