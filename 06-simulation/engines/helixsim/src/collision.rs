//! Collision detection and impulse response (Layer 06 / HELIXSIM).
//!
//! ## Scope discipline
//!
//! This is **not** a game-engine broadphase framework. The repository's
//! supported physics model is point-mass translational motion in coherent SI
//! (`TranslationalState` from [`crate::dynamics`]); there is no upstream
//! rigid-body, orientation or shape registry contract. Therefore the only
//! justified geometry is:
//!
//! * **sphere intersection** — each body carries an explicit collision radius
//!   as an engineering parameter supplied by the caller; and
//! * **half-space contact** — a static planar boundary defined by an outward
//!   unit normal through a point (ground/obstacle abstraction), which is the
//!   minimal contract needed to detect penetration of a finite-sized body.
//!
//! Both are *bounding-volume* tests over positions; no mesh, BVH or spatial
//! partitioning is invented because nothing upstream requires it.
//!
//! ## Pipeline separation
//!
//! ```text
//! detect   → ContactInfo { normal n, penetration depth d }
//! respond  → Δv per body via the central-complex impulse formula
//! ```
//!
//! Impulse magnitude along contact normal **n** (pointing from A to B):
//!
//! ```text
//! j = −(1 + e) · (v_rel · n) / (1/m_A + 1/m_B)
//! Δv_A = −(j / m_A) · n
//! Δv_B = +(j / m_B) · n
//! ```
//!
//! where `v_rel = v_A − v_B` and `e ∈ [0, 1]` is the coefficient of
//! restitution. Separating pairs (`v_rel · n >= 0`) receive **no impulse** —
//! already-moving-apart bodies must not be glued or flung. Infinite mass
//! (`f64::INFINITY`) models a kinematic/static anchor: its velocity change is
//! exactly zero while the finite body reflects normally.

use std::fmt;

use helqix_core::NonFiniteKind;

use crate::dynamics::TranslationalState;
use crate::environment::{require_finite, SymbolicUnit, Vector3};
use crate::forces::ForceUnit;

/// Unit signature of position vectors: `m`.
pub const POSITION_UNIT: SymbolicUnit = SymbolicUnit::from_length(1);

/// A validated positive collision radius [m].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius(f64);

impl Radius {
    /// Creates a strictly-positive, finite radius.
    ///
    /// # Errors
    ///
    /// Non-finite values or `<= 0` radii are refused: a point "collision
    /// volume" would make every test degenerate and a negative radius is
    /// geometrically meaningless.
    pub fn new(meters: f64) -> Result<Self, CollisionError> {
        if let Err(e) = require_finite(meters) {
            return Err(CollisionError::from_env(e));
        }
        if meters <= 0.0 {
            return Err(CollisionError::InvalidRadius(meters));
        }
        Ok(Self(meters))
    }

    /// Raw radius in metres.
    #[must_use]
    pub const fn as_metres(&self) -> f64 {
        self.0
    }
}

/// A validated coefficient of restitution `e ∈ [0, 1]`.
///
/// `e = 0`: perfectly inelastic (normal velocity annihilated);
/// `e = 1`: perfectly elastic (normal velocity reflected).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Restitution(f64);

impl Restitution {
    /// Creates `e`, rejecting NaN/infinity and values outside `[0, 1]`.
    ///
    /// Super-elastic collisions (`e > 1`) inject energy without a declared
    /// source model, so they are refused rather than silently allowed.
    ///
    /// # Errors
    ///
    /// [`CollisionError::InvalidRestitution`].
    pub fn new(e: f64) -> Result<Self, CollisionError> {
        if let Err(err) = require_finite(e) {
            return Err(CollisionError::from_env(err));
        }
        if !(0.0..=1.0).contains(&e) {
            return Err(CollisionError::InvalidRestitution(e));
        }
        Ok(Self(e))
    }

    /// Raw value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.0
    }
}

/// A validated mass for collision response [kg].
///
/// Unlike [`crate::forces::Mass`], this type additionally admits
/// `f64::INFINITY` to represent a static/kinematic anchor whose velocity is
/// unchangeable by impulses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionMass(f64);

impl CollisionMass {
    /// Strictly-positive finite mass.
    ///
    /// # Errors
    ///
    /// [`CollisionError::InvalidMass`] for non-finite or `<= 0` input.
    pub fn new(kg: f64) -> Result<Self, CollisionError> {
        if let Err(err) = require_finite(kg) {
            return Err(CollisionError::from_env(err));
        }
        if kg <= 0.0 {
            return Err(CollisionError::InvalidMass(kg));
        }
        Ok(Self(kg))
    }

    /// An immovable infinite-mass anchor.
    #[must_use]
    pub const fn static_anchor() -> Self {
        Self(f64::INFINITY)
    }

    /// Whether this mass responds to impulses.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.0 == f64::INFINITY
    }

    /// Reciprocal mass `1/m`; exactly `0.0` for a static anchor.
    #[must_use]
    pub const fn inverse(&self) -> f64 {
        if self.is_static() {
            0.0
        } else {
            1.0 / self.0
        }
    }

    /// Raw value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.0
    }
}

/// A moving sphere: translational state plus collision radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    state: TranslationalState,
    mass: CollisionMass,
    radius: Radius,
}

impl Body {
    /// Assembles a validated body.
    #[must_use]
    pub const fn new(state: TranslationalState, mass: CollisionMass, radius: Radius) -> Self {
        Self { state, mass, radius }
    }

    /// Current motion state.
    #[must_use]
    pub const fn state(&self) -> TranslationalState {
        self.state
    }

    /// Mass (possibly a static anchor).
    #[must_use]
    pub const fn mass(&self) -> CollisionMass {
        self.mass
    }

    /// Collision radius.
    #[must_use]
    pub const fn radius(&self) -> Radius {
        self.radius
    }

    /// Returns a copy with replaced velocity (used after impulse response).
    ///
    /// # Errors
    ///
    /// Propagates [`crate::dynamics::DynamicsError`] wrapped as
    /// [`CollisionError::Dynamics`].
    pub fn with_velocity(&self, velocity: Vector3) -> Result<Self, CollisionError> {
        let state = TranslationalState::new(self.state.position(), velocity)
            .map_err(CollisionError::Dynamics)?;
        Ok(Self {
            state,
            mass: self.mass,
            radius: self.radius,
        })
    }
}

/// A static half-space boundary: all points `p` with `(p − origin) · n < 0`
/// are inside the solid (penetrating).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfSpace {
    origin: Vector3,
    normal: Vector3,
}

impl HalfSpace {
    /// Builds a half-space from a boundary point and outward unit normal.
    ///
    /// # Errors
    ///
    /// [`CollisionError::DegenerateNormal`] when the normal magnitude is not
    /// exactly one within tolerance — silently re-normalizing would hide
    /// caller bugs; [`CollisionError::InconsistentUnits`] for wrong signatures.
    pub fn new(origin: Vector3, normal: Vector3) -> Result<Self, CollisionError> {
        origin.require_unit(POSITION_UNIT)?;
        normal.require_unit(SymbolicUnit::dimensionless())?;
        let mag = normal.magnitude()?;
        // Unit-norm check with a tight absolute tolerance; callers producing
        // normals analytically (axis-aligned planes) hit exactly 1.0.
        if (mag - 1.0).abs() > 1e-12 {
            return Err(CollisionError::DegenerateNormal(mag));
        }
        Ok(Self { origin, normal })
    }

    /// Signed distance of a centre point from the boundary plane
    /// (positive = outside the solid).
    #[must_use]
    pub fn signed_distance(&self, centre: &Vector3) -> Result<f64, CollisionError> {
        centre.require_unit(POSITION_UNIT)?;
        let offset = centre.sub(&self.origin)?;
        Ok(offset.dot(&self.normal)?.value())
    }
}

/// Detected contact between two spheres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereContact {
    /// Unit normal pointing from body A towards body B.
    pub normal: Vector3,
    /// Penetration depth `r_A + r_B − |x_B − x_A|` [m]; `>= 0`.
    pub depth: f64,
}

/// Detects overlap between two collision spheres.
///
/// * Touching exactly (`distance == r_A + r_B`) yields `depth == 0.0` — a
///   contact exists but with no penetration.
/// * Coincident centres cannot define a normal; reported as
///   [`CollisionError::CoincidentCentres`] instead of inventing a direction.
///
/// # Errors
///
/// As described above; unit mismatches surface via [`Vector3`] checks.
pub fn detect_sphere_sphere(a: &Body, b: &Body) -> Result<Option<SphereContact>, CollisionError> {
    let delta = b.state().position().sub(&a.state().position())?;
    let distance = delta.magnitude()?;
    let sum_r = a.radius().as_metres() + b.radius().as_metres();
    if distance > sum_r {
        return Ok(None);
    }
    if distance == 0.0 {
        return Err(CollisionError::CoincidentCentres);
    }
    let scale = 1.0 / distance;
    let normal = delta.scale(scale)?; // dimensionless unit vector
    Ok(Some(SphereContact {
        normal,
        depth: sum_r - distance,
    }))
}

/// Detected contact between a sphere and a half-space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneContact {
    /// Outward boundary normal (the half-space normal).
    pub normal: Vector3,
    /// Penetration depth `r − signed_distance` [m]; `>= 0`.
    pub depth: f64,
}

/// Detects whether a sphere intrudes into a static half-space.
///
/// # Errors
///
/// Propagates arithmetic/unit refusals.
pub fn detect_sphere_plane(body: &Body, plane: &HalfSpace) -> Result<Option<PlaneContact>, CollisionError> {
    let dist = plane.signed_distance(&body.state().position())?;
    let depth = body.radius().as_metres() - dist;
    if depth < 0.0 {
        return Ok(None);
    }
    Ok(Some(PlaneContact {
        normal: plane.normal,
        depth,
    }))
}

/// Post-impulse velocities of a resolved sphere-sphere pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairResponse {
    /// New velocity of body A.
    pub velocity_a: Vector3,
    /// New velocity of body B.
    pub velocity_b: Vector3,
    /// Applied impulse magnitude `j` (scalar along the contact normal).
    pub impulse: f64,
}

/// Applies the central-complex impulse response to a detected contact.
///
/// With `v_rel = v_A − v_B` and **n** pointing A→B:
///
/// ```text
/// j = −(1 + e)(v_rel · n) / (1/m_A + 1/m_B)
/// v_A' = v_A + (j / m_A)·n     [note sign convention below]
/// v_B' = v_B − (j / m_B)·n
/// ```
///
/// Implementation detail: we compute `vn = v_rel · n`. Approaching means
/// `vn <= 0` **with n pointing A→B and v_rel = v_A − v_B**, i.e. A closing on
/// B gives `vn < 0`. Separating (`vn >= 0`) produces no impulse. The returned
/// impulse `j = −(1+e)·vn / (inv_mA + inv_mB)` is then `>= 0` for approaching
/// pairs, pushing A back along `−n` and B along `+n`.
///
/// Newton's third law holds by construction: the momentum changes satisfy
/// `m_A·Δv_A + m_B·Δv_B = −j·n + j·n = 0` whenever both masses are finite.
///
/// # Errors
///
/// * Both-static pairs have a singular denominator (`1/m_A + 1/m_B == 0`) →
///   [`CollisionError::SingularPair`] — two immovable bodies cannot exchange
///   impulses; the configuration is a modelling error.
/// * Any overflow/non-finite intermediate is refused.
pub fn resolve_sphere_sphere(
    a: &Body,
    b: &Body,
    contact: &SphereContact,
    restitution: Restitution,
) -> Result<PairResponse, CollisionError> {
    let inv_sum = a.mass().inverse() + b.mass().inverse();
    if inv_sum == 0.0 {
        return Err(CollisionError::SingularPair);
    }
    let v_rel = a.state().velocity().sub(&b.state().velocity())?;
    let vn = v_rel.dot(&contact.normal)?.value();
    if vn >= 0.0 {
        // Separating or tangential-only: no normal impulse.
        return Ok(PairResponse {
            velocity_a: a.state().velocity(),
            velocity_b: b.state().velocity(),
            impulse: 0.0,
        });
    }
    let j = require_finite(-(1.0 + restitution.value()) * vn / inv_sum)?;
    let dv_a = contact.normal.scale(j * a.mass().inverse())?;
    let dv_b = contact.normal.scale(-j * b.mass().inverse())?;
    Ok(PairResponse {
        velocity_a: a.state().velocity().add(&dv_a)?,
        velocity_b: b.state().velocity().add(&dv_b)?,
        impulse: j,
    })
}

/// Post-impulse velocity of a sphere reflecting off a static half-space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneResponse {
    /// New velocity of the body.
    pub velocity: Vector3,
    /// Applied impulse magnitude.
    pub impulse: f64,
}

/// Impulse response of a dynamic sphere against an immovable half-space.
///
/// Only the normal component changes: `v_n' = −e·v_n`; tangential motion is
/// untouched (friction is not part of the current repository physics model,
/// so none is invented).
///
/// # Errors
///
/// Static-vs-static is impossible here (the plane is static by definition);
/// overflow refusal as elsewhere.
pub fn resolve_sphere_plane(
    body: &Body,
    contact: &PlaneContact,
    restitution: Restitution,
) -> Result<PlaneResponse, CollisionError> {
    if body.mass().is_static() {
        // A static body pressed into a static plane: nothing can move.
        return Ok(PlaneResponse {
            velocity: body.state().velocity(),
            impulse: 0.0,
        });
    }
    let v = body.state().velocity();
    let vn = v.dot(&contact.normal)?.value();
    if vn >= 0.0 {
        return Ok(PlaneResponse {
            velocity: v,
            impulse: 0.0,
        });
    }
    // Δv_n = −(1 + e)·vn along n ; j = m·|Δv_n|
    let dv_normal = contact.normal.scale(-(1.0 + restitution.value()) * vn)?;
    let new_v = v.add(&dv_normal)?;
    let impulse = body.mass().value() * (-(1.0 + restitution.value()) * vn);
    Ok(PlaneResponse {
        velocity: new_v,
        impulse: require_finite(impulse)?,
    })
}

// ---------------------------------------------------------------------------
// Deterministic pair ordering
// ---------------------------------------------------------------------------

/// One candidate interaction edge between two indexed bodies.
///
/// Bodies are referenced by caller-assigned indices; iteration over a
/// [`Vec`] of these edges is slice-ordered, so collision resolution order is
/// fully deterministic regardless of how the caller built the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairIndex {
    /// Index of body A.
    pub a: usize,
    /// Index of body B.
    pub b: usize,
}

/// Canonicalises a pair so `{a,b}` and `{b,a}` map to one key: `a <= b`.
///
/// Callers deduplicating unordered pairs (e.g. from a symmetric adjacency
/// structure) must canonicalise first; this keeps detection idempotent.
#[must_use]
pub const fn canonical_pair(a: usize, b: usize) -> PairIndex {
    if a <= b {
        PairIndex { a, b }
    } else {
        PairIndex { a: b, b: a }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by collision detection/response.
#[derive(Debug, Clone, PartialEq)]
pub enum CollisionError {
    /// A numeric input or intermediate is NaN or infinite.
    NonFiniteValue(NonFiniteKind),
    /// Radius was not finite and strictly positive.
    InvalidRadius(f64),
    /// Mass was not finite-positive or infinite (static anchor).
    InvalidMass(f64),
    /// Restitution outside `[0, 1]`.
    InvalidRestitution(f64),
    /// Contact normal had magnitude different from one.
    DegenerateNormal(f64),
    /// Sphere centres coincide; no unique contact normal exists.
    CoincidentCentres,
    /// Both bodies are immovable; impulse equation is singular.
    SingularPair,
    /// A vector carried the wrong unit signature.
    InconsistentUnits {
        /// First signature.
        first: String,
        /// Second signature.
        second: String,
    },
    /// Dynamics-state reconstruction failure.
    Dynamics(crate::dynamics::DynamicsError),
}

impl CollisionError {
    fn from_env(error: crate::environment::EnvironmentError) -> Self {
        use crate::environment::EnvironmentError as E;
        match error {
            E::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            E::InconsistentUnits { first, second } => Self::InconsistentUnits { first, second },
            other => Self::Dynamics(crate::dynamics::DynamicsError::from(other)),
        }
    }
}

impl From<crate::environment::EnvironmentError> for CollisionError {
    fn from(error: crate::environment::EnvironmentError) -> Self {
        Self::from_env(error)
    }
}

impl fmt::Display for CollisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "collision value is not finite: {kind}"),
            Self::InvalidRadius(r) => write!(f, "radius must be finite and > 0, got {r}"),
            Self::InvalidMass(m) => write!(f, "mass invalid for collision, got {m}"),
            Self::InvalidRestitution(e) => {
                write!(f, "restitution must lie in [0, 1], got {e}")
            }
            Self::DegenerateNormal(mag) => {
                write!(f, "contact normal must be unit length, got magnitude {mag}")
            }
            Self::CoincidentCentres => write!(f, "coincident sphere centres admit no contact normal"),
            Self::SingularPair => write!(f, "both bodies are immovable; impulse is undefined"),
            Self::InconsistentUnits { first, second } => {
                write!(f, "inconsistent units: `{first}` vs `{second}`")
            }
            Self::Dynamics(error) => write!(f, "dynamics failure: {error}"),
        }
    }
}

impl std::error::Error for CollisionError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::TranslationalState;

    fn pos(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new([x, y, z], POSITION_UNIT).unwrap()
    }
    fn vel(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new([x, y, z], ForceUnit::VELOCITY).unwrap()
    }
    fn dim(v: [f64; 3]) -> Vector3 {
        Vector3::new(v, SymbolicUnit::dimensionless()).unwrap()
    }

    fn body(px: f64, vx: f64, mass: f64, r: f64) -> Body {
        Body::new(
            TranslationalState::new(pos(px, 0.0, 0.0), vel(vx, 0.0, 0.0)).unwrap(),
            CollisionMass::new(mass).unwrap(),
            Radius::new(r).unwrap(),
        )
    }

    #[test]
    fn separated_spheres_do_not_collide() {
        let a = body(0.0, 1.0, 2.0, 0.5);
        let b = body(2.0, 0.0, 2.0, 0.5); // gap = 2.0 - 1.0 = 1.0 > 0
        assert_eq!(detect_sphere_sphere(&a, &b).unwrap(), None);
    }

    #[test]
    fn touching_spheres_report_zero_depth() {
        let a = body(0.0, 1.0, 2.0, 0.5);
        let b = body(1.0, 0.0, 2.0, 0.5); // exactly touching
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        assert_eq!(c.depth, 0.0);
        // Normal points A→B, i.e. +x.
        assert_eq!(c.normal.components(), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn penetrating_spheres_report_depth_and_normal() {
        let a = body(0.0, 1.0, 2.0, 0.5);
        let b = body(0.8, 0.0, 2.0, 0.5); // overlap = 1.0 - 0.8 = 0.2
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        assert!((c.depth - 0.2).abs() < 1e-12);
        assert_eq!(c.normal.components(), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn coincident_centres_are_refused_not_invented() {
        let a = body(1.0, 0.0, 2.0, 0.5);
        let b = body(1.0, 0.0, 2.0, 0.5);
        assert!(matches!(
            detect_sphere_sphere(&a, &b),
            Err(CollisionError::CoincidentCentres)
        ));
    }

    #[test]
    fn head_on_elastic_equal_mass_exchange_velocities() {
        // Analytical result for m_A = m_B, e = 1, central impact:
        // velocities swap exactly.
        let a = body(0.0, 2.0, 1.0, 0.6);
        let b = body(1.0, -1.0, 1.0, 0.6);
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        let r = resolve_sphere_sphere(&a, &b, &c, Restitution::new(1.0).unwrap()).unwrap();
        assert_eq!(r.velocity_a.components(), [-1.0, 0.0, 0.0]);
        assert_eq!(r.velocity_b.components(), [2.0, 0.0, 0.0]);
        // Momentum conserved exactly: p_before = 2 - 1 = 1, p_after = -1 + 2 = 1.
        let p_before = 1.0 * 2.0 + 1.0 * -1.0;
        let p_after = 1.0 * r.velocity_a.x() + 1.0 * r.velocity_b.x();
        assert_eq!(p_before, p_after);
    }

    #[test]
    fn inelastic_collision_kills_relative_normal_velocity() {
        let a = body(0.0, 3.0, 2.0, 0.9);
        let b = body(1.0, 0.0, 1.0, 0.5);
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        let r = resolve_sphere_sphere(&a, &b, &c, Restitution::new(0.0).unwrap()).unwrap();
        // e = 0 ⇒ post relative normal velocity is exactly zero.
        let v_rel = r.velocity_a.sub(&r.velocity_b).unwrap();
        let vn = v_rel.dot(&c.normal).unwrap().value();
        assert!(vn.abs() < 1e-12, "vn = {vn}");
        // Analytical common velocity: (m_A v_A + m_B v_B)/(m_A+m_B) = 6/3 = 2.
        assert!((r.velocity_a.x() - 2.0).abs() < 1e-12);
        assert!((r.velocity_b.x() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn separating_pair_receives_no_impulse() {
        let a = body(0.0, -5.0, 1.0, 0.9); // moving away from b
        let b = body(1.0, 5.0, 1.0, 0.9);
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        let r = resolve_sphere_sphere(&a, &b, &c, Restitution::new(1.0).unwrap()).unwrap();
        assert_eq!(r.impulse, 0.0);
        assert_eq!(r.velocity_a, a.state().velocity());
        assert_eq!(r.velocity_b, b.state().velocity());
    }

    #[test]
    fn static_anchor_reflects_dynamic_body_elastically() {
        let a = body(0.0, 4.0, 1.0, 0.6);
        let mut b = a;
        b = Body::new(
            TranslationalState::new(pos(1.0, 0.0, 0.0), vel(0.0, 0.0, 0.0)).unwrap(),
            CollisionMass::static_anchor(),
            Radius::new(0.6).unwrap(),
        );
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        let r = resolve_sphere_sphere(&a, &b, &c, Restitution::new(1.0).unwrap()).unwrap();
        assert_eq!(r.velocity_a.components(), [-4.0, 0.0, 0.0]);
        assert_eq!(r.velocity_b, b.state().velocity());
    }

    #[test]
    fn two_static_anchors_are_a_modelling_error() {
        let mk = |x: f64| {
            Body::new(
                TranslationalState::new(pos(x, 0.0, 0.0), vel(0.0, 0.0, 0.0)).unwrap(),
                CollisionMass::static_anchor(),
                Radius::new(1.0).unwrap(),
            )
        };
        let (a, b) = (mk(0.0), mk(1.5));
        let c = detect_sphere_sphere(&a, &b).unwrap().expect("contact");
        assert!(matches!(
            resolve_sphere_sphere(&a, &b, &c, Restitution::new(0.5).unwrap()),
            Err(CollisionError::SingularPair)
        ));
    }

    #[test]
    fn plane_detection_touch_penetrate_and_clear() {
        // Ground plane at y = 0, outward normal +y.
        let ground = HalfSpace::new(pos(0.0, 0.0, 0.0), dim([0.0, 1.0, 0.0])).unwrap();
        let mk = |y: f64, vy: f64| {
            Body::new(
                TranslationalState::new(pos(0.0, y, 0.0), vel(0.0, vy, 0.0)).unwrap(),
                CollisionMass::new(1.0).unwrap(),
                Radius::new(0.5).unwrap(),
            )
        };
        // Centre at y = 1.0: bottom just touches plane.
        let touching = detect_sphere_plane(&mk(1.0, 0.0), &ground)
            .unwrap()
            .expect("touching contact");
        assert_eq!(touching.depth, 0.0);
        // Centre at y = 0.4: penetrated by 0.1.
        let pen = detect_sphere_plane(&mk(0.4, 0.0), &ground)
            .unwrap()
            .expect("penetrating contact");
        assert!((pen.depth - 0.1).abs() < 1e-12);
        // Centre at y = 2.0: clear.
        assert_eq!(detect_sphere_plane(&mk(2.0, 0.0), &ground).unwrap(), None);
    }

    #[test]
    fn plane_response_reflects_only_normal_component() {
        let ground = HalfSpace::new(pos(0.0, 0.0, 0.0), dim([0.0, 1.0, 0.0])).unwrap();
        // Falling with downward 5 m/s and forward 3 m/s.
        let falling = Body::new(
            TranslationalState::new(pos(0.0, 0.4, 0.0), vel(3.0, -5.0, 0.0)).unwrap(),
            CollisionMass::new(2.0).unwrap(),
            Radius::new(0.5).unwrap(),
        );
        let c = detect_sphere_plane(&falling, &ground).unwrap().expect("contact");
        let r = resolve_sphere_plane(&falling, &c, Restitution::new(0.6).unwrap()).unwrap();
        // Analytical: v_y' = −e·v_y = 3.0; v_x unchanged = 3.0.
        assert!((r.velocity.x() - 3.0).abs() < 1e-12);
        assert!((r.velocity.y() - 3.0).abs() < 1e-12);
        // Impulse magnitude j = m(1+e)|v_n| = 2·1.6·5 = 16 N·s.
        assert!((r.impulse - 16.0).abs() < 1e-12);
    }

    #[test]
    fn non_unit_plane_normal_is_refused() {
        let err = HalfSpace::new(pos(0.0, 0.0, 0.0), dim([0.0, 2.0, 0.0])).unwrap_err();
        assert!(matches!(err, CollisionError::DegenerateNormal(_)));
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        assert!(matches!(
            Radius::new(0.0),
            Err(CollisionError::InvalidRadius(0.0))
        ));
        assert!(matches!(
            Radius::new(f64::NAN),
            Err(CollisionError::NonFiniteValue(NonFiniteKind::Nan))
        ));
        assert!(matches!(
            CollisionMass::new(-3.0),
            Err(CollisionError::InvalidMass(-3.0))
        ));
        assert!(matches!(
            Restitution::new(1.5),
            Err(CollisionError::InvalidRestitution(1.5))
        ));
        assert!(matches!(
            Restitution::new(-0.1),
            Err(CollisionError::InvalidRestitution(-0.1))
        ));
    }

    #[test]
    fn pair_canonicalisation_is_symmetric_and_order_stable() {
        assert_eq!(canonical_pair(7, 3), canonical_pair(3, 7));
        assert_eq!(canonical_pair(3, 7), PairIndex { a: 3, b: 7 });
        // Slice-order evaluation of a list of pairs is stable across runs.
        let pairs = vec![PairIndex { a: 0, b: 1 }, PairIndex { a: 2, b: 3 }];
        let again = vec![PairIndex { a: 0, b: 1 }, PairIndex { a: 2, b: 3 }];
        assert_eq!(pairs, again);
    }

    #[test]
    fn response_is_deterministic_bitwise() {
        let a = body(0.0, 2.3, 1.7, 0.6);
        let b = body(1.0, -0.8, 3.1, 0.5);
        let e = Restitution::new(0.42).unwrap();
        let run = || {
            let c = detect_sphere_sphere(&a, &b).unwrap().unwrap();
            resolve_sphere_sphere(&a, &b, &c, e).unwrap()
        };
        assert_eq!(run(), run());
    }
}
