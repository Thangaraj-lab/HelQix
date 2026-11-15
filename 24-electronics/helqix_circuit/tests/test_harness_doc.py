"""harness/0.1 document: connectors, wires, bundles, computed routes."""

import copy
import json
from pathlib import Path

import pytest

from helqix_circuit import InputError
from helqix_circuit.harness_doc import colour_entry, load_harness

HARNESS_EXAMPLES = Path(__file__).resolve().parents[2] / "examples" / "harness"
SCHEMA = Path(__file__).resolve().parents[3] / "docs" / "priya" / "schemas" / "harness.schema.json"


def document(**wire_overrides):
    wire = {
        "id": "W1",
        "from": "n1",
        "to": "n2",
        "from_pin": "J1.1",
        "to_pin": "J2.1",
        "gauge_awg": 20,
        "material": "copper",
        "insulation": {"colour": "red", "rating_c": 105},
        "length_mm": 600.0,
        "slack_mm": 25.0,
        "bend_radius_mm": 6.0,
        "bundle": "B1",
    }
    wire.update(wire_overrides)
    return {
        "version": "harness/0.1",
        "connectors": [
            {"id": "J1", "at_mm": [0.0, 0.0], "pins": {"1": [0.0, 0.0]}},
            {"id": "J2", "at_mm": [500.0, 0.0], "pins": {"1": [500.0, 0.0]}},
        ],
        "wires": [wire],
        "bundles": [{"id": "B1", "diameter_mm": 8.0}],
        "obstacles": [{"id": "fan", "box_mm": [200.0, -40.0, 300.0, 40.0]}],
    }


def test_document_derives_sizes_and_computes_the_route():
    wire = load_harness(document()).to_dict()["wires"][0]
    assert wire["diameter_mm"]["value"] == pytest.approx(0.8118, abs=5e-4)
    assert wire["diameter_mm"]["label"] == "CALCULATED"
    assert wire["area_mm2"]["value"] == pytest.approx(0.5177, abs=5e-4)
    route = wire["route"]
    assert route["points_mm"] == [[0.0, 0.0], [0.0, -46.0], [500.0, -46.0], [500.0, 0.0]]
    assert route["cut_length_mm"]["value"] == pytest.approx(586.8496, abs=1e-4)
    assert wire["length_margin_mm"]["value"] == pytest.approx(13.1504, abs=1e-4)
    assert wire["route_fits"] is True


def test_route_is_computed_and_a_supplied_polyline_is_refused():
    body = load_harness(document()).to_dict()
    assert "computed" in body["route_note"]
    with pytest.raises(InputError, match="unknown wire key"):
        load_harness(document(points_mm=[[0.0, 0.0], [500.0, 0.0]]))


def test_short_declared_length_is_reported_not_refused():
    wire = load_harness(document(length_mm=500.0)).to_dict()["wires"][0]
    assert wire["route_fits"] is False
    assert wire["length_margin_mm"]["value"] == pytest.approx(500.0 - 586.8496, abs=1e-4)


def test_colour_table_entries_and_unknown_colour():
    red = colour_entry("Red")
    assert red["name"] == "red"
    assert red["code"] == "RD"
    assert red["label"] == "REPORTED"
    earth = colour_entry("GNYE")
    assert earth["name"] == "green-yellow"
    assert earth["role"] == "protective earth"
    phase = colour_entry("brown")
    assert phase["role"] == "L1 phase"
    with pytest.raises(InputError, match="colour"):
        colour_entry("teal")


def test_blocked_route_is_reported_not_raised():
    body = document(to_pin="J2.1")
    body["connectors"][1]["pins"]["1"] = [500.0, 0.0]
    body["obstacles"] = [
        {"id": "a", "box_mm": [490.0, 20.0, 520.0, 22.0]},
        {"id": "b", "box_mm": [490.0, -22.0, 520.0, -20.0]},
        {"id": "c", "box_mm": [488.0, -22.0, 490.0, 22.0]},
        {"id": "d", "box_mm": [520.0, -22.0, 522.0, 22.0]},
    ]
    wire = load_harness(body).to_dict()["wires"][0]
    assert wire["route"]["blocked"] is True
    assert wire["route"]["cut_length_mm"] == {"value": None, "unit": "mm", "label": "UNKNOWN"}
    assert wire["route_fits"] is None
    assert wire["length_margin_mm"]["label"] == "UNKNOWN"


def test_same_document_gives_the_same_json():
    first = load_harness(document()).to_dict()
    second = load_harness(document()).to_dict()
    assert first == second


def test_bad_documents_are_refused():
    body = document()
    body["version"] = "harness/0.2"
    with pytest.raises(InputError, match="harness/0.1"):
        load_harness(body)
    with pytest.raises(InputError, match="unknown harness key"):
        load_harness({**document(), "routes": []})
    with pytest.raises(InputError, match="unknown pin"):
        load_harness(document(to_pin="J9.1"))
    with pytest.raises(InputError, match="unknown bundle"):
        load_harness(document(bundle="B9"))
    with pytest.raises(InputError, match="exactly one of gauge_awg or diameter_mm"):
        load_harness(document(diameter_mm=1.0))
    with pytest.raises(InputError, match="length_mm"):
        load_harness(document(length_mm=0.0))
    with pytest.raises(InputError, match="slack_mm"):
        load_harness(document(slack_mm=-1.0))
    with pytest.raises(InputError, match="bend_radius_mm"):
        load_harness(document(bend_radius_mm=0.0))
    with pytest.raises(InputError, match="rating_c"):
        load_harness(document(insulation={"colour": "red", "rating_c": 0}))
    with pytest.raises(InputError, match="colour"):
        load_harness(document(insulation={"colour": "teal", "rating_c": 105}))

    duplicate = document()
    duplicate["wires"].append(copy.deepcopy(duplicate["wires"][0]))
    with pytest.raises(InputError, match="duplicate wire id"):
        load_harness(duplicate)

    bad_box = document()
    bad_box["obstacles"][0]["box_mm"] = [300.0, 0.0, 200.0, 40.0]
    with pytest.raises(InputError, match="x0 < x1"):
        load_harness(bad_box)

    dot_id = document()
    dot_id["connectors"][0]["id"] = "J.1"
    with pytest.raises(InputError, match="dot"):
        load_harness(dot_id)

    bad_pin = document()
    bad_pin["connectors"][0]["pins"]["1"] = [0.0]
    with pytest.raises(InputError, match="pair of numbers"):
        load_harness(bad_pin)


def test_every_harness_example_loads_and_routes():
    paths = sorted(HARNESS_EXAMPLES.glob("*.json"))
    assert paths, "at least one harness example ships with the package"
    for path in paths:
        body = json.loads(path.read_text(encoding="utf-8"))
        doc = load_harness(body).to_dict()
        assert doc["version"] == "harness/0.1"
        for wire in doc["wires"]:
            assert wire["route"]["blocked"] is False, (path.name, wire["id"])


def test_schema_document_is_valid_json_for_this_version():
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    assert schema["properties"]["version"]["const"] == "harness/0.1"
    assert "route" not in schema["properties"]["wires"]["items"]["properties"]
