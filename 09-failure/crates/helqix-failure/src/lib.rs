//! HELQIX Layer 09 — explicit fault modeling, edge-triggered detection, and bounded propagation.
//!
//! This crate records declared failure modes and detector outcomes. It does not
//! infer hazards, reliability, safety, or physical causation from a graph.
#![forbid(unsafe_code)]

/// Metric threshold evaluation.
pub mod detector;
/// Fault definitions and severity.
pub mod fault;
/// Failure lifecycle and observations.
pub mod failure;
/// Declared failure consequences.
pub mod propagation;

pub use detector::{DetectorError, FailureDetector, ThresholdOperator, ThresholdRule};
pub use failure::{FailureError, FailureObservation, FailureRecord, FailureStatus, StatusTransition};
pub use fault::{Fault, FaultError, FaultSeverity};
pub use propagation::{FailurePropagation, FailurePropagator, PropagationError, PropagationRule, PropagationStep, MAX_PROPAGATION_HOPS};

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::{id::Identifier, quantity::Quantity, units::Unit};
    use helqix_time::SimTime;
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    fn time(s: i128) -> SimTime { SimTime::from_secs(s).unwrap() }

    #[test]
    fn threshold_detection_is_edge_triggered_and_unit_aware() {
        let amp = Unit::parse("A").unwrap();
        let threshold = Quantity::new(10.0, amp).unwrap();
        let rule = ThresholdRule::new(id("overcurrent"), id("cell"), id("current"), id("thermal_runaway"), ThresholdOperator::GreaterThan, threshold, FaultSeverity::Critical, "Current exceeds configured limit").unwrap();
        let mut detector = FailureDetector::new(id("battery_monitor")); detector.add_rule(rule).unwrap();
        let above = Quantity::new(11.0, amp).unwrap();
        assert_eq!(detector.observe(&id("current"), above, time(1)).unwrap().len(), 1);
        assert!(detector.observe(&id("current"), above, time(2)).unwrap().is_empty());
        detector.observe(&id("current"), Quantity::new(9.0, amp).unwrap(), time(3)).unwrap();
        assert_eq!(detector.observe(&id("current"), above, time(4)).unwrap().len(), 1);
    }

    #[test]
    fn failure_lifecycle_is_monotonic_and_terminal_after_mitigation() {
        let fault = Fault::new(id("f1"), id("cell"), id("open_circuit"), FaultSeverity::Warning, "Cell voltage fell below declared threshold", time(2), None).unwrap();
        let mut record = FailureRecord::new(fault);
        record.acknowledge(time(3)).unwrap(); record.mitigate(time(4)).unwrap();
        assert_eq!(record.status(), FailureStatus::Mitigated);
        assert_eq!(record.history().len(), 3);
        assert_eq!(record.acknowledge(time(5)), Err(FailureError::AlreadyMitigated));
    }
}
