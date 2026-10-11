import unittest
import os
import json
from unittest.mock import patch

from helqix_intelligence import ai_review_provider_status, parse_drone_intent, parse_engineering_intent
from helqix_intelligence.intent import _provider_request, _validate_ai_review


class DroneIntentParserTests(unittest.TestCase):
    def test_extracts_only_explicit_values_with_source_spans(self):
        text = "Design an agricultural quadrotor for a 2 kg payload, 30 minute flight, and 12 m/s wind."
        result = parse_drone_intent(text)
        values = {item["key"]: item for item in result["candidates"]}
        self.assertEqual(values["payload_mass_kg"]["value"], 2.0)
        self.assertEqual(values["target_duration_min"]["value"], 30.0)
        self.assertEqual(values["wind_speed_m_s"]["value"], 12.0)
        self.assertEqual(values["mission_type"]["value"], "agriculture")
        for candidate in result["candidates"]:
            self.assertEqual(text[candidate["start"]:candidate["end"]], candidate["source_text"])
            self.assertEqual(candidate["review_status"], "needs_review")
        self.assertTrue(result["review_required"])

    def test_does_not_confuse_battery_mass_with_payload(self):
        result = parse_drone_intent("Compare a 2 kg battery with a 5 kg battery.")
        self.assertEqual(result["conflicts"], ["battery_mass_kg"])
        self.assertFalse(any(item["key"] == "payload_mass_kg" for item in result["candidates"]))
        self.assertEqual({item["value"] for item in result["candidates"]}, {2.0, 5.0})
        self.assertTrue(all(item["review_status"] == "conflict_review_required" for item in result["candidates"]))

    def test_reports_no_unsupported_performance_inference(self):
        result = parse_drone_intent("Build a drone for crop monitoring.")
        self.assertEqual([item["key"] for item in result["candidates"]], ["mission_type"])
        self.assertIn("flight time", result["not_inferred"])

    def test_rejects_empty_and_overlong_intent(self):
        with self.assertRaises(ValueError):
            parse_drone_intent("   ")
        with self.assertRaises(ValueError):
            parse_drone_intent("x" * 5001)


class EngineeringIntentParserTests(unittest.TestCase):
    def test_omitted_domain_preserves_legacy_drone_api_contract(self):
        result = parse_engineering_intent("A 2 kg payload for a 30 minute drone mission.")
        self.assertEqual(result["parser_version"], "drone-intent-explicit-v1")
        self.assertIn("payload_mass_kg", {item["key"] for item in result["candidates"]})

    def test_extracts_only_explicit_vehicle_values_and_preserves_spans(self):
        text = "Electric vehicle: vehicle mass 1800 kg, battery capacity 64 kWh, target range 420 km, top speed 160 km/h."
        result = parse_engineering_intent(text, "battery electric vehicle")
        self.assertEqual(result["parser_version"], "vehicle-intent-explicit-v1")
        values = {item["key"]: item for item in result["candidates"]}
        self.assertEqual(values["vehicle_mass_kg"]["value"], 1800.0)
        self.assertEqual(values["battery_capacity_kwh"]["value"], 64.0)
        self.assertEqual(values["target_range_km"]["value"], 420.0)
        self.assertEqual(values["target_speed_kmh"]["value"], 160.0)
        for candidate in result["candidates"]:
            self.assertEqual(text[candidate["start"]:candidate["end"]], candidate["source_text"])
            self.assertEqual(candidate["review_status"], "needs_review")
        self.assertIn("range from battery capacity", result["not_inferred"])

    def test_preserves_the_exact_submitted_source_for_provenance_spans(self):
        text = "\n  Electric vehicle: target range 420 km.  \n"
        result = parse_engineering_intent(text, "vehicle")
        self.assertEqual(result["source_text"], text)
        candidate = result["candidates"][0]
        self.assertEqual(text[candidate["start"]:candidate["end"]], candidate["source_text"])
        self.assertGreater(candidate["start"], 0)

    def test_vehicle_conflicts_are_retained_for_human_review(self):
        result = parse_engineering_intent("Target range 300 km; target range 420 km.", "vehicle")
        self.assertEqual(result["conflicts"], ["target_range_km"])
        self.assertEqual({item["value"] for item in result["candidates"]}, {300.0, 420.0})
        self.assertTrue(all(item["review_status"] == "conflict_review_required" for item in result["candidates"]))

    def test_extracts_explicit_battery_cell_and_pack_values_without_inference(self):
        text = (
            "Battery pack capacity 12 kWh, cell capacity 4 Ah, nominal voltage 48 V, "
            "discharge current 2 A, and cutoff voltage 3.0 V."
        )
        result = parse_engineering_intent(text, "battery")
        values = {item["key"]: item for item in result["candidates"]}
        self.assertEqual(result["parser_version"], "battery-intent-explicit-v1")
        self.assertEqual(values["battery_energy_capacity_kwh"]["value"], 12.0)
        self.assertEqual(values["battery_cell_capacity_ah"]["value"], 4.0)
        self.assertEqual(values["battery_nominal_voltage_v"]["value"], 48.0)
        self.assertEqual(values["battery_discharge_current_a"]["value"], 2.0)
        self.assertEqual(values["battery_cutoff_voltage_v"]["value"], 3.0)
        for candidate in result["candidates"]:
            self.assertEqual(text[candidate["start"]:candidate["end"]], candidate["source_text"])
            self.assertEqual(candidate["review_status"], "needs_review")
        self.assertIn("cell chemistry", result["not_inferred"])
        self.assertTrue(result["review_required"])

    def test_battery_conflicts_remain_explicit_and_are_not_resolved(self):
        result = parse_engineering_intent(
            "Battery capacity 10 kWh; battery capacity 12 kWh.", "battery"
        )
        self.assertEqual(result["conflicts"], ["battery_energy_capacity_kwh"])
        self.assertEqual(
            {item["value"] for item in result["candidates"]}, {10.0, 12.0}
        )
        self.assertTrue(all(
            item["review_status"] == "conflict_review_required"
            for item in result["candidates"]
        ))

    def test_keeps_cell_mah_distinct_and_extracts_stated_pack_mass(self):
        result = parse_engineering_intent(
            "Battery pack mass 200 kg; cell capacity 5000 mAh.", "energy storage"
        )
        values = {item["key"]: item for item in result["candidates"]}
        self.assertEqual(values["battery_mass_kg"]["value"], 200.0)
        self.assertEqual(values["battery_cell_capacity_mah"]["value"], 5000.0)
        self.assertNotIn("battery_cell_capacity_ah", values)

    def test_unknown_domain_preserves_source_without_extracting_values(self):
        text = "Industrial press: 12 tonnes, 800 mm stroke, 4 second cycle."
        result = parse_engineering_intent(text, "industrial press")
        self.assertEqual(result["source_text"], text)
        self.assertEqual(result["candidates"], [])
        self.assertEqual(result["method"], "source_preservation_no_domain_rules")

    def test_domain_router_does_not_misclassify_device_as_ev(self):
        result = parse_engineering_intent("A sensor device weighs 2 kg.", "device")
        self.assertEqual(result["parser_version"], "engineering-intent-source-only-v1")
        self.assertEqual(result["candidates"], [])


class AIReviewValidationTests(unittest.TestCase):
    def test_provider_status_does_not_expose_secret_and_requires_explicit_configuration(self):
        with patch.dict(os.environ, {"HELQIX_INTENT_AI_PROVIDER": "openai", "OPENAI_API_KEY": "secret", "OPENAI_MODEL": "model-x"}, clear=False):
            status = ai_review_provider_status()
        self.assertTrue(status["available"])
        self.assertEqual(status["provider"], "openai")
        self.assertEqual(status["model"], "model-x")
        self.assertNotIn("secret", repr(status))

    def test_validates_exact_source_quotes_and_keeps_proposals_unaccepted(self):
        text = "A vehicle should carry 100 kg payload."
        result = _validate_ai_review({
            "objectives": [{"statement": "Carry a 100 kg payload", "source_quote": "carry 100 kg payload"}],
            "questions": [{"question": "What range is required?", "reason": "No range target was stated."}],
        }, text)
        self.assertEqual(result["objectives"][0]["review_status"], "needs_review")
        self.assertEqual(result["objectives"][0]["source_quote"], "carry 100 kg payload")
        self.assertTrue(result["review_required"])

    def test_rejects_objective_with_quote_not_present_verbatim(self):
        with self.assertRaises(RuntimeError):
            _validate_ai_review({"objectives": [{"statement": "Carry a payload", "source_quote": "payload of 100 kg"}], "questions": []}, "Carry 100 kg.")

    def test_rejects_unexpected_response_fields(self):
        with self.assertRaises(RuntimeError):
            _validate_ai_review({"objectives": [], "questions": [], "recommendation": "buy a motor"}, "Build a vehicle.")

    def test_provider_adapters_use_expected_protocol_shapes_without_live_network(self):
        class Response:
            def __init__(self, body): self.body = body
            def __enter__(self): return self
            def __exit__(self, *args): return None
            def read(self, limit): return self.body[:limit]

        expected = {"objectives": [{"statement": "carry a load", "source_quote": "carry a load"}], "questions": []}
        for provider in ("openai", "deepseek", "kimi", "qwen", "glm"):
            envelope = {"choices": [{"message": {"content": json.dumps(expected)}}]}
            with self.subTest(provider=provider), patch("helqix_intelligence.intent.urllib.request.urlopen", return_value=Response(json.dumps(envelope).encode())) as send:
                result = _provider_request(provider, "model-name", "test-key", "prompt", "carry a load")
                request = send.call_args.args[0]
                self.assertEqual(json.loads(request.data)["model"], "model-name")
                self.assertEqual(result["objectives"][0]["review_status"], "needs_review")

        anthropic = {"content": [{"type": "tool_use", "name": "brief_review", "input": expected}]}
        with patch("helqix_intelligence.intent.urllib.request.urlopen", return_value=Response(json.dumps(anthropic).encode())) as send:
            _provider_request("anthropic", "model-name", "test-key", "prompt", "carry a load")
            self.assertEqual(send.call_args.args[0].full_url, "https://api.anthropic.com/v1/messages")

        gemini = {"candidates": [{"content": {"parts": [{"text": json.dumps(expected)}]}}]}
        with patch("helqix_intelligence.intent.urllib.request.urlopen", return_value=Response(json.dumps(gemini).encode())) as send:
            _provider_request("gemini", "model-name", "test-key", "prompt", "carry a load")
            body = json.loads(send.call_args.args[0].data)
            self.assertIn("responseJsonSchema", body["generationConfig"])

if __name__ == "__main__":
    unittest.main()
