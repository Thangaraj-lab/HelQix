"""Tests that need PyBaMM. Synthetic inputs only: no measured data, no network."""
import unittest

from helqix_battery import calibrate, cases, integrity, model, split


def constant_current_case(case_id, rate, current_a, seconds=1800, step=30, rest=600, v0=4.18):
    n = seconds // step
    time_s = [float(k * step) for k in range(n + 1)] + [float(seconds + k * step) for k in range(1, rest // step + 1)]
    current = [current_a] * (n + 1) + [0.0] * (rest // step)
    return cases.Case(id=case_id, cell=case_id.split("_")[0], rate=rate, time_s=time_s, current_a=current,
                      voltage_v=[0.0] * len(time_s), discharge_samples=n + 1, initial_rest_voltage_v=v0)


class SimulationTests(unittest.TestCase):
    def test_output_has_one_finite_voltage_per_input_time(self):
        c = constant_current_case("cellx_1c", "1c", 5.0)
        v = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, model.ModelConfig())
        self.assertEqual(len(v), len(c.time_s))
        self.assertTrue(all(2.0 < x < 4.3 for x in v))

    def test_series_resistance_shifts_voltage_by_exactly_i_times_r(self):
        c = constant_current_case("cellx_1c", "1c", 5.0, seconds=600, rest=0)
        base = model.simulate(c.time_s, c.current_a, 4.18, model.ModelConfig(contact_resistance_ohm=0.0))
        shifted = model.simulate(c.time_s, c.current_a, 4.18, model.ModelConfig(contact_resistance_ohm=0.02))
        for b, s in zip(base, shifted):
            self.assertAlmostEqual(b - s, 5.0 * 0.02, delta=1e-5)

    def test_rest_only_holds_the_initial_open_circuit_voltage(self):
        c = constant_current_case("cellx_c10", "c10", 0.0, seconds=600, rest=600, v0=3.9)
        v = model.simulate(c.time_s, c.current_a, 3.9, model.ModelConfig())
        self.assertLess(max(abs(x - 3.9) for x in v), 1e-4)

    def test_identity_is_stable_and_changes_with_any_setting(self):
        a = model.identity(model.ModelConfig(contact_resistance_ohm=0.01))
        self.assertEqual(a["sha256"], model.identity(model.ModelConfig(contact_resistance_ohm=0.01))["sha256"])
        self.assertNotEqual(a["sha256"], model.identity(model.ModelConfig(contact_resistance_ohm=0.011))["sha256"])
        self.assertNotEqual(a["sha256"], model.identity(model.ModelConfig(contact_resistance_ohm=0.01, rtol=1e-8))["sha256"])
        self.assertEqual(a["parameter_set"], "Chen2020")

    def test_incomplete_simulation_is_an_error_not_a_short_result(self):
        c = constant_current_case("cellx_1c5", "1c5", 50.0, seconds=7200, rest=0)
        with self.assertRaises(model.SimulationIncomplete):
            model.simulate(c.time_s, c.current_a, 4.18, model.ModelConfig())


class CalibrationTests(unittest.TestCase):
    def _synthetic_split(self, true_r):
        made = []
        for cell in ("cella", "cellb"):
            for rate, current in (("c2", 2.5), ("1c", 5.0), ("c10", 0.5)):
                c = constant_current_case(f"{cell}_{rate}", rate, current, seconds=900, rest=300)
                c.voltage_v = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, model.ModelConfig(contact_resistance_ohm=true_r))
                made.append(c)
        return split.make_split(made)

    def test_recovers_a_known_resistance_from_calibration_cases_only(self):
        s = self._synthetic_split(0.018)
        result = calibrate.calibrate(s.calibration_set(), model.ModelConfig())
        self.assertAlmostEqual(result.contact_resistance_ohm, 0.018, delta=2e-4)
        self.assertEqual(sorted(result.fitted_on), sorted(c.id for c in s.calibration))
        self.assertFalse(any(cid.endswith("_c10") for cid in result.fitted_on))
        self.assertFalse(result.at_bound)

    def test_refuses_anything_but_a_calibration_set(self):
        s = self._synthetic_split(0.01)
        with self.assertRaises(split.SplitViolation):
            calibrate.calibrate(list(s.validation), model.ModelConfig())


class SensitivityTests(unittest.TestCase):
    def test_one_at_a_time_on_calibration_data_only(self):
        made = []
        for rate, current in (("1c", 5.0), ("c10", 0.5)):
            c = constant_current_case(f"cella_{rate}", rate, current, seconds=900, rest=300)
            c.voltage_v = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, model.ModelConfig(contact_resistance_ohm=0.01))
            made.append(c)
        s = split.make_split(made)
        from helqix_battery import sensitivity
        result = sensitivity.one_at_a_time(s.calibration_set(), model.ModelConfig(contact_resistance_ohm=0.01), step=0.10)
        names = {e["parameter"] for e in result["entries"]}
        self.assertIn("Contact resistance [Ohm]", names)
        self.assertEqual(len(result["entries"]), 2 * len(names))
        resistance_up = next(e for e in result["entries"] if e["parameter"] == "Contact resistance [Ohm]" and e["factor"] > 1)
        self.assertAlmostEqual(resistance_up["max_abs_dv_v"], 5.0 * 0.01 * 0.10, delta=1e-5)  # closed form: I * dR
        self.assertEqual(result["cases"], ["cella_1c"])
        with self.assertRaises(split.SplitViolation):
            sensitivity.one_at_a_time(list(s.validation), model.ModelConfig(), step=0.10)


class IntegrityTests(unittest.TestCase):
    def test_reports_tolerance_and_mesh_refinement_and_passes_for_a_settled_solution(self):
        c = constant_current_case("cellx_c10", "c10", 0.5, seconds=1800, rest=300)
        c.voltage_v = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, model.ModelConfig())
        report = integrity.check([c], model.ModelConfig(), threshold_v=0.046)
        self.assertTrue(report["passed"], report)
        entry = report["cases"][0]
        for key in ("rmse_baseline_v", "rmse_tight_tolerance_v", "rmse_refined_mesh_v", "max_abs_dv_tight_v", "max_abs_dv_refined_v"):
            self.assertIn(key, entry)

    def test_fails_when_refinement_flips_the_verdict(self):
        c = constant_current_case("cellx_1c", "1c", 5.0, seconds=1800, rest=300)
        sim = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, model.ModelConfig())
        c.voltage_v = [v - 0.03 for v in sim]  # baseline RMSE is about 30 mV
        probe = integrity.check([c], model.ModelConfig(), threshold_v=1.0)["cases"][0]
        baseline, refined = probe["rmse_baseline_v"], probe["rmse_refined_mesh_v"]
        self.assertNotEqual(baseline, refined, "mesh refinement changed nothing; the check is not refining")
        threshold = (baseline + refined) / 2.0  # one of the two lies on each side
        self.assertFalse(integrity.check([c], model.ModelConfig(), threshold_v=threshold)["passed"])

    def test_incomplete_refined_run_fails_integrity(self):
        c = constant_current_case("cellx_1c5", "1c5", 50.0, seconds=7200, rest=0)
        report = integrity.check([c], model.ModelConfig(), threshold_v=0.046)
        self.assertFalse(report["passed"])
        self.assertIn("error", report["cases"][0])


if __name__ == "__main__":
    unittest.main()
