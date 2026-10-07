//! Engineering interface definitions for HELQIX Layer 03 — Engineering.
//!
//! An [`Interface`] defines a typed structural boundary through which
//! engineering elements can exchange or connect engineering quantities.
//!
//! Example conceptual interfaces:
//!
//! ```text
//! Motor.output.thrust   ↔   Vehicle.input.thrust
//! Battery.output.voltage ↔   Motor.input.voltage
//! ```
//!
//! An interface definition describes **structure and compatibility**. It
//! does not transfer data, carry runtime values, emit signals, manage
//! sockets, or perform execution. It contains no physics, no equations,
//! no unit conversion, no AI, and no quantum logic.
//!
//! Compatibility between two interfaces is purely structural:
//!
//! - matching member names;
//! - matching member dimensions;
//! - complementary member directions (`Output` ↔ `Input`).
//!
//! Higher-level model validation (in a later layer) is responsible for
//! determining how interfaces connect to components, parameters, and
//! engineering models. This file deliberately does not depend on
//! `ParameterSet`, `EquationSet`, `Reality`, or any runtime state.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::units::Dimension;

/// Direction of an [`InterfaceMember`] across an interface boundary.
///
/// A direction is a structural role, not a runtime property:
///
/// - [`InterfaceDirection::Input`] consumes a quantity across the boundary;
/// - [`InterfaceDirection::Output`] produces a quantity across the boundary.
///
/// Two members with the same name are only compatible when their
/// directions are complementary (`Input` ↔ `Output`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterfaceDirection {
    /// An input port consumed by the owning engineering element.
    Input,
    /// An output port produced by the owning engineering element.
    Output,
}

/// A single typed engineering port belonging to an [`Interface`].
///
/// A member is identified by its [`Identifier`], carries an
/// [`InterfaceDirection`], and declares an engineering [`Dimension`].
/// It never contains a runtime value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceMember {
    name: Identifier,
    direction: InterfaceDirection,
    dimension: Dimension,
}

impl InterfaceMember {
    /// Creates a new interface member.
    ///
    /// The [`Identifier`] already enforces identifier validity, and the
    /// [`Dimension`] is a Foundation-owned value. No redundant validation
    /// is performed here.
    pub fn new(
        name: Identifier,
        direction: InterfaceDirection,
        dimension: Dimension,
    ) -> Self {
        Self {
            name,
            direction,
            dimension,
        }
    }

    /// Returns the member's identifier.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the member's direction.
    pub fn direction(&self) -> InterfaceDirection {
        self.direction
    }

    /// Returns the member's engineering dimension.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }
}

/// Error type for interface construction, mutation, and compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceError {
    /// A member with the same identifier is already present.
    DuplicateMember {
        /// Name of the interface.
        interface: String,
        /// Name of the duplicated member.
        name: String,
    },
    /// Two interfaces are structurally incompatible.
    Incompatible {
        /// Name of the source interface.
        from: String,
        /// Name of the target interface.
        to: String,
        /// Why the interfaces are incompatible.
        reason: IncompatibilityReason,
    },
    /// A structural invariant of an interface was violated.
    InvalidInterface {
        /// Name of the interface.
        interface: String,
        /// Human-readable description of the violated invariant.
        detail: String,
    },
}

/// Structural reason why two interfaces are incompatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncompatibilityReason {
    /// A member present in one interface is absent from the other.
    MissingMember,
    /// Members sharing a name declare different dimensions.
    DimensionMismatch,
    /// Members sharing a name declare non-complementary directions.
    DirectionMismatch,
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceError::DuplicateMember { interface, name } => {
                write!(f, "interface {interface} already declares member {name}")
            }
            InterfaceError::Incompatible { from, to, reason } => {
                let reason = match reason {
                    IncompatibilityReason::MissingMember => "missing member",
                    IncompatibilityReason::DimensionMismatch => "dimension mismatch",
                    IncompatibilityReason::DirectionMismatch => "direction mismatch",
                };
                write!(f, "interface {from} is incompatible with {to}: {reason}")
            }
            InterfaceError::InvalidInterface { interface, detail } => {
                write!(f, "invalid interface {interface}: {detail}")
            }
        }
    }
}

impl Error for InterfaceError {}

/// A typed structural boundary made of named [`InterfaceMember`] ports.
///
/// Members are held in a [`BTreeMap`] keyed on [`Identifier`], giving
/// deterministic iteration order. Duplicate member names are rejected.
///
/// An interface may contain:
///
/// - only inputs;
/// - only outputs;
/// - a mix of inputs and outputs;
/// - zero members.
///
/// An `Interface` holds no runtime values and performs no execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    name: Identifier,
    members: BTreeMap<Identifier, InterfaceMember>,
}

impl Interface {
    /// Creates an empty interface with the given name.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            members: BTreeMap::new(),
        }
    }

    /// Returns the interface's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of members declared by the interface.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Returns `true` if the interface declares no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Adds a member to the interface.
    ///
    /// The member's own [`Identifier`] is used as the canonical key.
    /// Duplicate names are rejected with
    /// [`InterfaceError::DuplicateMember`]. Existing members are never
    /// overwritten.
    pub fn add_member(&mut self, member: InterfaceMember) -> Result<(), InterfaceError> {
        if self.members.contains_key(member.name()) {
            return Err(InterfaceError::DuplicateMember {
                interface: self.name.as_str().to_owned(),
                name: member.name().as_str().to_owned(),
            });
        }

        let key = member.name().clone();
        self.members.insert(key, member);
        Ok(())
    }

    /// Returns an immutable reference to the member with the given name.
    pub fn member(&self, name: &Identifier) -> Option<&InterfaceMember> {
        self.members.get(name)
    }

    /// Returns `true` if a member with the given name is declared.
    pub fn has_member(&self, name: &Identifier) -> bool {
        self.members.contains_key(name)
    }

    /// Iterates over the interface's members in deterministic order.
    pub fn members(&self) -> impl Iterator<Item = &InterfaceMember> {
        self.members.values()
    }

    /// Removes and returns the member with the given name, if present.
    pub fn remove_member(&mut self, name: &Identifier) -> Option<InterfaceMember> {
        self.members.remove(name)
    }

    /// Determines whether this interface is structurally compatible with
    /// another interface.
    ///
    /// Compatibility requires:
    ///
    /// 1. identical member name sets;
    /// 2. identical dimensions on corresponding members;
    /// 3. complementary directions on corresponding members
    ///    (`Input` ↔ `Output`).
    ///
    /// No runtime values, unit conversion, equations, or physics are
    /// consulted. Only structural properties of the definitions are used.
    pub fn compatible_with(&self, other: &Interface) -> Result<(), InterfaceError> {
        for (name, member) in &self.members {
            let Some(other_member) = other.members.get(name) else {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::MissingMember,
                });
            };

            if member.dimension() != other_member.dimension() {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::DimensionMismatch,
                });
            }

            if member.direction() == other_member.direction() {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::DirectionMismatch,
                });
            }
        }

        for name in other.members.keys() {
            if !self.members.contains_key(name) {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::MissingMember,
                });
            }
        }

        Ok(())
    }

    /// Returns `true` if [`Interface::compatible_with`] succeeds.
    pub fn is_compatible_with(&self, other: &Interface) -> bool {
        self.compatible_with(other).is_ok()
    }

    /// Performs structural validation of the interface definition.
    ///
    /// Enforced invariants:
    ///
    /// 1. Every member key equals the corresponding member's name.
    ///
    /// Identifier validity and member uniqueness are already enforced by
    /// the [`Identifier`] type and the underlying [`BTreeMap`]. No
    /// physical, dimensional, or naming-convention rules are applied.
    pub fn validate(&self) -> Result<(), InterfaceError> {
        for (key, member) in &self.members {
            if key != member.name() {
                return Err(InterfaceError::InvalidInterface {
                    interface: self.name.as_str().to_owned(),
                    detail: format!(
                        "member key does not match member name: {}",
                        member.name().as_str()
                    ),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use helqix_core::units::Unit;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn length_dimension() -> Dimension {
        Unit::parse("m")
            .expect("m must exist in Foundation units")
            .dimension()
            .clone()
    }

    fn mass_dimension() -> Dimension {
        Unit::parse("kg")
            .expect("kg must exist in Foundation units")
            .dimension()
            .clone()
    }

    fn member(name: &str, direction: InterfaceDirection, dimension: Dimension) -> InterfaceMember {
        InterfaceMember::new(ident(name), direction, dimension)
    }

    #[test]
    fn create_empty_interface() {
        let interface = Interface::new(ident("vehicle_interface"));
        assert_eq!(interface.name().as_str(), "vehicle_interface");
        assert_eq!(interface.member_count(), 0);
        assert!(interface.is_empty());
    }

    #[test]
    fn create_interface_member() {
        let m = member("thrust", InterfaceDirection::Input, mass_dimension());
        assert_eq!(m.name().as_str(), "thrust");
        assert_eq!(m.direction(), InterfaceDirection::Input);
        assert_eq!(m.dimension(), &mass_dimension());
    }

    #[test]
    fn add_member() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("member added");

        assert_eq!(interface.member_count(), 1);
        assert!(!interface.is_empty());
    }

    #[test]
    fn duplicate_member_rejected() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("first add succeeds");

        let err = interface
            .add_member(member("thrust", InterfaceDirection::Output, mass_dimension()))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            InterfaceError::DuplicateMember {
                interface: "vehicle_interface".to_owned(),
                name: "thrust".to_owned(),
            }
        );
        assert_eq!(interface.member_count(), 1);
    }

    #[test]
    fn member_lookup() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("member added");

        let found = interface
            .member(&ident("thrust"))
            .expect("present");
        assert_eq!(found.name().as_str(), "thrust");

        assert!(interface.member(&ident("voltage")).is_none());
    }

    #[test]
    fn has_member() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("member added");

        assert!(interface.has_member(&ident("thrust")));
        assert!(!interface.has_member(&ident("voltage")));
    }

    #[test]
    fn member_count() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        assert_eq!(interface.member_count(), 0);

        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("add thrust");
        assert_eq!(interface.member_count(), 1);

        interface
            .add_member(member("voltage", InterfaceDirection::Input, mass_dimension()))
            .expect("add voltage");
        assert_eq!(interface.member_count(), 2);
    }

    #[test]
    fn deterministic_member_ordering() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("zeta", InterfaceDirection::Input, mass_dimension()))
            .expect("add zeta");
        interface
            .add_member(member("alpha", InterfaceDirection::Input, mass_dimension()))
            .expect("add alpha");
        interface
            .add_member(member("motor", InterfaceDirection::Input, mass_dimension()))
            .expect("add motor");

        let order: Vec<&str> = interface.members().map(|m| m.name().as_str()).collect();
        assert_eq!(order, vec!["alpha", "motor", "zeta"]);
    }

    #[test]
    fn remove_member() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member("thrust", InterfaceDirection::Input, mass_dimension()))
            .expect("add thrust");

        let removed = interface
            .remove_member(&ident("thrust"))
            .expect("removed");
        assert_eq!(removed.name().as_str(), "thrust");
        assert!(interface.is_empty());

        assert!(interface.remove_member(&ident("thrust")).is_none());
    }

    #[test]
    fn input_member() {
        let m = member("thrust", InterfaceDirection::Input, mass_dimension());
        assert_eq!(m.direction(), InterfaceDirection::Input);
    }

    #[test]
    fn output_member() {
        let m = member("thrust", InterfaceDirection::Output, mass_dimension());
        assert_eq!(m.direction(), InterfaceDirection::Output);
    }

    #[test]
    fn compatible_interfaces_with_complementary_directions() {
        let dim = mass_dimension();

        let mut producer = Interface::new(ident("motor_output"));
        producer
            .add_member(member("thrust", InterfaceDirection::Output, dim.clone()))
            .expect("add thrust");

        let mut consumer = Interface::new(ident("vehicle_input"));
        consumer
            .add_member(member("thrust", InterfaceDirection::Input, dim))
            .expect("add thrust");

        assert!(producer.compatible_with(&consumer).is_ok());
        assert!(consumer.compatible_with(&producer).is_ok());
    }

    #[test]
    fn incompatible_input_input() {
        let dim = mass_dimension();

        let mut a = Interface::new(ident("a"));
        a.add_member(member("thrust", InterfaceDirection::Input, dim.clone()))
            .expect("add thrust");

        let mut b = Interface::new(ident("b"));
        b.add_member(member("thrust", InterfaceDirection::Input, dim))
            .expect("add thrust");

        let err = a
            .compatible_with(&b)
            .expect_err("input/input incompatible");

        match err {
            InterfaceError::Incompatible { reason, .. } => {
                assert_eq!(reason, IncompatibilityReason::DirectionMismatch);
            }
            other => panic!("expected Incompatible, got {other:?}"),
        }
    }

    #[test]
    fn incompatible_output_output() {
        let dim = mass_dimension();

        let mut a = Interface::new(ident("a"));
        a.add_member(member("thrust", InterfaceDirection::Output, dim.clone()))
            .expect("add thrust");

        let mut b = Interface::new(ident("b"));
        b.add_member(member("thrust", InterfaceDirection::Output, dim))
            .expect("add thrust");

        let err = a
            .compatible_with(&b)
            .expect_err("output/output incompatible");

        match err {
            InterfaceError::Incompatible { reason, .. } => {
                assert_eq!(reason, IncompatibilityReason::DirectionMismatch);
            }
            other => panic!("expected Incompatible, got {other:?}"),
        }
    }

    #[test]
    fn incompatible_dimensions() {
        let mut a = Interface::new(ident("a"));
        a.add_member(member(
            "quantity",
            InterfaceDirection::Output,
            length_dimension(),
        ))
        .expect("add quantity");

        let mut b = Interface::new(ident("b"));
        b.add_member(member(
            "quantity",
            InterfaceDirection::Input,
            mass_dimension(),
        ))
        .expect("add quantity");

        let err = a
            .compatible_with(&b)
            .expect_err("different dimensions incompatible");

        match err {
            InterfaceError::Incompatible { reason, .. } => {
                assert_eq!(reason, IncompatibilityReason::DimensionMismatch);
            }
            other => panic!("expected Incompatible, got {other:?}"),
        }
    }

    #[test]
    fn incompatible_missing_member() {
        let dim = mass_dimension();

        let mut a = Interface::new(ident("a"));
        a.add_member(member("thrust", InterfaceDirection::Output, dim.clone()))
            .expect("add thrust");

        let mut b = Interface::new(ident("b"));
        b.add_member(member("thrust", InterfaceDirection::Input, dim.clone()))
            .expect("add thrust");
        b.add_member(member("voltage", InterfaceDirection::Input, dim))
            .expect("add voltage");

        let err = a
            .compatible_with(&b)
            .expect_err("missing member incompatible");

        match err {
            InterfaceError::Incompatible { reason, .. } => {
                assert_eq!(reason, IncompatibilityReason::MissingMember);
            }
            other => panic!("expected Incompatible, got {other:?}"),
        }
    }

    #[test]
    fn is_compatible_with_true_for_compatible_interfaces() {
        let dim = mass_dimension();

        let mut producer = Interface::new(ident("producer"));
        producer
            .add_member(member("thrust", InterfaceDirection::Output, dim.clone()))
            .expect("add thrust");

        let mut consumer = Interface::new(ident("consumer"));
        consumer
            .add_member(member("thrust", InterfaceDirection::Input, dim))
            .expect("add thrust");

        assert!(producer.is_compatible_with(&consumer));
        assert!(consumer.is_compatible_with(&producer));
    }

    #[test]
    fn is_compatible_with_false_for_incompatible_interfaces() {
        let dim = mass_dimension();

        let mut a = Interface::new(ident("a"));
        a.add_member(member("thrust", InterfaceDirection::Output, dim.clone()))
            .expect("add thrust");

        let mut b = Interface::new(ident("b"));
        b.add_member(member("thrust", InterfaceDirection::Output, dim))
            .expect("add thrust");

        assert!(!a.is_compatible_with(&b));
        assert!(!b.is_compatible_with(&a));
    }

    #[test]
    fn valid_interface_passes_validation() {
        let mut interface = Interface::new(ident("vehicle_interface"));
        interface
            .add_member(member(
                "thrust",
                InterfaceDirection::Input,
                mass_dimension(),
            ))
            .expect("add thrust");
        interface
            .add_member(member(
                "voltage",
                InterfaceDirection::Output,
                mass_dimension(),
            ))
            .expect("add voltage");

        assert!(interface.validate().is_ok());
    }

    #[test]
    fn interface_equality_behaves_deterministically() {
        let dim = mass_dimension();

        let mut a = Interface::new(ident("iface"));
        a.add_member(member("thrust", InterfaceDirection::Input, dim.clone()))
            .expect("add thrust");
        a.add_member(member("voltage", InterfaceDirection::Output, dim.clone()))
            .expect("add voltage");

        let mut b = Interface::new(ident("iface"));
        b.add_member(member("voltage", InterfaceDirection::Output, dim.clone()))
            .expect("add voltage");
        b.add_member(member("thrust", InterfaceDirection::Input, dim))
            .expect("add thrust");

        assert_eq!(a, b);
    }
}