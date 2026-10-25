"""Battery reference scenario for Phase 3: signals and declared SPM relations exported for helqix-investigate."""
import unittest

from helqix_battery import causal, model


class CausalExportTests(unittest.TestCase):
    def setUp(self):
        self.doc = causal.reference_input(current_a=5.0, seconds=3600, step_s=10, initial_ocv_v=4.18, config=model.ModelConfig())

    def test_matches_the_rust_input_schema(self):
        d = self.doc
        self.assertEqual(d["schema"], "helqix.investigation-input.v1")
        ids = {n["id"] for n in d["nodes"]}
        self.assertTrue({"current", "voltage", "neg_surface_stoichiometry", "pos_surface_stoichiometry"} <= ids)
        for n in d["nodes"]:
            self.assertEqual(len(n["values"]), len(d["times_seconds"]))
        for r in d["relations"]:
            self.assertIn(r["cause"], ids)
            self.assertIn(r["effect"], ids)

    def test_requirement_is_the_test_protocol_cutoff_not_invented(self):
        q = self.doc["requirement"]
        self.assertEqual((q["node"], q["comparator"], q["threshold"]), ("voltage", "at_least", 2.5))
        self.assertLess(min(n for n in next(x for x in self.doc["nodes"] if x["id"] == "voltage")["values"]), 2.5)

    def test_relations_come_from_the_model_structure_not_from_the_expected_answer(self):
        relations = {(r["cause"], r["effect"]) for r in self.doc["relations"]}
        # Every input to the terminal voltage of the SPM is declared, not only the one that will matter.
        for cause in ("neg_open_circuit_potential", "pos_open_circuit_potential", "neg_reaction_overpotential", "pos_reaction_overpotential"):
            self.assertIn((cause, "voltage"), relations)


if __name__ == "__main__":
    unittest.main()
