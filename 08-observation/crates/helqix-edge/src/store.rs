//! Keeps Observed values apart from Authoritative ones. Authoritative values change only through
//! [`ValueStore::set_authoritative`], which refuses a value whose origin is Observed: a
//! measurement can inform a human decision but never silently becomes the design.
use std::collections::BTreeMap;

use crate::ingest::Observed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin { Authoritative, Simulated, Observed }

#[derive(Debug, Clone, PartialEq)]
pub enum StoreError { ObservedCannotOverwriteAuthoritative { key: String }, SimulatedCannotOverwriteAuthoritative { key: String } }

#[derive(Debug, Clone, Default)]
pub struct ValueStore { authoritative: BTreeMap<String, f64>, observed: Vec<Observed> }

impl ValueStore {
    pub fn authoritative(&self, key: &str) -> Option<f64> { self.authoritative.get(key).copied() }
    pub fn observed(&self) -> &[Observed] { &self.observed }
    /// Adds observations to the Observed channel. Never touches Authoritative values.
    pub fn record_observed(&mut self, values: impl IntoIterator<Item = Observed>) { self.observed.extend(values); }
    /// The one write path into Authoritative values. Only an Authoritative-origin value (a human or
    /// design decision) is accepted.
    pub fn set_authoritative(&mut self, key: &str, value: f64, origin: Origin) -> Result<(), StoreError> {
        match origin {
            Origin::Authoritative => { self.authoritative.insert(key.into(), value); Ok(()) }
            Origin::Observed => Err(StoreError::ObservedCannotOverwriteAuthoritative { key: key.into() }),
            Origin::Simulated => Err(StoreError::SimulatedCannotOverwriteAuthoritative { key: key.into() }),
        }
    }
}
