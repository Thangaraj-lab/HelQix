//! Gate 4 demonstration on the core runtime: freeze a falling body before impact, test candidate
//! causes with single interventions, print Compare output and a dose-response check.
//! Usage: `helqix-branch-demo`. Exit 0 if every verdict matches the physics stated below.
use helqix_branch::intervention::{compare_runs, dose_response, intervention_test, Budget, Experiments, Intervention, Parameter, Verdict};
use helqix_core::id::Identifier;
use helqix_freeze::{freeze, run, Comparator, FreezeTrigger, PointMassSpec, RunSpec, StateLimit};
use std::process::ExitCode;

fn main() -> ExitCode {
    let spec = RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 4.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    };
    let limit = StateLimit { name: "stay above ground".into(), variable: 1, comparator: Comparator::AtLeast, threshold: 0.0 };
    let parent = run(&spec).expect("parent run");
    let frozen = freeze(&spec, &parent, 500, FreezeTrigger::TimeMarker(0.5)).expect("freeze");
    println!("parent: freeze at index 500 (t = 0.5 s), sha256 {}", frozen.sha256());
    let mut ex = Experiments::new(Budget { max_branches: 10, max_total_steps: 100_000 });
    let id = |s: &str| Identifier::parse(s).unwrap();
    let mut ok = true;
    // (label, intervention, expected verdict and why)
    let cases = [
        ("stop_vertical_velocity", Intervention::State { variable: 4, value: 0.0 }, Verdict::InterventionSupported, "candidate cause from Phase 3 (velocity_y -> height): stopping the fall delays impact"),
        ("move_out_of_plane", Intervention::State { variable: 2, value: 5.0 }, Verdict::NotSupported, "not a cause: z position does not enter the vertical motion"),
        ("more_drag", Intervention::Parameter { parameter: Parameter::DragCoefficient, value: 1.2 }, Verdict::InterventionSupported, "drag opposes the fall"),
    ];
    for (label, intervention, expected, why) in cases {
        let r = intervention_test(&mut ex, &parent, &frozen, &limit, id(label), intervention, 0.005).expect("branch");
        let matches = r.verdict == expected;
        ok &= matches;
        println!("{label}: parent impact {:?} s, branch impact {:?} s -> {:?} [{}] ({}){}", r.parent_failure_seconds, r.branch_failure_seconds, r.verdict, r.grade, why, if matches { "" } else { "  UNEXPECTED" });
        let branch = &ex.branches().last().unwrap().run;
        let c = compare_runs(&parent, branch, &[(1, "height".into()), (4, "velocity_y".into())], &limit);
        println!("  compare: {} aligned, first divergence {:?} s, max |dheight| {:.4} m, parent {:?}, branch {:?}", c.aligned_samples, c.first_divergence_seconds, c.max_abs_delta[0].1, c.parent_status, c.branch_status);
    }
    let dose = dose_response(&mut ex, &frozen, &limit, "drag_dose", &[0.6, 1.0, 1.4].map(|v| Intervention::Parameter { parameter: Parameter::DragCoefficient, value: v })).expect("dose");
    println!("dose-response (drag 0.6, 1.0, 1.4): impact {:?} s, monotonic {}", dose.failure_times_seconds, dose.monotonic);
    ok &= dose.monotonic;
    println!("{}", if ok { "PASS: verdicts match the stated physics (grade model-causal; not validated-causal)" } else { "FAIL" });
    if ok { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
