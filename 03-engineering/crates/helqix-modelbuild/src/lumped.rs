//! The one translation from a committed model to something the engine can run today: a single lumped body.
//!
//! The engine's runtime is a point mass with air drag. A model of several parts can be run honestly only as
//! that: mass is the sum of volume x stated density over the parts, and the frontal area is the footprint of the
//! parts' bounding box seen from above (x and z). Nothing is guessed. A part with no stated density, imported
//! geometry (no known volume), an unknown shape, or dimensions in an unexpected unit makes the model
//! unrunnable, and the part is named with the reason.
//!
//! Units accepted: dimensions in `mm`, positions in `m`, density in `kg/m3`. Anything else is refused rather than
//! converted by guesswork.
use helqix_contracts::{Geometry, ModelOp, Value};
use std::collections::BTreeMap;

pub const DENSITY_KEY: &str = "density_kg_m3";

#[derive(Debug, Clone, PartialEq)]
pub struct Lumped { pub mass_kg: f64, pub area_m2: f64, pub parts: usize }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Missing { pub part: String, pub reason: String }

struct P { name: String, geometry: Geometry, pos: [f64; 3], density: Option<Value> }

fn dim_m(v: Option<&Value>) -> Result<f64, String> {
    let v = v.ok_or("a dimension is missing")?;
    if v.unit != "mm" { return Err(format!("dimensions must be in mm (found {})", v.unit)); }
    if !(v.v.is_finite() && v.v > 0.0) { return Err("dimensions must be positive numbers".into()); }
    Ok(v.v / 1000.0)
}

/// Volume (m3) and half-extents (m) along x, y, z of a primitive, as the scene draws it.
fn shape(shape: &str, dims: &[Value]) -> Result<(f64, [f64; 3]), String> {
    match shape {
        "sphere" => { let d = dim_m(dims.first())?; Ok(((4.0 / 3.0) * std::f64::consts::PI * (d / 2.0).powi(3), [d / 2.0; 3])) }
        "cylinder" => { let (d, h) = (dim_m(dims.first())?, dim_m(dims.get(1))?); Ok((std::f64::consts::PI * (d / 2.0).powi(2) * h, [d / 2.0, h / 2.0, d / 2.0])) }
        "box" | "plate" => { let (a, b, c) = (dim_m(dims.first())?, dim_m(dims.get(1))?, dim_m(dims.get(2))?); Ok((a * b * c, [a / 2.0, b / 2.0, c / 2.0])) }
        other => Err(format!("unknown shape '{other}'")),
    }
}

/// `ops` are the active operations of a committed revision, in order.
pub fn lumped_body(ops: &[ModelOp]) -> Result<Lumped, Vec<Missing>> {
    let mut parts: BTreeMap<String, P> = BTreeMap::new();
    for op in ops {
        match op {
            ModelOp::PartAdd { id, name, geometry, .. } => { parts.insert(id.clone(), P { name: name.clone(), geometry: geometry.clone(), pos: [0.0; 3], density: None }); }
            ModelOp::PartPlace { id, transform } => { if let Some(p) = parts.get_mut(id) { p.pos = [0, 1, 2].map(|i| if transform.pos[i].unit == "m" { transform.pos[i].v } else { f64::NAN }); } }
            ModelOp::PartSetParam { id, key, value } if key == DENSITY_KEY => { if let Some(p) = parts.get_mut(id) { p.density = Some(value.clone()); } }
            ModelOp::PartRemove { id } => { parts.remove(id); }
            _ => {}
        }
    }
    if parts.is_empty() { return Err(vec![Missing { part: String::new(), reason: "the model has no parts".into() }]); }
    let (mut mass, mut missing) = (0.0, Vec::new());
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (id, p) in &parts {
        let mut fail = |reason: String| missing.push(Missing { part: format!("{} ({id})", p.name), reason });
        let Geometry::Primitive { shape: s, dims } = &p.geometry else { fail("imported geometry has no known volume".into()); continue };
        let Some(d) = &p.density else { fail("no density stated".into()); continue };
        if !matches!(d.unit.as_str(), "kg/m3" | "kg/m^3") || !(d.v.is_finite() && d.v > 0.0) { fail("density must be a positive number in kg/m3".into()); continue; }
        if p.pos.iter().any(|x| !x.is_finite()) { fail("position must be in metres".into()); continue; }
        match shape(s, dims) {
            Err(reason) => fail(reason),
            Ok((vol, e)) => {
                mass += vol * d.v;
                min_x = min_x.min(p.pos[0] - e[0]); max_x = max_x.max(p.pos[0] + e[0]);
                min_z = min_z.min(p.pos[2] - e[2]); max_z = max_z.max(p.pos[2] + e[2]);
            }
        }
    }
    if !missing.is_empty() { missing.sort(); return Err(missing); }
    Ok(Lumped { mass_kg: mass, area_m2: (max_x - min_x) * (max_z - min_z), parts: parts.len() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_contracts::{Maturity, Origin, Provenance, ProvenanceKind, Status, Transform};

    fn v(x: f64, unit: &str) -> Value { Value { v: x, unit: unit.into(), origin: Origin::UserStated, status: Status::Unverified, mock: false } }
    fn add(id: &str, shape: &str, dims: &[f64]) -> ModelOp {
        ModelOp::PartAdd { id: id.into(), name: id.into(), geometry: Geometry::Primitive { shape: shape.into(), dims: dims.iter().map(|d| v(*d, "mm")).collect() }, maturity: Maturity::UserDefined, prov: Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } }
    }
    fn place(id: &str, x: f64, z: f64) -> ModelOp {
        ModelOp::PartPlace { id: id.into(), transform: Transform { pos: [v(x, "m"), v(0.0, "m"), v(z, "m")], rot: [v(0.0, "deg"), v(0.0, "deg"), v(0.0, "deg")], scale: [1.0; 3], frame: "world".into() } }
    }
    fn density(id: &str, d: f64) -> ModelOp { ModelOp::PartSetParam { id: id.into(), key: DENSITY_KEY.into(), value: v(d, "kg/m3") } }

    #[test]
    fn sums_mass_and_takes_the_bounding_footprint() {
        let ops = [add("a", "box", &[100.0, 100.0, 100.0]), place("a", 0.0, 0.0), density("a", 1000.0),
                   add("b", "box", &[100.0, 100.0, 100.0]), place("b", 0.5, 0.0), density("b", 2000.0)];
        let l = lumped_body(&ops).unwrap();
        assert!((l.mass_kg - 3.0).abs() < 1e-9, "{l:?}");
        assert!((l.area_m2 - 0.6 * 0.1).abs() < 1e-9, "{l:?}"); // x from -0.05 to 0.55, 0.1 deep
        assert_eq!(l.parts, 2);
    }

    #[test]
    fn volumes_of_the_other_primitives() {
        let c = lumped_body(&[add("c", "cylinder", &[100.0, 200.0, 100.0]), place("c", 0.0, 0.0), density("c", 1000.0)]).unwrap();
        assert!((c.mass_kg - 1000.0 * std::f64::consts::PI * 0.05_f64.powi(2) * 0.2).abs() < 1e-9);
        let s = lumped_body(&[add("s", "sphere", &[100.0]), place("s", 0.0, 0.0), density("s", 1000.0)]).unwrap();
        assert!((s.mass_kg - 1000.0 * (4.0 / 3.0) * std::f64::consts::PI * 0.05_f64.powi(3)).abs() < 1e-9);
    }

    #[test]
    fn a_missing_density_is_named_never_invented() {
        let ops = [add("a", "box", &[100.0; 3]), place("a", 0.0, 0.0), density("a", 1000.0), add("b", "box", &[100.0; 3]), place("b", 0.5, 0.0)];
        assert_eq!(lumped_body(&ops), Err(vec![Missing { part: "b (b)".into(), reason: "no density stated".into() }]));
        assert!(lumped_body(&[]).is_err());
    }

    #[test]
    fn refuses_unknown_shapes_wrong_units_and_imports() {
        assert!(lumped_body(&[add("t", "torus", &[1.0; 3]), place("t", 0.0, 0.0), density("t", 1.0)]).unwrap_err()[0].reason.contains("unknown shape"));
        let mut bad_unit = density("a", 1000.0);
        if let ModelOp::PartSetParam { value, .. } = &mut bad_unit { value.unit = "g/cm3".into(); }
        assert!(lumped_body(&[add("a", "box", &[1.0; 3]), place("a", 0.0, 0.0), bad_unit]).unwrap_err()[0].reason.contains("kg/m3"));
        let imp = ModelOp::PartAdd { id: "i".into(), name: "i".into(), geometry: Geometry::Import { asset_id: "x".into() }, maturity: Maturity::UserDefined, prov: Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } };
        assert!(lumped_body(&[imp]).unwrap_err()[0].reason.contains("imported"));
    }

    #[test]
    fn a_removed_part_no_longer_counts() {
        let ops = [add("a", "box", &[100.0; 3]), place("a", 0.0, 0.0), add("b", "box", &[100.0; 3]), place("b", 1.0, 0.0), ModelOp::PartRemove { id: "b".into() }, density("a", 500.0)];
        assert!((lumped_body(&ops).unwrap().mass_kg - 0.5).abs() < 1e-9);
    }
}
