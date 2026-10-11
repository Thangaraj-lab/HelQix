//! HELQIX Layer 10 event contracts: time-ordered events, explicit routing, and triggers.
#![forbid(unsafe_code)]
/// Event payload contract.
pub mod event;
/// Ordered event queue.
pub mod event_bus;
/// Event routing.
pub mod propagation;
/// Event-trigger rules.
pub mod trigger;
pub use event::{Event, EventError, EventValue};
pub use event_bus::{EventBus, EventBusError, DEFAULT_MAX_PENDING_EVENTS};
pub use propagation::{EventDelivery, EventRouter, RouterError, Subscription};
pub use trigger::{PayloadCondition, Trigger, TriggerEngine, TriggerError, TriggeredAction};

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::id::Identifier;
    use helqix_time::SimTime;
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    fn time(s: i128) -> SimTime { SimTime::from_secs(s).unwrap() }
    fn event(id_value: &str, second: i128) -> Event { Event::new(id(id_value), id("temperature_limit"), id("cell"), time(second)) }
    #[test]
    fn bus_orders_equal_time_events_by_acceptance_and_rejects_late_input() {
        let mut bus = EventBus::default(); bus.publish(event("event_a", 1)).unwrap(); bus.publish(event("event_b", 1)).unwrap();
        let delivered = bus.drain_until(time(1)).unwrap(); assert_eq!(delivered[0].id, id("event_a")); assert_eq!(delivered[1].id, id("event_b"));
        assert!(matches!(bus.publish(event("event_c", 0)), Err(EventBusError::LateEvent { .. })));
    }
    #[test]
    fn trigger_creates_proposal_only_when_explicit_condition_matches() {
        let event = event("event_a", 3).with_field(id("limit"), EventValue::Bool(true)).unwrap();
        let trigger = Trigger { id: id("stop_action"), event_type: id("temperature_limit"), action_type: id("shutdown"), target: id("battery"), condition: Some(PayloadCondition { field: id("limit"), equals: EventValue::Bool(true) }), enabled: true };
        let mut engine = TriggerEngine::default(); engine.add(trigger).unwrap(); assert_eq!(engine.evaluate(&event).len(), 1);
        engine.set_enabled(&id("stop_action"), false).unwrap(); assert!(engine.evaluate(&event).is_empty());
    }
}
