import unittest

from helqix_battery import cases, report, split, uncertainty


def case(cid, rate, offset=0.0):
    t = [0.0, 10.0, 20.0, 30.0]
    return cases.Case(id=cid, cell=cid.split("_")[0], rate=rate, time_s=t, current_a=[0.5, 0.5, 0.0, 0.0],
                      voltage_v=[4.1, 4.0, 3.95, 3.97], discharge_samples=2, initial_rest_voltage_v=4.18)


class UncertaintyTests(unittest.TestCase):
    def test_summarises_held_out_errors_without_assuming_a_distribution(self):
        held = {"cella_c10": ([4.11, 4.01, 3.96, 3.98], [4.1, 4.0, 3.95, 3.97]), "cellb_c10": ([4.13, 4.03, 3.98, 4.00], [4.1, 4.0, 3.95, 3.97])}
        u = uncertainty.summarise(held)
        self.assertAlmostEqual(u["per_case"]["cella_c10"]["rmse_v"], 0.01, places=12)
        self.assertAlmostEqual(u["per_case"]["cellb_c10"]["p95_abs_error_v"], 0.03, places=12)
        self.assertAlmostEqual(u["across_cases"]["rmse_min_v"], 0.01, places=12)
        self.assertAlmostEqual(u["across_cases"]["rmse_max_v"], 0.03, places=12)
        self.assertEqual(u["across_cases"]["n_cases"], 2)
        self.assertIn("nearest-rank", u["method"])

    def test_empty_input_is_rejected(self):
        with self.assertRaises(ValueError):
            uncertainty.summarise({})


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.cases = [case("cella_c10", "c10"), case("cella_1c", "1c")]
        self.split = split.make_split(self.cases)
        self.sim = {"cella_c10": [4.11, 4.01, 3.96, 3.98], "cella_1c": [4.0, 3.9, 3.9, 3.9]}

    def build(self, **overrides):
        args = dict(split_=self.split, simulated=self.sim, fitted_on=["cella_1c"], model_identity={"sha256": "c" * 64, "name": "m"},
                    dataset_files={"x.csv": {"md5": "0" * 32, "sha256": "1" * 64}}, integrity={"passed": True},
                    extras={"calibration": {"contact_resistance_ohm": 0.01}}, date="2026-10-25")
        args.update(overrides)
        return report.build(**args)

    def test_matches_the_rust_report_schema(self):
        r = self.build()
        self.assertEqual(r["schema"], "helqix.validation-report.v1")
        self.assertEqual(r["split"]["sha256"], self.split.sha256)
        self.assertEqual(r["split"]["fitted_on"], ["cella_1c"])
        self.assertEqual(r["requirement"]["threshold"], {"value": 46.0, "unit": "mV"})
        self.assertEqual(r["requirement"]["comparison"], "LessThan")
        self.assertEqual(r["context_of_use"]["risk"], "Low")
        self.assertEqual(r["output_unit"], "V")
        roles = {c["id"]: c["role"] for c in r["cases"]}
        self.assertEqual(roles, {"cella_c10": "validation", "cella_1c": "calibration"})
        c10 = next(c for c in r["cases"] if c["id"] == "cella_c10")
        self.assertEqual(c10["measured"], self.cases[0].voltage_v)
        self.assertEqual(c10["simulated"], self.sim["cella_c10"])
        self.assertEqual(c10["input"], self.cases[0].current_a)

    def test_values_carry_origin_and_status_labels_and_no_verdict(self):
        r = self.build()
        self.assertEqual(r["value_labels"]["measured"], {"origin": "Observed", "status": "Unverified"})
        self.assertEqual(r["value_labels"]["simulated"]["origin"], "Simulated")
        self.assertNotIn("label", r)
        self.assertNotIn("Verified", r["value_labels"]["simulated"]["status"].replace("Unverified", ""))

    def test_missing_simulation_for_a_case_is_an_error(self):
        with self.assertRaises(ValueError):
            self.build(simulated={"cella_c10": self.sim["cella_c10"]})

    def test_fitted_on_must_be_calibration_cases(self):
        with self.assertRaises(split.SplitViolation):
            self.build(fitted_on=["cella_c10"])


if __name__ == "__main__":
    unittest.main()
