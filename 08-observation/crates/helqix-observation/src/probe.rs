//! Explicit paths for reading selected runtime values without mutating them.
use std::fmt;
use helqix_core::Identifier;
use helqix_state::State;
use helqix_time::SimTime;
use crate::metric::{Metric, MetricError, MetricKind, MetricUnit, MetricValue};

/// Stable identifier assigned to a probe.
pub type ProbeId = Identifier;

/// Value selected by a probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeTarget {
    /// Reads a named property value from a component.
    Property { /// Component identity.
        component: Identifier, /// Property identity.
        property: Identifier },
    /// Reads a named physical quantity from a component.
    PhysicalQuantity { /// Component identity.
        component: Identifier, /// Quantity identity.
        quantity: Identifier },
    /// Reads the number of components in the runtime state.
    ComponentCount,
    /// Reads the current state revision.
    StateRevision,
}

/// A validated named observation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe { id: ProbeId, target: ProbeTarget }
impl Probe {
    /// Creates a probe with a stable identifier and target.
    pub fn new(id: ProbeId, target: ProbeTarget) -> Self { Self { id, target } }
    /// Probe identifier.
    pub fn id(&self) -> &ProbeId { &self.id }
    /// Probe target.
    pub fn target(&self) -> &ProbeTarget { &self.target }

    /// Reads this probe from `state` and returns a timestamped metric.
    pub fn sample(&self, state: &State, at: SimTime) -> Result<Metric, ProbeError> {
        let (value, unit, kind) = match &self.target {
            ProbeTarget::Property { component, property } => {
                let component_state = state.component(component).ok_or_else(|| ProbeError::MissingComponent(component.clone()))?;
                let prop = component_state.property(property).ok_or_else(|| ProbeError::MissingProperty { component: component.clone(), property: property.clone() })?;
                let quantity = prop.value().ok_or_else(|| ProbeError::PropertyHasNoValue { component: component.clone(), property: property.clone() })?;
                (MetricValue::number(quantity.value()).map_err(ProbeError::Metric)?, MetricUnit::symbol(quantity.unit().symbol()).map_err(ProbeError::Metric)?, MetricKind::Gauge)
            }
            ProbeTarget::PhysicalQuantity { component, quantity } => {
                let component_state = state.component(component).ok_or_else(|| ProbeError::MissingComponent(component.clone()))?;
                let q = component_state.physical().get(quantity).ok_or_else(|| ProbeError::MissingPhysicalQuantity { component: component.clone(), quantity: quantity.clone() })?;
                (MetricValue::number(q.value()).map_err(ProbeError::Metric)?, MetricUnit::symbol(q.unit().symbol()).map_err(ProbeError::Metric)?, MetricKind::Gauge)
            }
            ProbeTarget::ComponentCount => (MetricValue::Integer(state.component_count() as i128), MetricUnit::Dimensionless, MetricKind::Gauge),
            ProbeTarget::StateRevision => (MetricValue::Integer(state.revision() as i128), MetricUnit::Dimensionless, MetricKind::Counter),
        };
        Metric::new(self.id.clone(), kind, value, unit, at, state.revision()).map_err(ProbeError::Metric)
    }
}

/// Errors while resolving or sampling a probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    /// The component is absent from the sampled state.
    MissingComponent(Identifier),
    /// The requested property is absent.
    MissingProperty { /// Component that was searched.
        component: Identifier, /// Requested property.
        property: Identifier },
    /// The property exists but has no assigned runtime value.
    PropertyHasNoValue { /// Component that owns the property.
        component: Identifier, /// Property without a value.
        property: Identifier },
    /// The requested physical quantity is absent.
    MissingPhysicalQuantity { /// Component that was searched.
        component: Identifier, /// Requested physical quantity.
        quantity: Identifier },
    /// The sample could not be represented as a valid metric.
    Metric(MetricError),
}
impl fmt::Display for ProbeError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::MissingComponent(c) => write!(f, "probe component `{c}` does not exist"),
    Self::MissingProperty { component, property } => write!(f, "probe property `{component}.{property}` does not exist"),
    Self::PropertyHasNoValue { component, property } => write!(f, "probe property `{component}.{property}` has no value"),
    Self::MissingPhysicalQuantity { component, quantity } => write!(f, "probe quantity `{component}.{quantity}` does not exist"),
    Self::Metric(e) => write!(f, "invalid probe sample: {e}") } } }
impl std::error::Error for ProbeError {}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn missing_component_is_reported() { let probe = Probe::new(Identifier::parse("speed_probe").unwrap(), ProbeTarget::Property { component: Identifier::parse("motor").unwrap(), property: Identifier::parse("speed").unwrap() }); assert!(matches!(probe.sample(&State::new(), SimTime::ZERO), Err(ProbeError::MissingComponent(_)))); }
    #[test] fn revision_probe_is_exact_and_deterministic() { let probe = Probe::new(Identifier::parse("revision").unwrap(), ProbeTarget::StateRevision); let s = State::new(); let m = probe.sample(&s, SimTime::ZERO).unwrap(); assert_eq!(m.value(), MetricValue::Integer(0)); }
}
