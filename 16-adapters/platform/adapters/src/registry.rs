use std::{
    collections::HashMap,
    sync::{atomic::{AtomicUsize, Ordering}, Arc, RwLock},
};

use crate::{
    adapter::{Adapter, AdapterContext, AdapterError, AdapterRequest, AdapterResponse, HealthReport},
    capability::AdapterDescriptor,
    lifecycle::AdapterState,
};

struct RegisteredAdapter {
    adapter: Arc<dyn Adapter>,
    descriptor: AdapterDescriptor,
    concurrency_limits: HashMap<String, Arc<ConcurrencyLimit>>,
}

/// Nonblocking per-capability admission control. A saturated adapter is
/// rejected immediately so callers can schedule a retry without parking an
/// executor thread or building an unbounded in-process queue.
struct ConcurrencyLimit {
    maximum: usize,
    in_flight: AtomicUsize,
}

impl ConcurrencyLimit {
    fn try_acquire(self: &Arc<Self>) -> Option<ConcurrencyPermit> {
        let mut current = self.in_flight.load(Ordering::Acquire);
        loop {
            if current >= self.maximum {
                return None;
            }
            match self.in_flight.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(ConcurrencyPermit(Arc::clone(self))),
                Err(observed) => current = observed,
            }
        }
    }
}

struct ConcurrencyPermit(Arc<ConcurrencyLimit>);

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        let previous = self.0.in_flight.fetch_sub(1, Ordering::Release);
        debug_assert!(previous > 0, "concurrency permit count must not underflow");
    }
}

/// Concurrent registry and capability-based router for adapter instances.
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: RwLock<HashMap<String, RegisteredAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&self, adapter: Arc<dyn Adapter>) -> Result<AdapterDescriptor, AdapterError> {
        let descriptor = adapter.descriptor().clone();
        descriptor.validate()?;
        let concurrency_limits = descriptor.capabilities.iter().filter_map(|capability| {
            capability.max_concurrency.map(|maximum| (
                capability.name.clone(),
                Arc::new(ConcurrencyLimit { maximum, in_flight: AtomicUsize::new(0) }),
            ))
        }).collect();
        let mut adapters = self.adapters.write().map_err(|_| AdapterError::RegistryPoisoned)?;
        if adapters.contains_key(&descriptor.id) {
            return Err(AdapterError::InvalidDescriptor(format!("adapter id {:?} is already registered", descriptor.id)));
        }
        adapters.insert(descriptor.id.clone(), RegisteredAdapter { adapter, descriptor: descriptor.clone(), concurrency_limits });
        Ok(descriptor)
    }

    pub fn unregister(&self, id: &str) -> Result<Option<Arc<dyn Adapter>>, AdapterError> {
        self.adapters.write().map_err(|_| AdapterError::RegistryPoisoned)
            .map(|mut adapters| adapters.remove(id).map(|entry| entry.adapter))
    }

    pub fn get(&self, id: &str) -> Result<Option<Arc<dyn Adapter>>, AdapterError> {
        self.adapters.read().map_err(|_| AdapterError::RegistryPoisoned)
            .map(|adapters| adapters.get(id).map(|entry| Arc::clone(&entry.adapter)))
    }

    pub fn descriptors(&self) -> Result<Vec<AdapterDescriptor>, AdapterError> {
        let adapters = self.adapters.read().map_err(|_| AdapterError::RegistryPoisoned)?;
        let mut descriptors: Vec<_> = adapters.values().map(|entry| entry.descriptor.clone()).collect();
        descriptors.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(descriptors)
    }

    pub fn find(&self, capability: &str, operation: &str) -> Result<Vec<Arc<dyn Adapter>>, AdapterError> {
        let adapters = self.adapters.read().map_err(|_| AdapterError::RegistryPoisoned)?;
        let mut matches: Vec<_> = adapters.values()
            .filter(|entry| entry.descriptor.supports(capability, operation))
            .map(|entry| Arc::clone(&entry.adapter))
            .collect();
        matches.sort_by(|left, right| left.descriptor().id.cmp(&right.descriptor().id));
        Ok(matches)
    }

    pub async fn execute(
        &self,
        adapter_id: &str,
        context: &AdapterContext,
        request: AdapterRequest,
    ) -> Result<AdapterResponse, AdapterError> {
        request.validate()?;
        ensure_context(context)?;
        let (adapter, descriptor, limits) = {
            let adapters = self.adapters.read().map_err(|_| AdapterError::RegistryPoisoned)?;
            let entry = adapters.get(adapter_id).ok_or_else(|| AdapterError::InvalidDescriptor(format!("adapter {adapter_id:?} is not registered")))?;
            (Arc::clone(&entry.adapter), entry.descriptor.clone(), entry.concurrency_limits.clone())
        };
        let actual_state = adapter.state();
        if actual_state != AdapterState::Ready {
            return Err(AdapterError::InvalidState { expected: "ready", actual: actual_state });
        }
        let matching_capabilities: Vec<_> = descriptor.capabilities.iter()
            .filter(|capability| capability.supports(&request.operation))
            .collect();
        if matching_capabilities.is_empty() {
            return Err(AdapterError::InvalidRequest(format!("adapter {adapter_id:?} does not support operation {:?}", request.operation)));
        }
        if request.idempotency_key.is_some() && !matching_capabilities.iter().any(|capability| capability.supports_idempotency) {
            return Err(AdapterError::InvalidRequest(format!("adapter {adapter_id:?} does not guarantee idempotency for operation {:?}", request.operation)));
        }
        let mut permits = Vec::with_capacity(matching_capabilities.len());
        for capability in matching_capabilities {
            if let Some(limit) = limits.get(&capability.name) {
                let permit = limit.try_acquire().ok_or_else(|| AdapterError::ConcurrencyLimitExceeded {
                    capability: capability.name.clone(),
                    limit: limit.maximum,
                })?;
                permits.push(permit);
            }
        }
        // Keep every declared capability permit alive until the adapter future
        // completes or is dropped, including during cancellation and panics.
        let result = adapter.execute(context, request).await;
        drop(permits);
        ensure_context(context)?;
        result
    }

    pub async fn health_check(&self, adapter_id: &str, context: &AdapterContext) -> Result<HealthReport, AdapterError> {
        ensure_context(context)?;
        let adapter = self.get(adapter_id)?.ok_or_else(|| AdapterError::InvalidDescriptor(format!("adapter {adapter_id:?} is not registered")))?;
        adapter.health_check(context).await
    }

    pub fn len(&self) -> Result<usize, AdapterError> {
        self.adapters.read().map(|adapters| adapters.len()).map_err(|_| AdapterError::RegistryPoisoned)
    }

    pub fn is_empty(&self) -> Result<bool, AdapterError> { self.len().map(|count| count == 0) }
}

#[cfg(test)]
mod tests {
    use super::ConcurrencyLimit;
    use std::sync::{atomic::Ordering, Arc};

    #[test]
    fn capability_limit_rejects_saturation_and_releases_capacity_on_drop() {
        let limit = Arc::new(ConcurrencyLimit { maximum: 1, in_flight: Default::default() });
        let permit = limit.try_acquire().expect("first execution should be admitted");
        assert!(limit.try_acquire().is_none(), "second execution must be rejected at the declared limit");
        drop(permit);
        let replacement = limit.try_acquire().expect("completed execution must release its permit");
        assert_eq!(limit.in_flight.load(Ordering::Acquire), 1);
        drop(replacement);
        assert_eq!(limit.in_flight.load(Ordering::Acquire), 0);
    }
}

fn ensure_context(context: &AdapterContext) -> Result<(), AdapterError> {
    context.validate()?;
    context.check_active()
}
