//! Structural environmental definition of a Reality.
//!
//! `Environment` is a structural environmental definition: it stores named
//! environmental quantities (gravity, ambient temperature, pressure, density,
//! wind speed, reference altitude) and validates their physical dimensions.
//! It is not a runtime simulation. It contains no time-varying behavior,
//! weather, fluid dynamics, randomness, or runtime state.
//!
//! Dimension compatibility is decided by comparing the dimension of the
//! supplied quantity's unit against the dimension of a reference unit parsed
//! through `helqix_core::units::Unit::parse`. Unit symbols are never compared,
//! and supplied quantities are stored unchanged. If a reference unit cannot be
//! parsed by the Foundation, the typed setter fails closed with
//! `EnvironmentError::DimensionMismatch`.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::quantity::Quantity;
use helqix_core::units::Unit;

/// Reference unit symbols used only to obtain expected dimensions.
const REFERENCE_ACCELERATION: &str = "m/s2";
const REFERENCE_TEMPERATURE: &str = "K";
const REFERENCE_PRESSURE: &str = "Pa";
const REFERENCE_DENSITY: &str = "kg/m3";
const REFERENCE_SPEED: &str = "m/s";
const REFERENCE_LENGTH: &str = "m";

/// Errors produced by environment definition and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentError {
    /// A parameter with this name is already defined.
    DuplicateParameter {
        /// Parameter name.
        name: String,
    },
    /// The parameter name is not a valid identifier or does not match its key.
    InvalidParameterName {
        /// Parameter name.
        name: String,
    },
    /// The parameter value is NaN or infinite.
    NonFiniteValue {
        /// Parameter name.
        name: String,
    },
    /// The quantity dimension is incompatible with the parameter.
    DimensionMismatch {
        /// Parameter name.
        name: String,
    },
    /// The requested parameter is not defined.
    MissingParameter {
        /// Parameter name.
        name: String,
    },
}

impl fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvironmentError::DuplicateParameter { name } => {
                write!(f, "duplicate environment parameter: {name}")
            }
            EnvironmentError::InvalidParameterName { name } => {
                write!(f, "invalid environment parameter name: {name}")
            }
            EnvironmentError::NonFiniteValue { name } => {
                write!(f, "non-finite value for environment parameter: {name}")
            }
            EnvironmentError::DimensionMismatch { name } => {
                write!(f, "dimension mismatch for environment parameter: {name}")
            }
            EnvironmentError::MissingParameter { name } => {
                write!(f, "missing environment parameter: {name}")
            }
        }
    }
}

impl Error for EnvironmentError {}

/// A named environmental quantity definition.
///
/// The name is immutable through this API. The value is a definition, not
/// runtime state.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentParameter {
    name: Identifier,
    value: Quantity,
}

impl EnvironmentParameter {
    /// Creates a parameter from a name and a quantity.
    pub fn new(name: Identifier, value: Quantity) -> Self {
        Self { name, value }
    }

    /// Returns the parameter name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the parameter quantity.
    pub fn value(&self) -> &Quantity {
        &self.value
    }

    /// Returns a mutable reference to the parameter quantity.
    pub fn value_mut(&mut self) -> &mut Quantity {
        &mut self.value
    }
}

/// Structural environmental definition owning named parameters.
///
/// This is not a runtime simulation. An empty environment is valid, and
/// `new()` inserts no default values.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    parameters: BTreeMap<Identifier, EnvironmentParameter>,
}

impl Environment {
    /// Creates an empty environment.
    pub fn new() -> Self {
        Self {
            parameters: BTreeMap::new(),
        }
    }

    /// Canonical name of the gravity parameter.
    pub fn gravity_name() -> Identifier {
        canonical("gravity")
    }

    /// Canonical name of the ambient temperature parameter.
    pub fn ambient_temperature_name() -> Identifier {
        canonical("ambient_temperature")
    }

    /// Canonical name of the ambient pressure parameter.
    pub fn ambient_pressure_name() -> Identifier {
        canonical("ambient_pressure")
    }

    /// Canonical name of the atmospheric density parameter.
    pub fn atmospheric_density_name() -> Identifier {
        canonical("atmospheric_density")
    }

    /// Canonical name of the wind speed parameter.
    pub fn wind_speed_name() -> Identifier {
        canonical("wind_speed")
    }

    /// Canonical name of the reference altitude parameter.
    pub fn reference_altitude_name() -> Identifier {
        canonical("reference_altitude")
    }

    /// Returns the number of parameters.
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Returns `true` if no parameters are defined.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Returns `true` if a parameter with this name is defined.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.parameters.contains_key(name)
    }

    /// Registers a parameter.
    ///
    /// Rejects duplicate names and non-finite values. Never overwrites.
    pub fn set(&mut self, parameter: EnvironmentParameter) -> Result<(), EnvironmentError> {
        let name = parameter.name().clone();

        if self.parameters.contains_key(&name) {
            return Err(EnvironmentError::DuplicateParameter {
                name: name.as_str().to_owned(),
            });
        }

        if !parameter.value().value().is_finite() {
            return Err(EnvironmentError::NonFiniteValue {
                name: name.as_str().to_owned(),
            });
        }

        self.parameters.insert(name, parameter);
        Ok(())
    }

    /// Registers a parameter from a name and a quantity, using the same rules
    /// as [`Environment::set`].
    pub fn set_value(&mut self, name: Identifier, value: Quantity) -> Result<(), EnvironmentError> {
        self.set(EnvironmentParameter::new(name, value))
    }

    /// Returns the parameter with this name, if defined.
    pub fn get(&self, name: &Identifier) -> Option<&EnvironmentParameter> {
        self.parameters.get(name)
    }

    /// Returns a mutable reference to the parameter with this name, if defined.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut EnvironmentParameter> {
        self.parameters.get_mut(name)
    }

    /// Returns the quantity of the parameter with this name, if defined.
    pub fn value(&self, name: &Identifier) -> Option<&Quantity> {
        self.get(name).map(EnvironmentParameter::value)
    }

    /// Removes and returns the parameter with this name, if defined.
    pub fn remove(&mut self, name: &Identifier) -> Option<EnvironmentParameter> {
        self.parameters.remove(name)
    }

    /// Iterates names and parameters in deterministic name order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &EnvironmentParameter)> {
        self.parameters.iter()
    }

    /// Iterates parameters in deterministic name order.
    pub fn values(&self) -> impl Iterator<Item = &EnvironmentParameter> {
        self.parameters.values()
    }

    /// Iterates parameter names in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.parameters.keys()
    }

    /// Sets gravity. The quantity must have acceleration dimension (L T^-2).
    pub fn set_gravity(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::gravity_name(), value, REFERENCE_ACCELERATION)
    }

    /// Sets ambient temperature. The quantity must have temperature dimension.
    pub fn set_ambient_temperature(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::ambient_temperature_name(), value, REFERENCE_TEMPERATURE)
    }

    /// Sets ambient pressure. The quantity must have pressure dimension.
    pub fn set_ambient_pressure(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::ambient_pressure_name(), value, REFERENCE_PRESSURE)
    }

    /// Sets atmospheric density. The quantity must have mass / volume dimension.
    pub fn set_atmospheric_density(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::atmospheric_density_name(), value, REFERENCE_DENSITY)
    }

    /// Sets wind speed. The quantity must have length / time dimension.
    pub fn set_wind_speed(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::wind_speed_name(), value, REFERENCE_SPEED)
    }

    /// Sets reference altitude. The quantity must have length dimension.
    pub fn set_reference_altitude(&mut self, value: Quantity) -> Result<(), EnvironmentError> {
        self.set_typed(Self::reference_altitude_name(), value, REFERENCE_LENGTH)
    }

    /// Returns the parameter with this name or `MissingParameter`.
    pub fn require(&self, name: &Identifier) -> Result<&EnvironmentParameter, EnvironmentError> {
        self.get(name)
            .ok_or_else(|| EnvironmentError::MissingParameter {
                name: name.as_str().to_owned(),
            })
    }

    /// Returns the quantity of the parameter with this name or `MissingParameter`.
    pub fn require_value(&self, name: &Identifier) -> Result<&Quantity, EnvironmentError> {
        self.require(name).map(EnvironmentParameter::value)
    }

    /// Validates structural consistency.
    ///
    /// Checks that each key equals its parameter name, that names are valid
    /// identifiers, and that every value is finite. An empty environment is
    /// valid; canonical parameters are not required.
    pub fn validate(&self) -> Result<(), EnvironmentError> {
        for (key, parameter) in &self.parameters {
            if key != parameter.name() || Identifier::parse(parameter.name().as_str()).is_err() {
                return Err(EnvironmentError::InvalidParameterName {
                    name: key.as_str().to_owned(),
                });
            }

            if !parameter.value().value().is_finite() {
                return Err(EnvironmentError::NonFiniteValue {
                    name: key.as_str().to_owned(),
                });
            }
        }

        Ok(())
    }

    /// Validates the dimension against a reference unit, then stores the
    /// original quantity unchanged.
    fn set_typed(
        &mut self,
        name: Identifier,
        value: Quantity,
        reference_symbol: &str,
    ) -> Result<(), EnvironmentError> {
        let compatible = match Unit::parse(reference_symbol) {
            Ok(reference) => value.unit().dimension() == reference.dimension(),
            Err(_) => false,
        };

        if !compatible {
            return Err(EnvironmentError::DimensionMismatch {
                name: name.as_str().to_owned(),
            });
        }

        self.set_value(name, value)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a canonical name. Canonical names are valid identifiers by
/// construction; an invalid one cannot be represented without panicking, so
/// this falls back to a parse of a fixed valid identifier.
fn canonical(name: &str) -> Identifier {
    match Identifier::parse(name) {
        Ok(identifier) => identifier,
        // Unreachable for the fixed canonical names above.
        Err(_) => loop_free_fallback(),
    }
}

fn loop_free_fallback() -> Identifier {
    match Identifier::parse("environment") {
        Ok(identifier) => identifier,
        Err(error) => {
            // "environment" is a valid identifier; reaching here indicates a
            // broken Foundation contract.
            let _ = error;
            std::process::abort()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("valid identifier")
    }

    fn quantity(value: f64, symbol: &str) -> Quantity {
        Quantity::new(value, Unit::parse(symbol).expect("valid unit")).expect("valid quantity")
    }

    #[test]
    fn new_environment_is_empty() {
        let environment = Environment::new();
        assert_eq!(environment.len(), 0);
        assert!(environment.is_empty());
        assert_eq!(environment.validate(), Ok(()));
    }

    #[test]
    fn environment_parameter_exposes_identity_and_value() {
        let parameter = EnvironmentParameter::new(id("altitude"), quantity(10.0, "m"));
        assert_eq!(parameter.name(), &id("altitude"));
        assert_eq!(parameter.value(), &quantity(10.0, "m"));
    }

    #[test]
    fn set_parameter() {
        let mut environment = Environment::new();
        let parameter = EnvironmentParameter::new(id("altitude"), quantity(10.0, "m"));
        environment.set(parameter.clone()).unwrap();

        assert_eq!(environment.len(), 1);
        assert!(environment.contains(&id("altitude")));
        assert_eq!(environment.get(&id("altitude")), Some(&parameter));
    }

    #[test]
    fn duplicate_parameter_is_rejected() {
        let mut environment = Environment::new();
        environment
            .set_value(id("altitude"), quantity(10.0, "m"))
            .unwrap();

        let result = environment.set_value(id("altitude"), quantity(20.0, "m"));
        assert_eq!(
            result,
            Err(EnvironmentError::DuplicateParameter {
                name: "altitude".to_owned(),
            })
        );
        assert_eq!(environment.len(), 1);
        assert_eq!(
            environment.value(&id("altitude")),
            Some(&quantity(10.0, "m"))
        );
    }

    #[test]
    fn set_value_registers_parameter() {
        let mut environment = Environment::new();
        environment
            .set_value(id("altitude"), quantity(5.0, "m"))
            .unwrap();

        let parameter = environment.get(&id("altitude")).unwrap();
        assert_eq!(parameter.name(), &id("altitude"));
        assert_eq!(parameter.value(), &quantity(5.0, "m"));
    }

    #[test]
    fn remove_parameter() {
        let mut environment = Environment::new();
        environment
            .set_value(id("altitude"), quantity(5.0, "m"))
            .unwrap();

        let removed = environment.remove(&id("altitude"));
        assert_eq!(
            removed,
            Some(EnvironmentParameter::new(id("altitude"), quantity(5.0, "m")))
        );
        assert!(environment.is_empty());
    }

    #[test]
    fn remove_missing_parameter_returns_none() {
        let mut environment = Environment::new();
        assert_eq!(environment.remove(&id("missing")), None);
    }

    #[test]
    fn deterministic_iteration() {
        let mut environment = Environment::new();
        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            environment.set_value(id(name), quantity(1.0, "m")).unwrap();
        }

        let keys: Vec<&str> = environment.keys().map(Identifier::as_str).collect();
        assert_eq!(keys, vec!["Alpha", "Battery", "Motor", "Zeta"]);

        let names: Vec<&str> = environment
            .values()
            .map(|parameter| parameter.name().as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "Battery", "Motor", "Zeta"]);

        let pairs: Vec<&str> = environment.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(pairs, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn gravity_accepts_acceleration_dimension() {
        let mut environment = Environment::new();
        environment.set_gravity(quantity(9.81, "m/s2")).unwrap();
        assert!(environment.contains(&Environment::gravity_name()));
    }

    #[test]
    fn gravity_rejects_wrong_dimension() {
        let mut environment = Environment::new();
        assert_eq!(
            environment.set_gravity(quantity(1.0, "m")),
            Err(EnvironmentError::DimensionMismatch {
                name: "gravity".to_owned(),
            })
        );
        assert_eq!(
            environment.set_gravity(quantity(1.0, "kg")),
            Err(EnvironmentError::DimensionMismatch {
                name: "gravity".to_owned(),
            })
        );
        assert!(environment.is_empty());
    }

    #[test]
    fn ambient_temperature_accepts_temperature_dimension() {
        let mut environment = Environment::new();
        environment
            .set_ambient_temperature(quantity(288.15, "K"))
            .unwrap();
        assert!(environment.contains(&Environment::ambient_temperature_name()));
    }

    #[test]
    fn ambient_pressure_accepts_pressure_dimension() {
        let mut environment = Environment::new();
        environment
            .set_ambient_pressure(quantity(101_325.0, "Pa"))
            .unwrap();
        assert!(environment.contains(&Environment::ambient_pressure_name()));
    }

    #[test]
    fn atmospheric_density_accepts_density_dimension() {
        let mut environment = Environment::new();
        environment
            .set_atmospheric_density(quantity(1.225, "kg/m3"))
            .unwrap();
        assert!(environment.contains(&Environment::atmospheric_density_name()));
    }

    #[test]
    fn wind_speed_accepts_speed_dimension() {
        let mut environment = Environment::new();
        environment.set_wind_speed(quantity(5.0, "m/s")).unwrap();
        assert!(environment.contains(&Environment::wind_speed_name()));
    }

    #[test]
    fn reference_altitude_accepts_length_dimension() {
        let mut environment = Environment::new();
        environment
            .set_reference_altitude(quantity(100.0, "m"))
            .unwrap();
        assert!(environment.contains(&Environment::reference_altitude_name()));
    }

    #[test]
    fn typed_setter_preserves_original_unit() {
        let mut environment = Environment::new();
        let supplied = quantity(1000.0, "m");
        environment.set_reference_altitude(supplied.clone()).unwrap();

        let stored = environment
            .value(&Environment::reference_altitude_name())
            .unwrap();
        assert_eq!(stored, &supplied);
        assert_eq!(stored.value(), 1000.0);
    }

    #[test]
    fn require_returns_existing_parameter() {
        let mut environment = Environment::new();
        environment
            .set_value(id("altitude"), quantity(5.0, "m"))
            .unwrap();

        let parameter = environment.require(&id("altitude")).unwrap();
        assert_eq!(parameter.name(), &id("altitude"));
    }

    #[test]
    fn require_missing_parameter_returns_error() {
        let environment = Environment::new();
        assert_eq!(
            environment.require(&id("altitude")),
            Err(EnvironmentError::MissingParameter {
                name: "altitude".to_owned(),
            })
        );
    }

    #[test]
    fn require_value_returns_quantity() {
        let mut environment = Environment::new();
        environment
            .set_value(id("altitude"), quantity(5.0, "m"))
            .unwrap();

        assert_eq!(
            environment.require_value(&id("altitude")),
            Ok(&quantity(5.0, "m"))
        );
        assert_eq!(
            environment.require_value(&id("missing")),
            Err(EnvironmentError::MissingParameter {
                name: "missing".to_owned(),
            })
        );
    }

    #[test]
    fn validate_accepts_partial_environment() {
        let mut environment = Environment::new();
        environment
            .set_ambient_pressure(quantity(101_325.0, "Pa"))
            .unwrap();
        assert_eq!(environment.validate(), Ok(()));
    }

    #[test]
    fn validate_accepts_empty_environment() {
        assert_eq!(Environment::default().validate(), Ok(()));
    }

    #[test]
    fn canonical_parameter_names_are_deterministic() {
        assert_eq!(Environment::gravity_name().as_str(), "gravity");
        assert_eq!(
            Environment::ambient_temperature_name().as_str(),
            "ambient_temperature"
        );
        assert_eq!(
            Environment::ambient_pressure_name().as_str(),
            "ambient_pressure"
        );
        assert_eq!(
            Environment::atmospheric_density_name().as_str(),
            "atmospheric_density"
        );
        assert_eq!(Environment::wind_speed_name().as_str(), "wind_speed");
        assert_eq!(
            Environment::reference_altitude_name().as_str(),
            "reference_altitude"
        );
    }
}