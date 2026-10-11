//! Action contract for HELQIX Layer 05 — Execution.
//!
//! An [`Action`] is a deterministic, immutable description of *what*
//! execution is being asked to do against the engineering reality. It
//! is not an executor: it does not run, it does not mutate state, it
//! does not step simulation, it does not read the clock, and it does
//! not decide whether the requested operation is possible.
//!
//! # Responsibility boundary
//!
//! This file owns **only**:
//!
//! * a stable identity ([`ActionId`]),
//! * a construction-time validation hook
//!   ([`ActionPayload::validate`]),
//! * immutability of the constructed value, and
//! * a structured, non-panicking error surface ([`ActionError`]).
//!
//! It deliberately owns **nothing** else. In particular, it does not
//! own:
//!
//! * engineering state — Layer 04,
//! * physics or simulation — Layer 06,
//! * logical time — Layer 07,
//! * scheduling — [`crate::scheduler`],
//! * execution orchestration — [`crate::executor`],
//! * the runtime — [`crate::runtime`],
//! * execution graph placement — [`crate::execution_graph`],
//! * the state transition itself — [`crate::transition`].
//!
//! # Domain independence
//!
//! Layer 05 is deliberately domain-independent. It does **not** know
//! about motors, batteries, drones, vehicles, or any other concrete
//! physical operation. A domain supplies its own content — kind,
//! target, parameters, preconditions — by implementing
//! [`ActionPayload`]. The domain's action is then
//! [`Action<MyPayload>`].
//!
//! # Immutability
//!
//! [`Action`] has **no `&mut self` methods**. Once constructed, its
//! identity and its payload cannot be changed by any safe operation.
//! This preserves replay identity: the same `Action` value at the same
//! pipeline position means the same requested operation, on every
//! run, in every process, on every platform.
//!
//! # Determinism
//!
//! No randomness, no wall clock, no global counters, no interior
//! mutability, no thread-scheduling input, no I/O, no `unsafe`. Two
//! `Action<P>` values with equal `id` and equal `payload` are equal.
//!
//! # Structural vs. execution validation
//!
//! [`ActionPayload::validate`] performs **structural** validation only:
//! is the payload well-formed as a value? ("a target is present",
//! "a required field is not empty", "two parameters are not mutually
//! exclusive").
//!
//! It must **not** perform **execution-time** validation: whether the
//! requested operation is possible against the current engineering
//! reality belongs to Layer 04 (via the executor), not to `action.rs`.
//! Keeping these two boundaries separate is a HELQIX invariant.

use core::fmt;
use core::hash::Hash;

// ============================================================================
// ActionId
// ============================================================================

/// Stable identifier for a HELQIX action.
///
/// An opaque, `Copy`-able wrapper around a `u64`. The caller supplies
/// the raw value — typically from a Foundation-layer ID allocator —
/// and Layer 05 never invents one. There is no global counter here, no
/// wall clock, no randomness.
///
/// If `helqix-core` later exposes a canonical typed ID, `ActionId` can
/// be re-implemented as a newtype over that type without changing any
/// other public surface in this file.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ActionId(u64);

impl ActionId {
    /// Wrap a caller-supplied raw identifier.
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw identifier value.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ActionId({})", self.0)
    }
}

// ============================================================================
// ActionPayload
// ============================================================================

/// Domain-supplied content of an [`Action`].
///
/// A HELQIX domain (Layer 03, or an application crate) implements this
/// trait for its own action-content type. The trait's only obligation
/// is **structural validation**: at construction, the payload must be
/// able to say whether *it* is well-formed.
///
/// The payload decides what an action *is*:
///
/// * the kind of operation (e.g. an enum),
/// * the target entity or component,
/// * the parameters, quantities, or units,
/// * any preconditions or expected effects.
///
/// Layer 05 never looks inside. This preserves domain independence and
/// keeps the pipeline monomorphic over a single type parameter.
///
/// # Not required
///
/// The payload need not implement `Clone`, `Serialize`, `Ord`, or any
/// trait other than this one. Downstream layers add the bounds they
/// actually require.
pub trait ActionPayload: Sized {
    /// Structural validation.
    ///
    /// Called exactly once, from [`Action::try_new`]. Must return an
    /// [`ActionError`] describing the structural problem when the
    /// payload is malformed.
    ///
    /// Must **not** consult Layer 04 state, Layer 06 simulation, or
    /// Layer 07 time. Whether the action can run against the current
    /// reality is a separate, later validation.
    fn validate(&self) -> Result<(), ActionError>;
}

// ============================================================================
// ActionError
// ============================================================================

/// Error returned when an action's payload fails structural
/// validation.
///
/// Deliberately minimal: a single reason string. The failure surface
/// is small, the reason is human-readable, and a caller that needs to
/// discriminate between failure modes can do so by comparing
/// [`Self::reason`] or by having the payload return structured data
/// in its own type and wrapping it into a reason string here.
///
/// Non-panicking, `Display`-able, and `std::error::Error` — matching
/// the shape of the error types in Layer 07.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ActionError {
    reason: &'static str,
}

impl ActionError {
    /// Construct an error from a `'static` reason string.
    #[inline]
    #[must_use]
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    /// The reason string.
    #[inline]
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid action: {}", self.reason)
    }
}

impl std::error::Error for ActionError {}

// ============================================================================
// Action
// ============================================================================

/// An immutable, deterministic execution request.
///
/// See the module documentation for the responsibility boundary. This
/// type is a **description**, never an executor.
///
/// # Type parameter
///
/// `P` is the domain-supplied action content. It is `Sized` and
/// implements [`ActionPayload`]; no other bound is imposed here.
///
/// # Invariants
///
/// 1. Once constructed, `id` and `payload` cannot be mutated by any
///    safe operation — there are no `&mut self` methods.
/// 2. Construction runs [`ActionPayload::validate`] exactly once and
///    rejects the payload on failure.
/// 3. Two `Action<P>` values are equal iff their `id` and `payload`
///    are equal (derived `PartialEq` / `Eq`).
/// 4. There is no interior mutability, no global counter, no
///    wall-clock read, and no `unsafe` in this type or its methods.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Action<P: ActionPayload> {
    id: ActionId,
    payload: P,
}

impl<P: ActionPayload> Action<P> {
    /// Construct an action, validating its payload.
    ///
    /// Returns `Err(ActionError)` if the payload's
    /// [`ActionPayload::validate`] rejects it. On success, the
    /// returned action is fully formed and immutable.
    ///
    /// # Determinism
    ///
    /// The only observable side-effect is whatever `payload.validate()`
    /// itself does (typically: nothing). This function does not read
    /// the clock, touch global state, or allocate beyond what `P`
    /// already contains.
    pub fn try_new(id: ActionId, payload: P) -> Result<Self, ActionError> {
        payload.validate()?;
        Ok(Self { id, payload })
    }

    /// The stable action identity.
    #[inline]
    #[must_use]
    pub const fn id(&self) -> ActionId {
        self.id
    }

    /// Borrow the domain-supplied payload.
    #[inline]
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    /// Consume the action and return its payload.
    ///
    /// Useful when a downstream layer (`executor`, `transition`) needs
    /// to move the payload out of the action without cloning.
    #[inline]
    #[must_use]
    pub fn into_payload(self) -> P {
        self.payload
    }

    /// Consume the action and return both parts.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (ActionId, P) {
        (self.id, self.payload)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative domain payload used only by these tests. It
    /// proves the API surface is usable for an arbitrary domain type
    /// without Layer 05 knowing anything about it.
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    struct SetParameter {
        target: u32,
        value: i64,
    }

    impl ActionPayload for SetParameter {
        fn validate(&self) -> Result<(), ActionError> {
            if self.target == 0 {
                return Err(ActionError::new("target must be non-zero"));
            }
            Ok(())
        }
    }

    fn id(n: u64) -> ActionId {
        ActionId::from_raw(n)
    }

    fn ok_payload() -> SetParameter {
        SetParameter {
            target: 42,
            value: 7,
        }
    }

    // ------------------------------------------------------------------
    // 1. Valid construction
    // ------------------------------------------------------------------

    #[test]
    fn valid_action_constructs_and_preserves_id_and_payload() {
        let a = Action::try_new(id(1), ok_payload()).unwrap();
        assert_eq!(a.id(), id(1));
        assert_eq!(a.payload().target, 42);
        assert_eq!(a.payload().value, 7);
    }

    // ------------------------------------------------------------------
    // 2. Structural validation
    // ------------------------------------------------------------------

    #[test]
    fn invalid_payload_is_rejected_with_structured_error() {
        let bad = SetParameter {
            target: 0,
            value: 0,
        };
        let err = Action::try_new(id(1), bad).unwrap_err();
        assert_eq!(err.reason(), "target must be non-zero");
    }

    #[test]
    fn error_display_is_informative() {
        let err = ActionError::new("kind must not be empty");
        let s = err.to_string();
        assert!(s.contains("kind must not be empty"), "got: {}", s);
        // And it implements std::error::Error.
        let _boxed: Box<dyn std::error::Error> = Box::new(err);
    }

    #[test]
    fn error_reason_is_accessible_and_exact() {
        let e = ActionError::new("x");
        assert_eq!(e.reason(), "x");
        assert_eq!(e, ActionError::new("x"));
        assert_ne!(e, ActionError::new("y"));
    }

    // ------------------------------------------------------------------
    // 3. Identity stability
    // ------------------------------------------------------------------

    #[test]
    fn id_is_stable_through_construction_and_extraction() {
        let a = Action::try_new(id(9_999), ok_payload()).unwrap();
        assert_eq!(a.id().raw(), 9_999);
        let (extracted, _) = a.into_parts();
        assert_eq!(extracted.raw(), 9_999);
    }

    #[test]
    fn action_id_is_copy_and_totally_ordered() {
        let a = id(1);
        let b = a; // Copy
        assert_eq!(a, b);
        assert!(id(1) < id(2));
        assert!(id(2) > id(1));
        assert!(id(1) <= id(1));
        assert!(id(1) >= id(1));
    }

    // ------------------------------------------------------------------
    // 4. Deterministic equality
    // ------------------------------------------------------------------

    #[test]
    fn equal_id_and_payload_yield_equal_actions() {
        let a = Action::try_new(id(5), ok_payload()).unwrap();
        let b = Action::try_new(id(5), ok_payload()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_id_yields_distinct_actions() {
        let a = Action::try_new(id(5), ok_payload()).unwrap();
        let b = Action::try_new(id(6), ok_payload()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_payload_yields_distinct_actions() {
        let a = Action::try_new(
            id(5),
            SetParameter {
                target: 1,
                value: 2,
            },
        )
        .unwrap();
        let b = Action::try_new(
            id(5),
            SetParameter {
                target: 1,
                value: 3,
            },
        )
        .unwrap();
        assert_ne!(a, b);
    }

    // ------------------------------------------------------------------
    // 5. Ownership transfer
    // ------------------------------------------------------------------

    #[test]
    fn into_parts_returns_id_and_payload_by_value() {
        let a = Action::try_new(id(3), ok_payload()).unwrap();
        let (aid, payload) = a.into_parts();
        assert_eq!(aid, id(3));
        assert_eq!(payload, ok_payload());
    }

    #[test]
    fn into_payload_returns_payload_only() {
        let a = Action::try_new(id(4), ok_payload()).unwrap();
        let payload = a.into_payload();
        assert_eq!(payload, ok_payload());
    }

    // ------------------------------------------------------------------
    // 6. Validation runs exactly once at construction
    // ------------------------------------------------------------------

    #[test]
    fn validation_runs_only_at_construction_not_on_access() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLS: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug, Eq, PartialEq)]
        struct Counting;
        impl ActionPayload for Counting {
            fn validate(&self) -> Result<(), ActionError> {
                CALLS.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        CALLS.store(0, Ordering::SeqCst);

        let a = Action::try_new(id(1), Counting).unwrap();
        let _b = Action::try_new(id(2), Counting).unwrap();
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);

        // Accessors do not re-run validation.
        let _ = a.id();
        let _ = a.payload();
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    }

    // ------------------------------------------------------------------
    // 7. Failure does not produce a half-built Action
    // ------------------------------------------------------------------

    #[test]
    fn failed_construction_returns_only_the_error() {
        // Type-level assertion: `try_new` returns
        // `Result<Action<P>, ActionError>`; on error there is no
        // Action to leak.
        let result = Action::try_new(
            id(1),
            SetParameter {
                target: 0,
                value: 0,
            },
        );
        assert!(result.is_err());
        let _err = result.unwrap_err();
    }

    // ------------------------------------------------------------------
    // 8. Hash / Eq consistency
    // ------------------------------------------------------------------

    #[test]
    fn equal_actions_hash_equal() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn h<P: ActionPayload + Hash>(a: &Action<P>) -> u64 {
            let mut hasher = DefaultHasher::new();
            a.hash(&mut hasher);
            hasher.finish()
        }

        let a = Action::try_new(id(1), ok_payload()).unwrap();
        let b = Action::try_new(id(1), ok_payload()).unwrap();
        assert_eq!(a, b);
        assert_eq!(h(&a), h(&b));
    }

    // ------------------------------------------------------------------
    // 9. Domain independence: two payload types coexist cleanly
    // ------------------------------------------------------------------

    #[test]
    fn actions_of_different_payload_types_coexist() {
        #[derive(Debug, Eq, PartialEq)]
        struct Other;
        impl ActionPayload for Other {
            fn validate(&self) -> Result<(), ActionError> {
                Ok(())
            }
        }

        let _a: Action<SetParameter> = Action::try_new(id(1), ok_payload()).unwrap();
        let _b: Action<Other> = Action::try_new(id(1), Other).unwrap();
        // Compilation is the test: two different domain payloads
        // produce two distinct Action types without Layer 05 knowing
        // anything about either domain.
    }

    // ------------------------------------------------------------------
    // 10. Cloning is explicit and preserves identity
    // ------------------------------------------------------------------

    #[test]
    fn cloning_preserves_id_and_payload() {
        let a = Action::try_new(id(7), ok_payload()).unwrap();
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.id(), b.id());
        assert_eq!(a.payload(), b.payload());
    }

    // ------------------------------------------------------------------
    // 11. Determinism: identical construction yields identical values
    // ------------------------------------------------------------------

    #[test]
    fn identical_construction_is_deterministic() {
        let build = || Action::try_new(id(100), ok_payload()).unwrap();
        let a = build();
        let b = build();
        let c = build();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    // ------------------------------------------------------------------
    // 12. Boundary: maximum ActionId
    // ------------------------------------------------------------------

    #[test]
    fn maximum_action_id_is_supported() {
        let a = Action::try_new(id(u64::MAX), ok_payload()).unwrap();
        assert_eq!(a.id().raw(), u64::MAX);
    }

    // ------------------------------------------------------------------
    // 13. Boundary: minimum ActionId
    // ------------------------------------------------------------------

    #[test]
    fn minimum_action_id_is_supported() {
        let a = Action::try_new(id(0), ok_payload()).unwrap();
        assert_eq!(a.id().raw(), 0);
    }

    // ------------------------------------------------------------------
    // 14. Payload borrowing does not require Clone
    // ------------------------------------------------------------------

    #[test]
    fn payload_can_be_non_clone() {
        #[derive(Debug, Eq, PartialEq)]
        struct NonClone(String);

        impl ActionPayload for NonClone {
            fn validate(&self) -> Result<(), ActionError> {
                if self.0.is_empty() {
                    return Err(ActionError::new("payload must not be empty"));
                }
                Ok(())
            }
        }

        let a = Action::try_new(id(1), NonClone("x".into())).unwrap();
        assert_eq!(a.payload(), &NonClone("x".into()));

        let b = Action::try_new(id(2), NonClone(String::new()));
        assert!(b.is_err());
    }
}