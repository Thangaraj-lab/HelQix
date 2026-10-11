//! Deterministic spatial values stored by the state layer.
//!
//! These types represent dimensioned vectors, transforms, and orientation.
//! They do not integrate motion, apply forces, or advance simulation time.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::{Dimension, Identifier, Unit};

/// A finite three dimensional vector with an explicit unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
    unit: Unit,
}

impl Vector3 {
    /// Creates coordinates after rejecting NaN and infinite values.
    pub fn new(x: f64, y: f64, z: f64, unit: Unit) -> Result<Self, SpatialStateError> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(SpatialStateError::NonFiniteVector);
        }
        Ok(Self {
            x: canonical_zero(x),
            y: canonical_zero(y),
            z: canonical_zero(z),
            unit,
        })
    }

    #[must_use]
    pub const fn x(self) -> f64 { self.x }

    #[must_use]
    pub const fn y(self) -> f64 { self.y }

    #[must_use]
    pub const fn z(self) -> f64 { self.z }

    #[must_use]
    pub const fn unit(self) -> Unit { self.unit }

    /// Checks the finite-coordinate invariant.
    pub fn validate(self) -> Result<(), SpatialStateError> {
        if self.x.is_finite() && self.y.is_finite() && self.z.is_finite() {
            Ok(())
        } else {
            Err(SpatialStateError::NonFiniteVector)
        }
    }
}

/// A normalized unit quaternion stored in canonical sign form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitQuaternion {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl UnitQuaternion {
    /// Returns the identity rotation.
    #[must_use]
    pub const fn identity() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }

    /// Normalizes input components and canonicalizes the sign so equivalent
    /// `q` and `-q` orientations have one stable representation.
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Result<Self, SpatialStateError> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() || !w.is_finite() {
            return Err(SpatialStateError::NonFiniteQuaternion);
        }

        // Scale before computing the norm so finite values near `f64::MAX`
        // do not overflow during normalization and subnormal inputs remain
        // normalizable.
        let scale = x.abs().max(y.abs()).max(z.abs()).max(w.abs());
        if scale == 0.0 {
            return Err(SpatialStateError::ZeroQuaternion);
        }
        let (x, y, z, w) = (x / scale, y / scale, z / scale, w / scale);
        let norm = x.hypot(y).hypot(z).hypot(w);
        if !norm.is_finite() || norm == 0.0 {
            return Err(SpatialStateError::NonFiniteQuaternion);
        }

        let (mut x, mut y, mut z, mut w) = (x / norm, y / norm, z / norm, w / norm);
        let sign_is_negative = if w != 0.0 {
            w < 0.0
        } else if x != 0.0 {
            x < 0.0
        } else if y != 0.0 {
            y < 0.0
        } else {
            z < 0.0
        };
        if sign_is_negative {
            x = -x;
            y = -y;
            z = -z;
            w = -w;
        }

        Ok(Self {
            x: canonical_zero(x),
            y: canonical_zero(y),
            z: canonical_zero(z),
            w: canonical_zero(w),
        })
    }

    #[must_use]
    pub const fn x(self) -> f64 { self.x }

    #[must_use]
    pub const fn y(self) -> f64 { self.y }

    #[must_use]
    pub const fn z(self) -> f64 { self.z }

    #[must_use]
    pub const fn w(self) -> f64 { self.w }

    /// Checks normalization and finite-component invariants.
    pub fn validate(self) -> Result<(), SpatialStateError> {
        let norm = self.x.hypot(self.y).hypot(self.z).hypot(self.w);
        if !norm.is_finite() {
            return Err(SpatialStateError::NonFiniteQuaternion);
        }
        if (norm - 1.0).abs() > 1e-12 {
            return Err(SpatialStateError::NonUnitQuaternion);
        }
        Ok(())
    }

    /// Accepts normalized canonical components without renormalizing, which
    /// preserves their exact floating-point values during deserialization.
    pub(crate) fn from_canonical_parts(
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    ) -> Result<Self, SpatialStateError> {
        let value = Self { x, y, z, w };
        value.validate()?;

        let canonical_sign = if w != 0.0 {
            w > 0.0
        } else if x != 0.0 {
            x > 0.0
        } else if y != 0.0 {
            y > 0.0
        } else {
            z >= 0.0
        };
        let has_negative_zero = [x, y, z, w]
            .iter()
            .any(|component| *component == 0.0 && component.is_sign_negative());
        if !canonical_sign || has_negative_zero {
            return Err(SpatialStateError::NonCanonicalQuaternion);
        }
        Ok(value)
    }
}

impl Default for UnitQuaternion {
    fn default() -> Self { Self::identity() }
}

/// Position and orientation captured for one spatial entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialState {
    position: Vector3,
    orientation: UnitQuaternion,
}

impl SpatialState {
    /// Creates a spatial transform whose position uses a length unit.
    pub fn new(
        position: Vector3,
        orientation: UnitQuaternion,
    ) -> Result<Self, SpatialStateError> {
        let value = Self { position, orientation };
        value.validate()?;
        Ok(value)
    }

    /// Returns the position vector, including its unit.
    #[must_use]
    pub const fn position(self) -> Vector3 { self.position }

    /// Returns the normalized orientation.
    #[must_use]
    pub const fn orientation(self) -> UnitQuaternion { self.orientation }

    /// Checks finite coordinates, length dimensions, and quaternion validity.
    pub fn validate(&self) -> Result<(), SpatialStateError> {
        self.position.validate()?;
        if self.position.unit().dimension() != Dimension::LENGTH {
            return Err(SpatialStateError::InvalidPositionUnit {
                symbol: self.position.unit().symbol().to_owned(),
            });
        }
        self.orientation.validate()
    }
}

impl Default for SpatialState {
    fn default() -> Self {
        let metre = Unit::parse("m").expect("metre is registered in Foundation");
        let position = Vector3::new(0.0, 0.0, 0.0, metre)
            .expect("zero coordinates are finite");
        Self::new(position, UnitQuaternion::identity())
            .expect("identity transform is valid")
    }
}

/// An identifier-indexed vector collection with deterministic iteration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VectorField {
    vectors: BTreeMap<Identifier, Vector3>,
}

impl VectorField {
    /// Creates an empty vector field.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Returns the number of named vectors.
    #[must_use]
    pub fn len(&self) -> usize { self.vectors.len() }

    /// Returns whether the field contains no vectors.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.vectors.is_empty() }

    /// Tests whether a vector is stored under `name`.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool { self.vectors.contains_key(name) }

    /// Returns a vector by identifier.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&Vector3> { self.vectors.get(name) }

    /// Iterates over vectors in deterministic identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Vector3)> {
        self.vectors.iter()
    }

    /// Inserts a vector and rejects duplicate identifiers.
    pub fn insert(
        &mut self,
        name: Identifier,
        value: Vector3,
    ) -> Result<(), SpatialStateError> {
        if self.vectors.contains_key(&name) {
            return Err(SpatialStateError::DuplicateVector { name });
        }
        value.validate()?;
        self.vectors.insert(name, value);
        Ok(())
    }

    /// Replaces a vector and returns the previous value, if present.
    pub fn set(&mut self, name: Identifier, value: Vector3) -> Option<Vector3> {
        self.vectors.insert(name, value)
    }

    /// Removes and returns a vector by identifier.
    pub fn remove(&mut self, name: &Identifier) -> Option<Vector3> {
        self.vectors.remove(name)
    }

    /// Validates all stored vectors.
    pub fn validate(&self) -> Result<(), SpatialStateError> {
        for value in self.vectors.values() {
            value.validate()?;
        }
        Ok(())
    }
}

/// Descriptive alias for a vector field used in spatial state.
pub type SpatialVectorField = VectorField;

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

/// Errors returned while constructing or validating spatial values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialStateError {
    NonFiniteVector,
    NonFiniteQuaternion,
    ZeroQuaternion,
    NonUnitQuaternion,
    NonCanonicalQuaternion,
    InvalidPositionUnit { symbol: String },
    DuplicateVector { name: Identifier },
}

impl fmt::Display for SpatialStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteVector => write!(f, "vector coordinates must be finite"),
            Self::NonFiniteQuaternion => write!(f, "quaternion coordinates must be finite"),
            Self::ZeroQuaternion => write!(f, "a zero quaternion cannot represent an orientation"),
            Self::NonUnitQuaternion => write!(f, "quaternion must have unit length"),
            Self::NonCanonicalQuaternion => write!(f, "quaternion must use canonical sign form"),
            Self::InvalidPositionUnit { symbol } => {
                write!(f, "position unit `{symbol}` must measure length")
            }
            Self::DuplicateVector { name } => write!(f, "vector already exists: {name}"),
        }
    }
}

impl Error for SpatialStateError {}
