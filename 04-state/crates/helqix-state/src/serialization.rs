//! Versioned canonical binary encoding for complete runtime states.
//!
//! The format is little-endian and intentionally independent of Rust's memory
//! layout. Components, properties, physical quantities, and vector fields are
//! serialized in `BTreeMap` order. Decoding validates all lengths, tags,
//! identifiers, units, values, duplicates, and trailing bytes before returning
//! a state. Format version 3 includes unit-tagged vectors and transforms;
//! versions 1 and 2 remain readable for stored development data.

use std::{error::Error, fmt};

use helqix_core::{Identifier, Quantity, Unit};

use crate::{
    component_state::ComponentState,
    property_state::PropertyState,
    spatial_state::{SpatialState, UnitQuaternion, Vector3, VectorField},
    state::{State, StateError},
};

const MAGIC: &[u8; 4] = b"HLQS";
const VERSION: u16 = 3;
const VERSION_WITH_UNITS: u16 = 3;
const PREVIOUS_VERSION: u16 = 2;
const LEGACY_VERSION: u16 = 1;
const MAX_ITEMS: usize = 1_000_000;
const MAX_STRING_BYTES: usize = 1024;
const MAX_ENCODED_BYTES: usize = 64 * 1024 * 1024;

/// Canonical serializer and decoder for [`State`].
pub struct StateSerializer;

impl StateSerializer {
    /// Serializes a validated state to its canonical byte representation.
    pub fn to_bytes(state: &State) -> Result<Vec<u8>, StateSerializationError> {
        state.validate().map_err(StateSerializationError::InvalidState)?;
        let mut writer = Writer::new();
        writer.raw(MAGIC)?;
        writer.u16(VERSION)?;
        writer.u64(state.revision())?;
        writer.count(state.component_count())?;

        for component in state.components() {
            writer.string(component.name().as_str())?;
            writer.count(component.property_count())?;
            for property in component.properties() {
                writer.string(property.name().as_str())?;
                match property.value() {
                    None => writer.byte(0)?,
                    Some(quantity) => {
                        writer.byte(1)?;
                        writer.f64(quantity.value())?;
                        writer.string(quantity.unit().symbol())?;
                    }
                }
            }

            writer.count(component.physical().len())?;
            for (name, quantity) in component.physical().iter() {
                writer.string(name.as_str())?;
                writer.f64(quantity.value())?;
                writer.string(quantity.unit().symbol())?;
            }

            match component.spatial() {
                None => writer.byte(0)?,
                Some(spatial) => {
                    writer.byte(1)?;
                    let position = spatial.position();
                    writer.f64(position.x())?;
                    writer.f64(position.y())?;
                    writer.f64(position.z())?;
                    writer.string(position.unit().symbol())?;
                    let orientation = spatial.orientation();
                    writer.f64(orientation.x())?;
                    writer.f64(orientation.y())?;
                    writer.f64(orientation.z())?;
                    writer.f64(orientation.w())?;
                }
            }

            writer.count(component.vectors().len())?;
            for (name, vector) in component.vectors().iter() {
                writer.string(name.as_str())?;
                writer.f64(vector.x())?;
                writer.f64(vector.y())?;
                writer.f64(vector.z())?;
                writer.string(vector.unit().symbol())?;
            }
        }
        Ok(writer.finish())
    }

    /// Decodes a state, rejecting malformed or unsupported byte streams.
    pub fn from_bytes(bytes: &[u8]) -> Result<State, StateSerializationError> {
        if bytes.len() > MAX_ENCODED_BYTES {
            return Err(StateSerializationError::LimitExceeded);
        }
        let mut reader = Reader::new(bytes);
        if reader.raw(4)? != MAGIC.as_slice() {
            return Err(StateSerializationError::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != VERSION && version != PREVIOUS_VERSION && version != LEGACY_VERSION {
            return Err(StateSerializationError::UnsupportedVersion(version));
        }
        let revision = reader.u64()?;
        let component_count = reader.count()?;
        let mut state = State::new();

        for _ in 0..component_count {
            let component_name = parse_identifier(reader.string()?)?;
            let property_count = reader.count()?;
            let mut component = ComponentState::new(component_name);

            for _ in 0..property_count {
                let name = parse_identifier(reader.string()?)?;
                let value = match reader.byte()? {
                    0 => None,
                    1 => Some(read_quantity(&mut reader)?),
                    tag => return Err(StateSerializationError::InvalidValueTag(tag)),
                };
                let property = match value {
                    Some(quantity) => PropertyState::with_value(name, quantity),
                    None => PropertyState::new(name),
                };
                component.add_property(property).map_err(|error| {
                    StateSerializationError::InvalidStructure(error.to_string())
                })?;
            }

            if version >= PREVIOUS_VERSION {
                let physical_count = reader.count()?;
                for _ in 0..physical_count {
                    let name = parse_identifier(reader.string()?)?;
                    let quantity = read_quantity(&mut reader)?;
                    component.physical_mut().insert(name, quantity).map_err(|error| {
                        StateSerializationError::InvalidStructure(error.to_string())
                    })?;
                }

                match reader.byte()? {
                    0 => {}
                    1 => {
                        let x = reader.f64()?;
                        let y = reader.f64()?;
                        let z = reader.f64()?;
                        let position_unit = if version >= VERSION_WITH_UNITS {
                            Unit::parse(reader.string()?)
                                .map_err(|error| StateSerializationError::InvalidUnit(error.to_string()))?
                        } else {
                            Unit::parse("m").expect("metre is registered in Foundation")
                        };
                        let position = Vector3::new(x, y, z, position_unit)
                            .map_err(StateSerializationError::Spatial)?;
                        let orientation = UnitQuaternion::from_canonical_parts(
                            reader.f64()?, reader.f64()?, reader.f64()?, reader.f64()?,
                        ).map_err(StateSerializationError::Spatial)?;
                        let spatial = SpatialState::new(position, orientation)
                            .map_err(StateSerializationError::Spatial)?;
                        component.set_spatial(Some(spatial));
                    }
                    tag => return Err(StateSerializationError::InvalidValueTag(tag)),
                }

                let vector_count = reader.count()?;
                let mut vectors = VectorField::new();
                for _ in 0..vector_count {
                    let name = parse_identifier(reader.string()?)?;
                    let x = reader.f64()?;
                    let y = reader.f64()?;
                    let z = reader.f64()?;
                    let vector_unit = if version >= VERSION_WITH_UNITS {
                        Unit::parse(reader.string()?)
                            .map_err(|error| StateSerializationError::InvalidUnit(error.to_string()))?
                    } else {
                        Unit::parse("one").expect("dimensionless unit is registered in Foundation")
                    };
                    let value = Vector3::new(x, y, z, vector_unit)
                        .map_err(StateSerializationError::Spatial)?;
                    vectors.insert(name, value).map_err(StateSerializationError::Spatial)?;
                }
                *component.vectors_mut() = vectors;
            }

            state.add_component(component).map_err(|error| match error {
                StateError::DuplicateComponent { name } => {
                    StateSerializationError::InvalidStructure(format!("duplicate component: {name}"))
                }
                other => StateSerializationError::InvalidState(other),
            })?;
        }

        if !reader.is_finished() {
            return Err(StateSerializationError::TrailingBytes);
        }
        state.set_revision(revision);
        state.validate().map_err(StateSerializationError::InvalidState)?;
        Ok(state)
    }
}

fn read_quantity(reader: &mut Reader<'_>) -> Result<Quantity, StateSerializationError> {
    let value = reader.f64()?;
    let unit_symbol = reader.string()?;
    let unit = Unit::parse(unit_symbol)
        .map_err(|error| StateSerializationError::InvalidUnit(error.to_string()))?;
    Quantity::new(value, unit)
        .map_err(|error| StateSerializationError::InvalidQuantity(error.to_string()))
}

fn parse_identifier(value: &str) -> Result<Identifier, StateSerializationError> {
    Identifier::parse(value)
        .map_err(|error| StateSerializationError::InvalidIdentifier(error.to_string()))
}

struct Writer {
    bytes: Vec<u8>,
    items_written: usize,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new(), items_written: 0 }
    }
    fn ensure_capacity(&self, additional: usize) -> Result<(), StateSerializationError> {
        if self.bytes.len().checked_add(additional).map_or(true, |size| size > MAX_ENCODED_BYTES) {
            return Err(StateSerializationError::LimitExceeded);
        }
        Ok(())
    }
    fn raw(&mut self, value: &[u8]) -> Result<(), StateSerializationError> {
        self.ensure_capacity(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn byte(&mut self, value: u8) -> Result<(), StateSerializationError> { self.raw(&[value]) }
    fn u16(&mut self, value: u16) -> Result<(), StateSerializationError> { self.raw(&value.to_le_bytes()) }
    fn u32(&mut self, value: u32) -> Result<(), StateSerializationError> { self.raw(&value.to_le_bytes()) }
    fn u64(&mut self, value: u64) -> Result<(), StateSerializationError> { self.raw(&value.to_le_bytes()) }
    fn f64(&mut self, value: f64) -> Result<(), StateSerializationError> {
        let canonical = if value == 0.0 { 0.0 } else { value };
        self.raw(&canonical.to_le_bytes())
    }
    fn count(&mut self, count: usize) -> Result<(), StateSerializationError> {
        self.items_written = self.items_written
            .checked_add(count)
            .ok_or(StateSerializationError::LimitExceeded)?;
        if count > MAX_ITEMS || self.items_written > MAX_ITEMS {
            return Err(StateSerializationError::LimitExceeded);
        }
        self.u32(u32::try_from(count).map_err(|_| StateSerializationError::LimitExceeded)?)
    }
    fn string(&mut self, value: &str) -> Result<(), StateSerializationError> {
        if value.len() > MAX_STRING_BYTES { return Err(StateSerializationError::LimitExceeded); }
        self.u16(u16::try_from(value.len()).map_err(|_| StateSerializationError::LimitExceeded)?)?;
        self.raw(value.as_bytes())
    }
    fn finish(self) -> Vec<u8> { self.bytes }
}

struct Reader<'a> { bytes: &'a [u8], offset: usize, items_seen: usize }

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, offset: 0, items_seen: 0 } }
    fn raw(&mut self, size: usize) -> Result<&'a [u8], StateSerializationError> {
        let end = self.offset.checked_add(size).ok_or(StateSerializationError::Truncated)?;
        let value = self.bytes.get(self.offset..end).ok_or(StateSerializationError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, StateSerializationError> { Ok(self.raw(1)?[0]) }
    fn u16(&mut self) -> Result<u16, StateSerializationError> { Ok(u16::from_le_bytes(self.raw(2)?.try_into().expect("two bytes"))) }
    fn u32(&mut self) -> Result<u32, StateSerializationError> { Ok(u32::from_le_bytes(self.raw(4)?.try_into().expect("four bytes"))) }
    fn u64(&mut self) -> Result<u64, StateSerializationError> { Ok(u64::from_le_bytes(self.raw(8)?.try_into().expect("eight bytes"))) }
    fn f64(&mut self) -> Result<f64, StateSerializationError> { Ok(f64::from_le_bytes(self.raw(8)?.try_into().expect("eight bytes"))) }
    fn count(&mut self) -> Result<usize, StateSerializationError> {
        let count = self.u32()? as usize;
        self.items_seen = self.items_seen.checked_add(count).ok_or(StateSerializationError::LimitExceeded)?;
        if count > MAX_ITEMS || self.items_seen > MAX_ITEMS { return Err(StateSerializationError::LimitExceeded); }
        Ok(count)
    }
    fn string(&mut self) -> Result<&'a str, StateSerializationError> {
        let len = self.u16()? as usize;
        if len > MAX_STRING_BYTES { return Err(StateSerializationError::LimitExceeded); }
        std::str::from_utf8(self.raw(len)?).map_err(|_| StateSerializationError::InvalidUtf8)
    }
    fn is_finished(&self) -> bool { self.offset == self.bytes.len() }
}

/// Failures produced while encoding or decoding a state.
#[derive(Debug, Clone, PartialEq)]
pub enum StateSerializationError {
    InvalidState(StateError),
    InvalidMagic,
    UnsupportedVersion(u16),
    Truncated,
    TrailingBytes,
    InvalidUtf8,
    InvalidValueTag(u8),
    InvalidIdentifier(String),
    InvalidUnit(String),
    InvalidQuantity(String),
    InvalidStructure(String),
    Spatial(crate::spatial_state::SpatialStateError),
    LimitExceeded,
}

impl fmt::Display for StateSerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(error) => write!(f, "state is invalid: {error}"),
            Self::InvalidMagic => write!(f, "invalid state data signature"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported state format version {version}"),
            Self::Truncated => write!(f, "state data ended before the value was complete"),
            Self::TrailingBytes => write!(f, "state data contains trailing bytes"),
            Self::InvalidUtf8 => write!(f, "state data contains invalid UTF-8"),
            Self::InvalidValueTag(tag) => write!(f, "unknown state value tag {tag}"),
            Self::InvalidIdentifier(error) => write!(f, "invalid identifier: {error}"),
            Self::InvalidUnit(error) => write!(f, "invalid unit: {error}"),
            Self::InvalidQuantity(error) => write!(f, "invalid quantity: {error}"),
            Self::InvalidStructure(error) => write!(f, "invalid state structure: {error}"),
            Self::Spatial(error) => write!(f, "invalid spatial value: {error}"),
            Self::LimitExceeded => write!(f, "state data exceeds a format limit"),
        }
    }
}

impl Error for StateSerializationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidState(error) => Some(error),
            Self::Spatial(error) => Some(error),
            _ => None,
        }
    }
}

