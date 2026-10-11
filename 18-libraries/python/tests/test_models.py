import unittest

from pydantic import ValidationError

from helqix_drone_domain import DroneScenario, DroneSpecification, EngineeringReality, validate_resource_payload


SPEC = {
    "schema": "helqix.drone-specification.v1", "aircraft": "quadrotor",
    "frameDiagonalMm": 650, "dryMassKg": 2.4, "payloadMassKg": 2,
    "batteryMassKg": 2, "batteryCapacityMah": 16000, "batteryVoltageV": 22.2,
    "propellerDiameterIn": 15, "motorKvRpmPerVolt": 320,
}


class DroneDomainContractTests(unittest.TestCase):
    def test_generic_engineering_reality_accepts_non_drone_domain_and_resolves_links(self):
        reality = EngineeringReality.model_validate({
            "schema": "helqix.engineering-reality.v1",
            "name": "Pump station", "domain": "industrial.pump",
            "components": [
                {"id": "pump", "name": "Main pump", "component_type": "centrifugal_pump",
                 "properties": [{"name": "rated_power", "value": 2.2, "unit": "kW", "origin": "catalog", "source_ref": "catalog:pump-22"}]},
                {"id": "controller", "name": "Drive controller", "component_type": "motor_controller"},
            ],
            "interfaces": [
                {"id": "pump.power", "component_id": "pump", "name": "Power input", "direction": "in", "domain": "electrical"},
                {"id": "drive.output", "component_id": "controller", "name": "Drive output", "direction": "out", "domain": "electrical"},
            ],
            "connections": [{"id": "drive-to-pump", "source_interface_id": "drive.output", "target_interface_id": "pump.power", "connection_type": "electrical_power"}],
            "requirements": [{"id": "flow-rate", "statement": "Deliver required process flow", "priority": "must", "review_status": "draft"}],
            "model_bindings": [{"id": "pump-model", "model_id": "pump.curve", "model_version": "1.0", "scope_component_id": "pump", "declared_fidelity": "vendor curve"}],
        })
        payload = validate_resource_payload("projects", {"engineering_reality": reality.model_dump(by_alias=True)})
        self.assertEqual(payload["engineering_reality"]["domain"], "industrial.pump")
        self.assertEqual(len(reality.connections), 1)

    def test_generic_engineering_reality_rejects_dangling_links_and_unknown_fields(self):
        invalid = {
            "schema": "helqix.engineering-reality.v1", "name": "System", "domain": "generic",
            "components": [{"id": "unit-a", "name": "Unit A", "component_type": "unit"}],
            "interfaces": [{"id": "port-a", "component_id": "missing", "name": "Port", "direction": "out", "domain": "signal"}],
            "made_up_result": 123,
        }
        with self.assertRaises(ValidationError):
            EngineeringReality.model_validate(invalid)

    def test_generic_engineering_reality_rejects_duplicate_ids_and_incompatible_port_direction(self):
        invalid = {
            "schema": "helqix.engineering-reality.v1", "name": "System", "domain": "generic",
            "components": [
                {"id": "unit-a", "name": "Unit A", "component_type": "unit"},
                {"id": "unit-b", "name": "Unit B", "component_type": "unit"},
            ],
            "interfaces": [
                {"id": "in-a", "component_id": "unit-a", "name": "Input", "direction": "in", "domain": "signal"},
                {"id": "out-b", "component_id": "unit-b", "name": "Output", "direction": "out", "domain": "signal"},
            ],
            "connections": [{"id": "duplicate", "source_interface_id": "in-a", "target_interface_id": "out-b", "connection_type": "signal"}],
            "requirements": [{"id": "duplicate", "statement": "A requirement", "priority": "must"}],
        }
        with self.assertRaises(ValidationError):
            EngineeringReality.model_validate(invalid)
        invalid["requirements"][0]["id"] = "requirement-a"
        with self.assertRaises(ValidationError):
            EngineeringReality.model_validate(invalid)

    def test_connections_require_matching_interface_domains(self):
        reality = {
            "schema": "helqix.engineering-reality.v1", "name": "Power path", "domain": "generic",
            "components": [
                {"id": "source", "name": "Source", "component_type": "power_source"},
                {"id": "load", "name": "Load", "component_type": "load"},
            ],
            "interfaces": [
                {"id": "source.electrical", "component_id": "source", "name": "Power out", "direction": "out", "domain": "electrical_power"},
                {"id": "load.fluid", "component_id": "load", "name": "Fluid in", "direction": "in", "domain": "fluid"},
            ],
            "connections": [{"id": "source-to-load", "source_interface_id": "source.electrical", "target_interface_id": "load.fluid", "connection_type": "direct"}],
        }
        with self.assertRaisesRegex(ValidationError, "incompatible interface domains"):
            EngineeringReality.model_validate(reality)

    def test_explicit_verification_rule_requires_supported_metric_and_matching_unit(self):
        base = {
            "schema": "helqix.engineering-reality.v1", "name": "Cell test", "domain": "energy.battery",
            "requirements": [{"id": "duration", "statement": "At least one hour", "priority": "must",
                              "review_status": "accepted", "verification_rule": {
                                  "metric": "duration_h", "operator": ">=", "threshold": 1, "unit": "h"}}],
        }
        reality = EngineeringReality.model_validate(base)
        self.assertEqual(reality.requirements[0].verification_rule.threshold, 1)
        for rule in (
            {"metric": "duration_h", "operator": ">=", "threshold": 1, "unit": "V"},
            {"metric": "unknown_metric", "operator": ">=", "threshold": 1, "unit": "h"},
            {"metric": "duration_h", "operator": ">=", "threshold": float("inf"), "unit": "h"},
        ):
            with self.subTest(rule=rule), self.assertRaises(ValidationError):
                EngineeringReality.model_validate({**base, "requirements": [{**base["requirements"][0], "verification_rule": rule}]})

    def test_bounded_search_requires_real_candidate_outputs_and_consistent_ranking(self):
        candidates = [
            {"current_a": 1.0, "state": "completed", "run_id": "run-1", "duration_h": 3.0,
             "discharged_capacity_ah": 2.9, "final_voltage_v": 3.0},
            {"current_a": 2.0, "state": "completed", "run_id": "run-2", "duration_h": 1.4,
             "discharged_capacity_ah": 2.7, "final_voltage_v": 3.0},
        ]
        search = {
            "schema": "helqix.bounded-search.v1", "name": "Cell current sweep", "status": "completed",
            "base_reality_revision": 1, "binding_id": "cell-spm",
            "variable": {"component_id": "cell", "property_name": "current_a", "unit": "A", "values": [1.0, 2.0]},
            "objective": {"metric": "discharged_capacity_ah", "direction": "maximize"},
            "constraint": {"metric": "duration_h", "operator": ">=", "value_h": 2.0},
            "candidate_count": 2, "completed_count": 2, "best_evaluated_candidate_a": 1.0,
            "candidates": candidates, "applicability": "Finite single-cell sweep only.",
        }
        saved = validate_resource_payload("searches", search)
        self.assertEqual(saved["best_evaluated_candidate_a"], 1.0)
        invalid_ranking = {**search, "best_evaluated_candidate_a": 2.0}
        with self.assertRaises(ValidationError):
            validate_resource_payload("searches", invalid_ranking)
        invalid_result = {**search, "candidates": [{**candidates[0], "run_id": None}, candidates[1]]}
        with self.assertRaises(ValidationError):
            validate_resource_payload("searches", invalid_result)

    def test_verification_record_binds_verdict_to_metric_rule_and_model_digest(self):
        digest = "a" * 64
        check = {
            "schema": "helqix.verification-check.v1", "requirement_id": "capacity", "code": "capacity",
            "requirementTitle": "At least 2 Ah", "governingStandard": "User-defined accepted criterion",
            "conditionFormula": "discharged_capacity_ah >= 2 Ah", "metric": "discharged_capacity_ah",
            "operator": ">=", "threshold": 2.0, "unit": "Ah", "observed_value": 3.0,
            "observedValue": "3 Ah", "limitThreshold": ">= 2 Ah", "marginPercent": 50.0,
            "verdict": "pass", "method": "explicit threshold", "source_run_id": "run-1",
            "source_model_sha256": digest, "current_model_sha256": digest,
        }
        saved = validate_resource_payload("verifications", check)
        self.assertEqual(saved["verdict"], "pass")
        with self.assertRaises(ValidationError):
            validate_resource_payload("verifications", {**check, "verdict": "fail"})
        with self.assertRaises(ValidationError):
            validate_resource_payload("verifications", {**check, "current_model_sha256": "b" * 64})

    def test_accepts_versioned_draft_with_ui_camel_case(self):
        model = DroneSpecification.model_validate(SPEC)
        self.assertEqual(model.schema_version, "helqix.drone-specification.v1")
        self.assertEqual(model.battery_mass_kg, 2)

    def test_rejects_out_of_range_and_unknown_design_inputs(self):
        invalid = {**SPEC, "batteryMassKg": 0, "unsupportedThrustEstimate": 999}
        with self.assertRaises(ValidationError):
            DroneSpecification.model_validate(invalid)

    def test_validates_project_and_scenario_payloads(self):
        project = validate_resource_payload("projects", {"intent": "Agri drone", "specification": SPEC})
        self.assertEqual(project["specification"]["schema"], "helqix.drone-specification.v1")
        scenario = DroneScenario.model_validate({
            "schema": "helqix.drone-scenario.v1", "name": "Five kg battery",
            "description": "Configuration only", "base_specification": SPEC,
            "parameters": {"battery_mass_kg": 5, "payload_mass_kg": 2, "wind_speed_m_s": 12},
            "execution_state": "not_run",
        })
        self.assertEqual(scenario.parameters.battery_mass_kg, 5)
        self.assertEqual(scenario.execution_state, "not_run")

    def test_scenario_can_store_explicit_pack_rating_without_inferring_from_mass(self):
        scenario = DroneScenario.model_validate({
            "schema": "helqix.drone-scenario.v1", "name": "Five kg pack",
            "description": "Configuration only", "base_specification": SPEC,
            "parameters": {"battery_mass_kg": 5, "battery_capacity_mah": 24000,
                            "battery_voltage_v": 44.4, "payload_mass_kg": 2, "wind_speed_m_s": 12},
            "execution_state": "not_run",
        })
        self.assertEqual(scenario.parameters.battery_capacity_mah, 24000)
        self.assertEqual(scenario.parameters.battery_voltage_v, 44.4)
        self.assertEqual(scenario.execution_state, "not_run")
        with self.assertRaises(ValidationError):
            DroneScenario.model_validate({
                "schema": "helqix.drone-scenario.v1", "name": "Invalid pack",
                "description": "", "base_specification": SPEC,
                "parameters": {"battery_mass_kg": 5, "battery_capacity_mah": 0,
                                "payload_mass_kg": 2, "wind_speed_m_s": 12},
                "execution_state": "not_run",
            })

    def test_upgrades_valid_legacy_specification_without_changing_its_values(self):
        legacy = {key: value for key, value in SPEC.items() if key != "schema"}
        result = validate_resource_payload("projects", {"intent": "saved design", "specification": legacy})
        self.assertEqual(result["specification"]["schema"], "helqix.drone-specification.v1")
        self.assertEqual(result["specification"]["batteryMassKg"], legacy["batteryMassKg"])

    def test_rejects_non_finite_scenario_values(self):
        with self.assertRaises(ValidationError):
            DroneScenario.model_validate({
                "schema": "helqix.drone-scenario.v1", "name": "Invalid",
                "description": "", "base_specification": SPEC,
                "parameters": {"battery_mass_kg": float("nan"), "payload_mass_kg": 2, "wind_speed_m_s": 12},
                "execution_state": "not_run",
            })


if __name__ == "__main__":
    unittest.main()
