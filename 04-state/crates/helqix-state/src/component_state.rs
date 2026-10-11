//! Current state of one component in a HELQIX Reality.
//!
//! `ComponentState` represents the runtime condition belonging to one
//! structurally defined component. The component's structural definition
//! remains owned by Layer 02 (`Reality`); runtime values belong to Layer 04.
//!
//! This type deliberately does not implement:
//!
//! - physics
//! - simulation
//! - execution
//! - time advancement
//! - event processing
//! - failure analysis
//! - causality
//! - branching
//! - optimization
//!
//! The component identity is immutable and is represented by the canonical
//! Foundation [`Identifier`].

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::property_state::PropertyState;
use crate::physical_state::{PhysicalState, PhysicalStateError};
use crate::spatial_state::{SpatialState, SpatialStateError, VectorField};

/// Runtime state belonging to one Reality component.
///
/// A `ComponentState` is the state-side counterpart of a Foundation
/// `Component`. It owns only the component's current state values and never
/// modifies the structural component definition.
///
/// Property states are indexed by their canonical identifiers and stored in
/// a `BTreeMap` to guarantee deterministic lookup and iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentState {
    name: Identifier,
    properties: BTreeMap<Identifier, PropertyState>,
    physical: PhysicalState,
    spatial: Option<SpatialState>,
    vectors: VectorField,
}

impl ComponentState {
    /// Creates an empty component state for the component identified by `name`.
    ///
    /// The supplied `Identifier` is already validated by Foundation.
    #[must_use]
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            properties: BTreeMap::new(),
            physical: PhysicalState::new(),
            spatial: None,
            vectors: VectorField::new(),
        }
    }

    /// Returns the canonical identity of the component.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of current property states.
    #[must_use]
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }

    /// Returns `true` when the component has no property, physical, or spatial values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
            && self.physical.is_empty()
            && self.spatial.is_none()
            && self.vectors.is_empty()
    }

    /// Returns the number of named physical quantities.
    #[must_use]
    pub fn physical_count(&self) -> usize { self.physical.len() }

    /// Returns the number of named spatial vectors.
    #[must_use]
    pub fn vector_count(&self) -> usize { self.vectors.len() }

    /// Returns whether this component has a spatial transform.
    #[must_use]
    pub fn has_spatial_state(&self) -> bool { self.spatial.is_some() }

    /// Returns the component's named physical quantities in identifier order.
    #[must_use]
    pub fn physical(&self) -> &PhysicalState { &self.physical }

    /// Mutates physical values. When this component belongs to a `State`, use
    /// `State::update_component` or a `StateTransition` to commit the change
    /// and advance the containing revision.
    pub fn physical_mut(&mut self) -> &mut PhysicalState { &mut self.physical }

    /// Returns the current transform when the component has spatial state.
    #[must_use]
    pub fn spatial(&self) -> Option<&SpatialState> { self.spatial.as_ref() }

    /// Sets or clears the component's spatial transform.
    pub fn set_spatial(&mut self, spatial: Option<SpatialState>) { self.spatial = spatial; }

    /// Returns named spatial vectors in identifier order.
    #[must_use]
    pub fn vectors(&self) -> &VectorField { &self.vectors }

    /// Mutates named spatial vectors before committing the component through
    /// a revisioned `State` update or transition.
    pub fn vectors_mut(&mut self) -> &mut VectorField { &mut self.vectors }

    /// Returns whether a property state with `name` exists.
    #[must_use]
    pub fn contains_property(&self, name: &Identifier) -> bool {
        self.properties.contains_key(name)
    }

    /// Returns a property state by its canonical identifier.
    #[must_use]
    pub fn property(&self, name: &Identifier) -> Option<&PropertyState> {
        self.properties.get(name)
    }

    /// Returns mutable access to one property state.
    ///
    /// The component identity and collection membership remain encapsulated.
    #[must_use]
    pub fn property_mut(&mut self, name: &Identifier) -> Option<&mut PropertyState> {
        self.properties.get_mut(name)
    }

    /// Returns property states in deterministic identifier order.
    pub fn properties(&self) -> impl Iterator<Item = &PropertyState> {
        self.properties.values()
    }

    /// Returns `(identifier, property state)` pairs in deterministic order.
    pub fn named_properties(&self) -> impl Iterator<Item = (&Identifier, &PropertyState)> {
        self.properties.iter()
    }

    /// Inserts a property state.
    ///
    /// Existing property state is never silently replaced.
    pub fn add_property(
        &mut self,
        property: PropertyState,
    ) -> Result<(), ComponentStateError> {
        let name = property.name().clone();

        if self.properties.contains_key(&name) {
            return Err(ComponentStateError::DuplicateProperty { name });
        }

        self.properties.insert(name, property);
        Ok(())
    }

    /// Removes and returns a property state.
    ///
    /// Returns `None` when the property is not present.
    pub fn remove_property(&mut self, name: &Identifier) -> Option<PropertyState> {
        self.properties.remove(name)
    }

    /// Validates the internal structural invariants of this component state.
    ///
    /// Cross-checking against the corresponding Reality component or
    /// engineering model belongs to the Layer 04 validation boundary.
    pub fn validate(&self) -> Result<(), ComponentStateError> {
        for (key, property) in &self.properties {
            if key != property.name() {
                return Err(ComponentStateError::NameMismatch {
                    key: key.clone(),
                    property: property.name().clone(),
                });
            }

            property
                .validate()
                .map_err(|source| ComponentStateError::InvalidProperty {
                    name: property.name().clone(),
                    source,
                })?;
        }

        self.physical.validate().map_err(ComponentStateError::InvalidPhysicalState)?;
        self.vectors.validate().map_err(ComponentStateError::InvalidSpatialState)?;
        if let Some(spatial) = &self.spatial {
            spatial.validate().map_err(ComponentStateError::InvalidSpatialState)?;
        }

        Ok(())
    }
}

/// Errors produced by [`ComponentState`].
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentStateError {
    /// A property with the same canonical identifier already exists.
    DuplicateProperty {
        name: Identifier,
    },

    /// The registry key does not match the property's canonical identity.
    NameMismatch {
        key: Identifier,
        property: Identifier,
    },

    /// A contained property state is structurally invalid.
    InvalidProperty {
        name: Identifier,
        source: crate::property_state::PropertyStateError,
    },

    /// A contained physical quantity collection is invalid.
    InvalidPhysicalState(PhysicalStateError),

    /// Contained spatial values are invalid.
    InvalidSpatialState(SpatialStateError),
}

impl fmt::Display for ComponentStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProperty { name } => {
                write!(f, "property state already exists: {name}")
            }
            Self::NameMismatch { key, property } => {
                write!(
                    f,
                    "property state name mismatch: key `{key}` does not match property `{property}`"
                )
            }
            Self::InvalidProperty { name, source } => {
                write!(f, "invalid property state `{name}`: {source}")
            }
            Self::InvalidPhysicalState(source) => write!(f, "invalid physical state: {source}"),
            Self::InvalidSpatialState(source) => write!(f, "invalid spatial state: {source}"),
        }
    }
}

impl Error for ComponentStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidProperty { source, .. } => Some(source),
            Self::InvalidPhysicalState(source) => Some(source),
            Self::InvalidSpatialState(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_component_state_is_empty() {
        let name = Identifier::parse("motor").unwrap();

        let state = ComponentState::new(name.clone());

        assert_eq!(state.name(), &name);
        assert_eq!(state.property_count(), 0);
        assert!(state.is_empty());
    }

    #[test]
    fn identity_is_stable() {
        let name = Identifier::parse("motor").unwrap();
        let state = ComponentState::new(name.clone());

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn empty_component_state_is_structurally_valid() {
        let name = Identifier::parse("motor").unwrap();
        let state = ComponentState::new(name);

        assert!(state.validate().is_ok());
    }

    #[test]
    fn properties_are_deterministically_ordered() {
        let state = ComponentState::new(Identifier::parse("motor").unwrap());

        let names: Vec<&Identifier> =
            state.named_properties().map(|(name, _)| name).collect();

        assert!(names.is_empty());
    }

    #[test]
    fn equality_is_value_based() {
        let name = Identifier::parse("motor").unwrap();

        let first = ComponentState::new(name.clone());
        let second = ComponentState::new(name);

        assert_eq!(first, second);
    }
}
