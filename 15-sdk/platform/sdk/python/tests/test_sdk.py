"""Regression coverage for public HELQIX SDK contracts and wire codecs."""

from __future__ import annotations

import math
import unittest

from helqix import (
    ActionAccepted,
    ActionFailed,
    ActionFailure,
    ActionInvocation,
    ActionSucceeded,
    BehaviourDefinition,
    BehaviourRef,
    Component,
    ComponentKey,
    ComponentRef,
    RealityDocument,
    RealityMetadata,
    RealityValidationError,
    action_outcome_from_dict,
    action_outcome_to_dict,
)


class RealityContractTests(unittest.TestCase):
    def test_reality_round_trip_uses_camel_case_wire_schema(self) -> None:
        reality = RealityDocument(
            "reality-1",
            RealityMetadata("Demo", created_at="2026-10-08T12:00:00Z"),
            components=({"id": "transform", "type": "spatial", "data": {"x": 2}},),
        )
        payload = reality.to_dict()
        self.assertEqual(payload["schemaVersion"], 1)
        self.assertEqual(payload["metadata"]["createdAt"], "2026-10-08T12:00:00Z")
        self.assertEqual(RealityDocument.from_json(reality.to_json()), reality)

    def test_reality_detaches_and_freezes_nested_input(self) -> None:
        source = {"id": "component-1", "type": "transform", "data": {"position": [1, 2]}}
        reality = RealityDocument("reality-1", RealityMetadata("Demo"), components=(source,))
        source["data"]["position"][0] = 99
        self.assertEqual(reality.components[0]["data"]["position"][0], 1)
        with self.assertRaises(TypeError):
            reality.components[0]["data"]["x"] = 1

    def test_duplicate_component_ids_are_rejected(self) -> None:
        with self.assertRaises(RealityValidationError):
            RealityDocument("reality-1", RealityMetadata("Demo"), components=(
                {"id": "duplicate", "type": "a"},
                {"id": "duplicate", "type": "b"},
            ))

    def test_invalid_timestamp_and_non_finite_extension_are_rejected(self) -> None:
        with self.assertRaises(RealityValidationError):
            RealityMetadata("Demo", created_at="2026-02-30T12:00:00Z")
        with self.assertRaises(TypeError):
            RealityMetadata("Demo", extensions={"bad": math.nan})

    def test_unknown_component_reference_fields_are_rejected(self) -> None:
        with self.assertRaises(RealityValidationError):
            ComponentRef.from_dict({"id": "c1", "type": "transform", "unexpected": True})

    def test_reality_wire_decoder_rejects_unknown_fields_at_every_level(self) -> None:
        base = RealityDocument("reality-1", RealityMetadata("Demo"), components=(
            {"id": "c1", "type": "transform"},
        )).to_dict()
        for changed in (
            {**base, "unexpected": True},
            {**base, "metadata": {**base["metadata"], "unexpected": True}},
            {**base, "components": [{**base["components"][0], "unexpected": True}]},
        ):
            with self.subTest(changed=changed):
                with self.assertRaises(RealityValidationError):
                    RealityDocument.from_dict(changed)

    def test_reality_wire_decoder_requires_version_and_component_collection(self) -> None:
        base = RealityDocument("reality-1", RealityMetadata("Demo"), components=()).to_dict()
        for key in ("schemaVersion", "components"):
            changed = {name: value for name, value in base.items() if name != key}
            with self.subTest(missing=key):
                with self.assertRaises(RealityValidationError):
                    RealityDocument.from_dict(changed)


class ComponentContractTests(unittest.TestCase):
    def test_component_round_trip_and_immutable_data(self) -> None:
        original = {"position": [0, 1, 2]}
        component = Component("c1", "transform", 2, original, {"source": "sdk"})
        original["position"][0] = 99
        self.assertEqual(component.data["position"], (0, 1, 2))
        self.assertEqual(Component.from_dict(component.to_dict()), component)

    def test_invalid_component_version_is_rejected(self) -> None:
        with self.assertRaises(RealityValidationError):
            Component("c1", "transform", True, {})

    def test_component_key_serializes_with_public_casing(self) -> None:
        self.assertEqual(ComponentKey("entity-1", "c1").to_dict(), {
            "ownerId": "entity-1", "componentId": "c1",
        })


class BehaviourContractTests(unittest.TestCase):
    def test_behaviour_reference_and_definition_round_trip(self) -> None:
        ref = BehaviourRef("b1", "event-handler", {"events": ["ready"]})
        definition = BehaviourDefinition("b1", "event-handler", 3, {"enabled": True}, "Lifecycle handler")
        self.assertEqual(BehaviourRef.from_dict(ref.to_dict()), ref)
        self.assertEqual(BehaviourDefinition.from_dict(definition.to_dict()), definition)
        self.assertEqual(ref.config["events"], ("ready",))

    def test_zero_behaviour_version_is_rejected(self) -> None:
        with self.assertRaises(RealityValidationError):
            BehaviourDefinition("b1", "event-handler", 0, {})


class ActionContractTests(unittest.TestCase):
    def test_invocation_round_trip_and_idempotency_fields(self) -> None:
        invocation = ActionInvocation("i1", "r1", "move", {"position": [1, 2]}, "retry-key")
        wire = invocation.to_dict()
        self.assertEqual(wire["idempotencyKey"], "retry-key")
        self.assertEqual(ActionInvocation.from_dict(wire), invocation)

    def test_invocation_rejects_invalid_timestamp_and_unknown_fields(self) -> None:
        with self.assertRaises(RealityValidationError):
            ActionInvocation("i1", "r1", "move", {}, "key", requested_at="yesterday")
        with self.assertRaises(RealityValidationError):
            ActionInvocation.from_dict({
                "invocationId": "i1", "realityId": "r1", "actionId": "move",
                "input": {}, "idempotencyKey": "key", "extra": True,
            })

    def test_all_outcome_variants_round_trip(self) -> None:
        outcomes = (
            ActionSucceeded({"done": True}, "2026-10-08T12:00:00Z"),
            ActionAccepted("operation-1", "2026-10-08T12:00:00Z"),
            ActionFailed(ActionFailure("denied", "Policy rejected request", False), "2026-10-08T12:00:00Z"),
        )
        for outcome in outcomes:
            self.assertEqual(action_outcome_from_dict(action_outcome_to_dict(outcome)), outcome)

    def test_outcome_output_is_immutable(self) -> None:
        output = {"nested": [1]}
        outcome = ActionSucceeded(output, "2026-10-08T12:00:00Z")
        output["nested"][0] = 2
        self.assertEqual(outcome.output["nested"], (1,))


if __name__ == "__main__":
    unittest.main()
