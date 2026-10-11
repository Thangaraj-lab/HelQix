//! Deterministic snapshots of observable runtime state.
use std::{collections::BTreeMap, fmt};
use helqix_core::Identifier;
use helqix_state::State;
use helqix_time::SimTime;
use crate::{metric::{Metric, MetricError, MetricKind, MetricUnit, MetricValue}, probe::{Probe, ProbeError, ProbeId}};

/// Origin of a metric within the sampled state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationSource {
    /// Runtime property belonging to a component.
    Property { /// Component identity.
        component: Identifier, /// Property identity.
        property: Identifier },
    /// Named physical quantity belonging to a component.
    PhysicalQuantity { /// Component identity.
        component: Identifier, /// Quantity identity.
        quantity: Identifier },
    /// Metric emitted by an explicitly registered probe.
    Probe(ProbeId),
}

/// Immutable, deterministically ordered collection sampled at one logical time.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationFrame { at: SimTime, state_revision: u64, metrics: Vec<Metric>, sources: Vec<ObservationSource> }
impl ObservationFrame {
    /// Logical time at which the frame was captured.
    pub fn observed_at(&self) -> SimTime { self.at }
    /// State revision captured by this frame.
    pub fn state_revision(&self) -> u64 { self.state_revision }
    /// Number of metrics in the frame.
    pub fn len(&self) -> usize { self.metrics.len() }
    /// Whether this frame contains no samples.
    pub fn is_empty(&self) -> bool { self.metrics.is_empty() }
    /// Metrics in deterministic emission order.
    pub fn metrics(&self) -> &[Metric] { &self.metrics }
    /// Returns the first metric with `id`.
    pub fn metric(&self, id: &Identifier) -> Option<&Metric> { self.metrics.iter().find(|metric| metric.id() == id) }
    /// Provenance corresponding one-to-one with [`Self::metrics`].
    pub fn sources(&self) -> &[ObservationSource] { &self.sources }
}

/// Read-only observer with a deterministic, duplicate-safe probe registry.
#[derive(Debug, Clone, Default)]
pub struct Observer { probes: BTreeMap<ProbeId, Probe> }
impl Observer {
    /// Creates an observer with no custom probes.
    pub fn new() -> Self { Self::default() }
    /// Adds a probe. Duplicate IDs are rejected and do not replace the old probe.
    pub fn register_probe(&mut self, probe: Probe) -> Result<(), ObserverError> {
        let id = probe.id().clone();
        if self.probes.contains_key(&id) { return Err(ObserverError::DuplicateProbe(id)); }
        self.probes.insert(id, probe); Ok(())
    }
    /// Removes and returns a registered probe, if present.
    pub fn remove_probe(&mut self, id: &ProbeId) -> Option<Probe> { self.probes.remove(id) }
    /// Registered probe count.
    pub fn probe_count(&self) -> usize { self.probes.len() }

    /// Captures all valued properties, physical quantities, and registered probes.
    ///
    /// Components and values are traversed in Foundation's canonical identifier
    /// order. The method only reads `state`; it does not mutate the state or clock.
    pub fn observe(&self, state: &State, at: SimTime) -> Result<ObservationFrame, ObserverError> {
        let mut metrics = Vec::new();
        let mut sources = Vec::new();
        let mut sequence = 0usize;
        for (component_id, component) in state.named_components() {
            for (property_id, property) in component.named_properties() {
                if let Some(quantity) = property.value() {
                    let metric_id = generated_id(sequence)?; sequence += 1;
                    let value = MetricValue::number(quantity.value()).map_err(ObserverError::Metric)?;
                    let unit = MetricUnit::symbol(quantity.unit().symbol()).map_err(ObserverError::Metric)?;
                    metrics.push(Metric::new(metric_id, MetricKind::Gauge, value, unit, at, state.revision()).map_err(ObserverError::Metric)?);
                    sources.push(ObservationSource::Property { component: component_id.clone(), property: property_id.clone() });
                }
            }
            for (quantity_id, quantity) in component.physical().named_quantities() {
                let metric_id = generated_id(sequence)?; sequence += 1;
                let value = MetricValue::number(quantity.value()).map_err(ObserverError::Metric)?;
                let unit = MetricUnit::symbol(quantity.unit().symbol()).map_err(ObserverError::Metric)?;
                metrics.push(Metric::new(metric_id, MetricKind::Gauge, value, unit, at, state.revision()).map_err(ObserverError::Metric)?);
                sources.push(ObservationSource::PhysicalQuantity { component: component_id.clone(), quantity: quantity_id.clone() });
            }
        }
        for probe in self.probes.values() {
            metrics.push(probe.sample(state, at).map_err(ObserverError::Probe)?);
            sources.push(ObservationSource::Probe(probe.id().clone()));
        }
        Ok(ObservationFrame { at, state_revision: state.revision(), metrics, sources })
    }
}

fn generated_id(sequence: usize) -> Result<Identifier, ObserverError> {
    Identifier::parse(&format!("state_metric_{sequence}")).map_err(|_| ObserverError::MetricIdExhausted)
}

/// Failures raised while configuring or capturing observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserverError {
    /// Probe ID already exists.
    DuplicateProbe(ProbeId),
    /// Metric validation failed.
    Metric(MetricError),
    /// A registered probe could not resolve against the requested state.
    Probe(ProbeError),
    /// Generated metric identity exceeded the Foundation identifier constraints.
    MetricIdExhausted,
}
impl fmt::Display for ObserverError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::DuplicateProbe(id) => write!(f, "probe `{id}` is already registered"), Self::Metric(e) => write!(f, "observation metric is invalid: {e}"),
    Self::Probe(e) => write!(f, "observation probe failed: {e}"), Self::MetricIdExhausted => f.write_str("could not generate a valid metric identifier") } } }
impl std::error::Error for ObserverError {}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn empty_state_produces_empty_frame_without_mutation() { let state = State::new(); let frame = Observer::new().observe(&state, SimTime::ZERO).unwrap(); assert!(frame.is_empty()); assert_eq!(frame.state_revision(), 0); assert_eq!(state.revision(), 0); }
    #[test] fn probe_registry_rejects_duplicate_ids() { let mut observer = Observer::new(); let id = Identifier::parse("component_count").unwrap(); let p = Probe::new(id.clone(), crate::ProbeTarget::ComponentCount); observer.register_probe(p.clone()).unwrap(); assert_eq!(observer.register_probe(p), Err(ObserverError::DuplicateProbe(id))); }
    #[test] fn frames_keep_timestamp_and_state_revision() { let s = State::new(); let t = SimTime::from_nanos(42); let f = Observer::new().observe(&s, t).unwrap(); assert_eq!(f.observed_at(), t); assert_eq!(f.state_revision(), s.revision()); }
}
