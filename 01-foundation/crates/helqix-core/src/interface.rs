//! Typed engineering interfaces for HELQIX Core.
//!
//! An interface describes a boundary through which engineering quantities can
//! be exchanged between components.
//!
//! Foundation owns:
//! - interface identity;
//! - member identity;
//! - physical dimension;
//! - member direction;
//! - structural compatibility.
//!
//! Foundation does NOT own:
//! - runtime values;
//! - state updates;
//! - signal propagation;
//! - execution;
//! - physics;
//! - simulation.
//!
//! Runtime connection and data propagation belong to later HELQIX layers.

use std::collections::BTreeMap;
use std::fmt;

use crate::error::{IncompatibilityReason, InterfaceError};
use crate::id::Identifier;
use crate::units::Dimension;

/// Direction of an interface member.
///
/// `Input` means the owning component consumes the quantity.
///
/// `Output` means the owning component produces the quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InterfaceDirection {
    Input,
    Output,
}

impl fmt::Display for InterfaceDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Input => "input",
            Self::Output => "output",
        })
    }
}

/// One typed member of an engineering interface.
///
/// A member has:
/// - a stable identifier;
/// - a physical dimension;
/// - a direction.
///
/// The dimension is deliberately represented by [`Dimension`] rather than a
/// concrete [`Quantity`]. An interface defines *what kind of quantity* crosses
/// the boundary, not its runtime value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceMember {
    name: Identifier,
    direction: InterfaceDirection,
    dimension: Dimension,
}

impl InterfaceMember {
    /// Creates a typed interface member.
    #[must_use]
    pub const fn new(
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

    /// Returns the member identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the member direction.
    #[must_use]
    pub const fn direction(&self) -> InterfaceDirection {
        self.direction
    }

    /// Returns the physical dimension carried by the member.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        self.dimension
    }
}

impl fmt::Display for InterfaceMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} `{}` [{}]",
            self.direction,
            self.name,
            self.dimension
        )
    }
}

/// A typed engineering interface.
///
/// Members are stored in a [`BTreeMap`] so iteration order is deterministic.
/// Interface definitions therefore have stable structural representation
/// independent of insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    name: Identifier,
    members: BTreeMap<Identifier, InterfaceMember>,
}

impl Interface {
    /// Creates an empty interface definition.
    #[must_use]
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            members: BTreeMap::new(),
        }
    }

    /// Returns the interface identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of members.
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Returns whether the interface contains no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Adds a member.
    ///
    /// Interface member names must be unique within the interface.
    pub fn add_member(
        &mut self,
        member: InterfaceMember,
    ) -> Result<(), InterfaceError> {
        let name = member.name().clone();

        if self.members.contains_key(&name) {
            return Err(InterfaceError::DuplicateMember {
                interface: self.name.as_str().to_owned(),
                name: name.as_str().to_owned(),
            });
        }

        self.members.insert(name, member);
        Ok(())
    }

    /// Returns a member by identifier.
    #[must_use]
    pub fn member(&self, name: &Identifier) -> Option<&InterfaceMember> {
        self.members.get(name)
    }

    /// Returns whether a member exists.
    #[must_use]
    pub fn has_member(&self, name: &Identifier) -> bool {
        self.members.contains_key(name)
    }

    /// Returns all members in deterministic identifier order.
    pub fn members(&self) -> impl Iterator<Item = &InterfaceMember> {
        self.members.values()
    }

    /// Removes a member.
    ///
    /// Returns the removed member when it existed.
    pub fn remove_member(
        &mut self,
        name: &Identifier,
    ) -> Option<InterfaceMember> {
        self.members.remove(name)
    }

    /// Checks whether this interface can connect to another interface.
    ///
    /// Connection compatibility follows the Foundation contract:
    ///
    /// 1. Dimensions must match.
    /// 2. Directions must be complementary.
    ///
    /// Therefore:
    ///
    /// ```text
    /// Output → Input     valid
    /// Input  → Output    valid
    /// Input  → Input     invalid
    /// Output → Output    invalid
    /// ```
    ///
    /// The two interfaces must also expose the same member names. A missing
    /// member is treated as a direction-level incompatibility because the
    /// current Foundation error vocabulary intentionally exposes only
    /// dimension and direction mismatch for interface compatibility.
    pub fn compatible_with(
        &self,
        other: &Self,
    ) -> Result<(), InterfaceError> {
        for (name, from_member) in &self.members {
            let Some(to_member) = other.members.get(name) else {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::DirectionMismatch,
                });
            };

            if from_member.dimension() != to_member.dimension() {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::DimensionMismatch,
                });
            }

            if !directions_are_complementary(
                from_member.direction(),
                to_member.direction(),
            ) {
                return Err(InterfaceError::Incompatible {
                    from: self.name.as_str().to_owned(),
                    to: other.name.as_str().to_owned(),
                    reason: IncompatibilityReason::DirectionMismatch,
                });
            }
        }

        // An interface with no members in the first interface cannot prove
        // compatibility with an interface containing additional members.
        if self.member_count() != other.member_count() {
            return Err(InterfaceError::Incompatible {
                from: self.name.as_str().to_owned(),
                to: other.name.as_str().to_owned(),
                reason: IncompatibilityReason::DirectionMismatch,
            });
        }

        Ok(())
    }

    /// Returns whether two interfaces are compatible without constructing an
    /// error value.
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.compatible_with(other).is_ok()
    }
}

fn directions_are_complementary(
    from: InterfaceDirection,
    to: InterfaceDirection,
) -> bool {
    matches!(
        (from, to),
        (
            InterfaceDirection::Output,
            InterfaceDirection::Input
        ) | (
            InterfaceDirection::Input,
            InterfaceDirection::Output
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("test identifier must be valid")
    }

    fn length_input(name: &str) -> InterfaceMember {
        InterfaceMember::new(
            id(name),
            InterfaceDirection::Input,
            Dimension::LENGTH,
        )
    }

    fn length_output(name: &str) -> InterfaceMember {
        InterfaceMember::new(
            id(name),
            InterfaceDirection::Output,
            Dimension::LENGTH,
        )
    }

    #[test]
    fn member_preserves_identity_direction_and_dimension() {
        let member = InterfaceMember::new(
            id("position"),
            InterfaceDirection::Output,
            Dimension::LENGTH,
        );

        assert_eq!(member.name().as_str(), "position");
        assert_eq!(member.direction(), InterfaceDirection::Output);
        assert_eq!(member.dimension(), Dimension::LENGTH);
    }

    #[test]
    fn new_interface_is_empty() {
        let interface = Interface::new(id("motor_output"));

        assert_eq!(interface.name().as_str(), "motor_output");
        assert_eq!(interface.member_count(), 0);
        assert!(interface.is_empty());
    }

    #[test]
    fn member_can_be_added_and_retrieved() {
        let mut interface = Interface::new(id("motor_output"));
        let member = length_output("shaft_position");

        interface
            .add_member(member)
            .expect("member should be accepted");

        let stored = interface
            .member(&id("shaft_position"))
            .expect("member should exist");

        assert_eq!(stored.direction(), InterfaceDirection::Output);
        assert_eq!(stored.dimension(), Dimension::LENGTH);
    }

    #[test]
    fn duplicate_member_is_rejected() {
        let mut interface = Interface::new(id("motor_output"));

        interface
            .add_member(length_output("position"))
            .expect("first member should be accepted");

        let error = interface
            .add_member(length_input("position"))
            .expect_err("duplicate member must fail");

        assert_eq!(
            error,
            InterfaceError::DuplicateMember {
                interface: "motor_output".to_owned(),
                name: "position".to_owned(),
            }
        );
    }

    #[test]
    fn duplicate_detection_is_based_on_name_not_direction() {
        let mut interface = Interface::new(id("boundary"));

        interface
            .add_member(length_output("signal"))
            .expect("first member should be accepted");

        assert!(interface
            .add_member(length_input("signal"))
            .is_err());

        assert_eq!(interface.member_count(), 1);
    }

    #[test]
    fn member_iteration_is_deterministic() {
        let mut interface = Interface::new(id("boundary"));

        interface
            .add_member(length_output("zeta"))
            .expect("member should be accepted");

        interface
            .add_member(length_output("alpha"))
            .expect("member should be accepted");

        interface
            .add_member(length_output("middle"))
            .expect("member should be accepted");

        let names: Vec<_> = interface
            .members()
            .map(|member| member.name().as_str())
            .collect();

        assert_eq!(names, vec!["alpha", "middle", "zeta"]);
    }

    #[test]
    fn output_to_input_same_dimension_is_compatible() {
        let mut output = Interface::new(id("source"));
        output
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let mut input = Interface::new(id("sink"));
        input
            .add_member(length_input("position"))
            .expect("member should be accepted");

        assert!(output.compatible_with(&input).is_ok());
        assert!(input.compatible_with(&output).is_ok());
    }

    #[test]
    fn input_to_input_is_directionally_incompatible() {
        let mut first = Interface::new(id("first"));
        first
            .add_member(length_input("position"))
            .expect("member should be accepted");

        let mut second = Interface::new(id("second"));
        second
            .add_member(length_input("position"))
            .expect("member should be accepted");

        assert_eq!(
            first.compatible_with(&second),
            Err(InterfaceError::Incompatible {
                from: "first".to_owned(),
                to: "second".to_owned(),
                reason: IncompatibilityReason::DirectionMismatch,
            })
        );
    }

    #[test]
    fn output_to_output_is_directionally_incompatible() {
        let mut first = Interface::new(id("first"));
        first
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let mut second = Interface::new(id("second"));
        second
            .add_member(length_output("position"))
            .expect("member should be accepted");

        assert_eq!(
            first.compatible_with(&second),
            Err(InterfaceError::Incompatible {
                from: "first".to_owned(),
                to: "second".to_owned(),
                reason: IncompatibilityReason::DirectionMismatch,
            })
        );
    }

    #[test]
    fn different_dimensions_are_incompatible() {
        let mut source = Interface::new(id("source"));
        source
            .add_member(InterfaceMember::new(
                id("value"),
                InterfaceDirection::Output,
                Dimension::LENGTH,
            ))
            .expect("member should be accepted");

        let mut sink = Interface::new(id("sink"));
        sink.add_member(InterfaceMember::new(
            id("value"),
            InterfaceDirection::Input,
            Dimension::MASS,
        ))
        .expect("member should be accepted");

        assert_eq!(
            source.compatible_with(&sink),
            Err(InterfaceError::Incompatible {
                from: "source".to_owned(),
                to: "sink".to_owned(),
                reason: IncompatibilityReason::DimensionMismatch,
            })
        );
    }

    #[test]
    fn missing_member_is_incompatible() {
        let mut source = Interface::new(id("source"));
        source
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let sink = Interface::new(id("sink"));

        assert_eq!(
            source.compatible_with(&sink),
            Err(InterfaceError::Incompatible {
                from: "source".to_owned(),
                to: "sink".to_owned(),
                reason: IncompatibilityReason::DirectionMismatch,
            })
        );
    }

    #[test]
    fn additional_member_is_incompatible() {
        let mut source = Interface::new(id("source"));
        source
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let mut sink = Interface::new(id("sink"));
        sink.add_member(length_input("position"))
            .expect("member should be accepted");
        sink.add_member(length_input("velocity"))
            .expect("member should be accepted");

        assert_eq!(
            source.compatible_with(&sink),
            Err(InterfaceError::Incompatible {
                from: "source".to_owned(),
                to: "sink".to_owned(),
                reason: IncompatibilityReason::DirectionMismatch,
            })
        );
    }

    #[test]
    fn incompatible_check_is_symmetric_for_direction() {
        let mut output = Interface::new(id("output"));
        output
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let mut input = Interface::new(id("input"));
        input
            .add_member(length_input("position"))
            .expect("member should be accepted");

        assert!(output.is_compatible_with(&input));
        assert!(input.is_compatible_with(&output));
    }

    #[test]
    fn remove_member_returns_removed_definition() {
        let mut interface = Interface::new(id("boundary"));

        interface
            .add_member(length_output("position"))
            .expect("member should be accepted");

        let removed = interface
            .remove_member(&id("position"))
            .expect("member should exist");

        assert_eq!(removed.name().as_str(), "position");
        assert!(interface.is_empty());
    }

    #[test]
    fn removing_unknown_member_is_safe() {
        let mut interface = Interface::new(id("boundary"));

        assert!(interface.remove_member(&id("missing")).is_none());
    }

    #[test]
    fn member_display_is_structured() {
        let member = length_output("position");

        assert_eq!(member.to_string(), "output `position` [L]");
    }

    #[test]
    fn direction_display_is_stable() {
        assert_eq!(InterfaceDirection::Input.to_string(), "input");
        assert_eq!(InterfaceDirection::Output.to_string(), "output");
    }

    #[test]
    fn compatibility_error_display_matches_core_contract() {
        let error = InterfaceError::Incompatible {
            from: "motor_output".to_owned(),
            to: "controller_input".to_owned(),
            reason: IncompatibilityReason::DimensionMismatch,
        };

        assert_eq!(
            error.to_string(),
            "interface `motor_output` cannot connect to \
             `controller_input`: dimension mismatch"
        );
    }
}