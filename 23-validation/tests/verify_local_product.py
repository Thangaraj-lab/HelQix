"""Run local HTTP checks across HELQIX account, domain, intent, and tool integrations.

Start a throwaway local gateway first. This script intentionally creates a
unique account, project, two scenario records, a verification fixture, (when
available) a real PyBaMM discharge and Blender .blend, and an evidence bundle.
"""

from __future__ import annotations

import argparse
import hashlib
import http.cookiejar
import json
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


SPECIFICATION = {
    "schema": "helqix.drone-specification.v1",
    "aircraft": "quadrotor",
    "frameDiagonalMm": 650,
    "dryMassKg": 2.4,
    "payloadMassKg": 2,
    "batteryMassKg": 1.1,
    "batteryCapacityMah": 16000,
    "batteryVoltageV": 22.2,
    "propellerDiameterIn": 15,
    "motorKvRpmPerVolt": 320,
}

EV_CONFIGURATION = {
    "schema": "helqix.ev-battery-vehicle.v1",
    "vehicleName": "HELQIX validation EV",
    "vehicleMassKg": 1500,
    "dragCoefficient": 0.29,
    "frontalAreaM2": 2.2,
    "rollingResistanceCoefficient": 0.01,
    "drivetrainEfficiency": 0.9,
    "accessoryPowerW": 250,
    "batteryPack": {
        "seriesCells": 96,
        "parallelCells": 10,
        "cellCapacityAh": 2.5,
        "cellNominalVoltageV": 3.6,
        "cellCutoffVoltageV": 2.7,
        "cellMaxContinuousCurrentA": 10,
        "initialSoc": 0.8,
    },
    "driveCycle": [
        {"timeS": 0, "speedMS": 0, "roadGradePercent": 0},
        {"timeS": 2, "speedMS": 5, "roadGradePercent": 0},
        {"timeS": 4, "speedMS": 5, "roadGradePercent": 0},
    ],
    "modelId": "pybamm.lithium-ion.spm",
    "parameterSet": "Chen2020",
}


def request(opener: urllib.request.OpenerDirector, base_url: str, path: str,
            method: str = "GET", body: dict | None = None,
            headers: dict[str, str] | None = None) -> tuple[int, dict, object | None]:
    data = None if body is None else json.dumps(body, allow_nan=False).encode("utf-8")
    req = urllib.request.Request(
        base_url + path,
        data=data,
        method=method,
        headers={"Content-Type": "application/json", **(headers or {})},
    )
    try:
        with opener.open(req, timeout=240) as response:
            raw = response.read()
            payload = json.loads(raw) if raw else None
            return response.status, dict(response.headers.items()), payload
    except urllib.error.HTTPError as error:
        raw = error.read()
        payload = json.loads(raw) if raw else None
        return error.code, dict(error.headers.items()), payload


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:8091")
    parser.add_argument("--ui-origin", default="http://127.0.0.1:5178")
    args = parser.parse_args()
    base = args.base_url.rstrip("/")
    parsed = urllib.parse.urlparse(base)
    require(parsed.scheme == "http" and parsed.hostname in {"127.0.0.1", "localhost"},
            "Refusing to create test data outside a loopback HTTP gateway")

    status, _, health = request(urllib.request.build_opener(), base, "/health/live")
    require(status == 200 and health.get("status") == "live", "Gateway health check failed")

    preflight = urllib.request.Request(
        base + "/v1/projects",
        method="OPTIONS",
        headers={
            "Origin": args.ui_origin,
            "Access-Control-Request-Method": "POST",
            "Access-Control-Request-Headers": "content-type,idempotency-key,if-match",
        },
    )
    with urllib.request.build_opener().open(preflight, timeout=10) as response:
        allowed = response.headers.get("access-control-allow-headers", "").lower()
        require(response.status == 200 and "idempotency-key" in allowed and "if-match" in allowed,
                "Browser CORS preflight does not allow the project's required request headers")

    cookies = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookies))
    status, _, denied = request(opener, base, "/v1/projects")
    require(status == 401, f"Unauthenticated project request must be denied, got {status}: {denied}")

    suffix = uuid.uuid4().hex
    status, _, signup = request(opener, base, "/v1/auth/signup", "POST", {
        "name": "HELQIX local validation",
        "email": f"helqix-validation-{suffix}@example.invalid",
        "password": f"Local-validation-{suffix}-Long!",
        "organization": "Local smoke check",
    })
    require(status == 201 and signup.get("user"), f"Account creation failed: {signup}")
    status, _, session = request(opener, base, "/v1/auth/me")
    require(status == 200 and session["user"]["email"].endswith("@example.invalid"), "Session cookie did not restore the account")

    catalog_query = urllib.parse.urlencode({"domain": "battery electric vehicle"})
    status, _, catalog = request(opener, base, f"/v1/libraries/component-types?{catalog_query}")
    catalog_entries = {entry["id"]: entry for entry in (catalog or {}).get("entries", [])}
    require(status == 200 and "lithium_ion_cell" in catalog_entries
            and catalog_entries["lithium_ion_cell"]["supportedModels"][0]["modelId"] == "pybamm.lithium-ion.spm"
            and "physical cell matches" in catalog_entries["lithium_ion_cell"]["supportedModels"][0]["scope"],
            f"The authenticated vehicle catalog did not expose the scoped lithium-ion solver binding: {catalog}")

    intent = "Agricultural drone: 2 kg payload, 30 minute mission in 12 m/s wind; compare 2 kg battery and 5 kg battery."
    status, _, analysis = request(opener, base, "/v1/intelligence/intent/parse", "POST", {"intent": intent})
    require(status == 200, f"Intent extraction failed: {analysis}")
    keys = {item["key"] for item in analysis["candidates"]}
    require({"mission_type", "payload_mass_kg", "target_duration_min", "wind_speed_m_s"}.issubset(keys),
            "Intent extraction missed explicit drone inputs")
    require(analysis["conflicts"] == ["battery_mass_kg"], "Conflicting battery masses were not flagged for review")
    spaced_intent = "  Electric vehicle: target range 420 km. \n"
    status, _, spaced_analysis = request(opener, base, "/v1/intelligence/intent/parse", "POST",
        {"intent": spaced_intent, "domain": "vehicle"})
    candidate = next((item for item in spaced_analysis.get("candidates", []) if item["key"] == "target_range_km"), None)
    require(status == 200 and spaced_analysis.get("source_text") == spaced_intent and candidate is not None
            and spaced_intent[candidate["start"]:candidate["end"]] == candidate["source_text"],
            f"Intent extraction must preserve exact input and source character offsets: {spaced_analysis}")

    status, _, invalid = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": intent, "specification": {**SPECIFICATION, "batteryMassKg": 0}}
    })
    require(status == 422, f"Invalid draft input should be rejected, got {status}: {invalid}")

    status, _, project = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": intent, "specification": SPECIFICATION, "intent_analysis": analysis}
    })
    require(status == 201, f"Valid project draft could not be saved: {project}")
    project_id = project["data"]["id"]

    status, _, ev_project = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": "Battery-electric vehicle drive-cycle study", "ev_vehicle": EV_CONFIGURATION}
    })
    require(status == 201, f"Explicit EV model could not be saved: {ev_project}")
    ev_project_id = ev_project["data"]["id"]
    saved_ev_configuration = ev_project["data"]["payload"]["ev_vehicle"]

    # The HELQIX project contract is domain-neutral: verify another engineered
    # product can use shared reality structure without passing drone fields.
    pump_reality = {
        "schema": "helqix.engineering-reality.v1",
        "name": "Process pump assembly", "domain": "industrial.pump",
        "components": [
            {"id": "pump", "name": "Main pump", "component_type": "centrifugal_pump",
             "properties": [{"name": "rated_power", "value": 2.2, "unit": "kW", "origin": "catalog", "source_ref": "catalog:pump-22"}]},
            {"id": "drive", "name": "Variable speed drive", "component_type": "motor_controller"},
        ],
        "interfaces": [
            {"id": "pump.power", "component_id": "pump", "name": "Power input", "direction": "in", "domain": "electrical"},
            {"id": "drive.output", "component_id": "drive", "name": "Drive output", "direction": "out", "domain": "electrical"},
        ],
        "connections": [{"id": "drive-to-pump", "source_interface_id": "drive.output", "target_interface_id": "pump.power", "connection_type": "electrical_power"}],
        "requirements": [{"id": "flow", "statement": "Deliver required process flow", "priority": "must", "review_status": "draft"}],
        "model_bindings": [{"id": "pump-curve", "model_id": "pump.curve", "model_version": "1.0", "scope_component_id": "pump", "declared_fidelity": "vendor curve"}],
    }
    status, _, generic_project = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": "Process pump assembly", "engineering_reality": pump_reality}
    })
    require(status == 201 and generic_project["data"]["payload"]["engineering_reality"]["domain"] == "industrial.pump",
            f"Domain-neutral engineering reality did not persist through the project API: {generic_project}")
    generic_project_id = generic_project["data"]["id"]
    status, _, reality_run_response = request(opener, base, "/v1/integrations/helqix_reality/execute", "POST", {
        "operation": "validate_reality", "inputs": {"project_id": generic_project_id}
    })
    require(status == 200, f"Saved generic engineering model could not be validated: {reality_run_response}")
    reality_run = reality_run_response["run"]
    reality_result = reality_run["result"]
    require(reality_result.get("status") == "structurally_valid_draft"
            and reality_result.get("counts") == {"components": 2, "properties": 1, "interfaces": 2,
                                                   "connections": 1, "requirements": 1, "model_bindings": 1},
            f"Reality validator returned unexpected structural results: {reality_result}")
    model_snapshot = reality_run.get("project_snapshot", {}).get("models", {}).get("engineering_reality", {})
    require(model_snapshot.get("revision") == 1 and len(model_snapshot.get("sha256", "")) == 64,
            f"Reality run did not capture its saved model revision and digest: {reality_run}")
    run_query = urllib.parse.urlencode({"project_id": generic_project_id, "limit": 10})
    status, _, generic_runs = request(opener, base, f"/v1/integrations/runs?{run_query}")
    require(status == 200 and len(generic_runs["runs"]) == 1
            and generic_runs["runs"][0]["project_snapshot"] == reality_run["project_snapshot"],
            f"Project-scoped run history did not retain model provenance: {generic_runs}")
    status, _, activity = request(opener, base, f"/v1/projects/{generic_project_id}/activity?limit=50")
    actions = [event["action"] for event in activity.get("events", [])]
    require(status == 200 and "resource.created" in actions and "tool_run.recorded" in actions,
            f"Append-only project activity did not include saved model and execution records: {activity}")
    require(all("inputs" not in event["details"] and "result" not in event["details"]
                for event in activity["events"]),
            "Audit activity must not duplicate arbitrary engineering inputs or model outputs")
    status, export_headers, generic_evidence = request(
        opener, base, f"/v1/projects/{generic_project_id}/evidence/export")
    require(status == 200 and generic_evidence.get("project", {}).get("id") == generic_project_id
            and len(generic_evidence.get("integration_runs", [])) == 1
            and generic_evidence["integration_runs"][0].get("project_snapshot") == reality_run["project_snapshot"],
            f"Evidence export did not preserve the generic product model and its run provenance: {generic_evidence}")
    generic_unsigned_bundle = {key: value for key, value in generic_evidence.items() if key != "bundle_sha256"}
    generic_digest = hashlib.sha256(json.dumps(
        generic_unsigned_bundle, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")).hexdigest()
    generic_digest_header = next((value for name, value in export_headers.items()
                                  if name.lower() == "x-helqix-bundle-sha256"), None)
    require(generic_digest == generic_evidence["bundle_sha256"]
            and generic_digest_header == generic_digest,
            "Generic engineering evidence bundle digest did not match its contents")
    settings_payload = {
        **generic_project["data"]["payload"],
        "name": "Process pump prototype",
        "domain": "industrial.rotating_equipment",
        "description": "Versioned settings update; model components remain unchanged.",
        "engineering_reality": {
            **pump_reality,
            "name": "Process pump prototype",
            "domain": "industrial.rotating_equipment",
            "description": "Versioned settings update; model components remain unchanged.",
            "revision": 2,
        },
    }
    status, _, settings_update = request(opener, base, f"/v1/projects/{generic_project_id}", "PUT", {
        "payload": settings_payload,
    }, headers={"If-Match": f'"{generic_project["data"]["version"]}"'})
    require(status == 200 and settings_update["data"]["version"] == 2
            and settings_update["data"]["payload"]["engineering_reality"]["revision"] == 2
            and settings_update["data"]["payload"]["engineering_reality"]["components"] == pump_reality["components"]
            and settings_update["data"]["payload"]["intent"] == generic_project["data"]["payload"]["intent"],
            f"Versioned project settings update changed or dropped engineering data: {settings_update}")
    status, _, stale_settings = request(opener, base, f"/v1/projects/{generic_project_id}", "PUT", {
        "payload": generic_project["data"]["payload"],
    }, headers={"If-Match": f'"{generic_project["data"]["version"]}"'})
    require(status == 412, f"A stale project settings form must not overwrite a newer revision: {stale_settings}")
    invalid_reality = {**pump_reality, "interfaces": [
        {"id": "bad.port", "component_id": "absent", "name": "Power input", "direction": "in", "domain": "electrical"}
    ]}
    status, _, invalid_generic = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": "Invalid product structure", "engineering_reality": invalid_reality}
    })
    require(status == 422, f"A product reality with dangling component links must be rejected: {invalid_generic}")
    incompatible_reality = {**pump_reality, "interfaces": [
        pump_reality["interfaces"][0],
        {**pump_reality["interfaces"][1], "domain": "control_signal"},
    ]}
    status, _, incompatible_generic = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": "Mismatched interface domains", "engineering_reality": incompatible_reality}
    })
    require(status == 422 and "incompatible interface domains" in str(incompatible_generic).lower(),
            f"A cross-domain connection without a declared converter must be rejected: {incompatible_generic}")
    battery_reality = {
        "schema": "helqix.engineering-reality.v1", "name": "Cell discharge test", "domain": "energy.battery",
        "components": [{"id": "cell", "name": "Test lithium-ion cell", "component_type": "lithium_ion_cell",
                        "properties": [
                            {"name": "current_a", "value": 2.0, "unit": "A", "origin": "user"},
                            {"name": "cutoff_voltage_v", "value": 3.0, "unit": "V", "origin": "user"},
                        ]}],
        "requirements": [
            {"id": "capacity-minimum", "statement": "Deliver at least 0.1 Ah under the declared discharge test",
             "priority": "must", "review_status": "accepted",
             "verification_rule": {"metric": "discharged_capacity_ah", "operator": ">=", "threshold": 0.1, "unit": "Ah"}},
            {"id": "unattainable-duration", "statement": "Run for at least 1000 hours",
             "priority": "should", "review_status": "accepted",
             "verification_rule": {"metric": "duration_h", "operator": ">=", "threshold": 1000, "unit": "h"}},
        ],
        "model_bindings": [{"id": "cell-spm", "model_id": "pybamm.lithium-ion.spm", "model_version": "Chen2020",
                            "scope_component_id": "cell", "declared_fidelity": "single-cell SPM"}],
    }
    status, _, battery_project = request(opener, base, "/v1/projects", "POST", {
        "payload": {"intent": "Run an explicit single-cell discharge analysis", "engineering_reality": battery_reality}
    })
    require(status == 201, f"Compatible battery-cell engineering reality did not save: {battery_project}")
    battery_project_id = battery_project["data"]["id"]
    battery_branch = {
        "schema": "helqix.engineering-branch.v1", "name": "Higher-current cell case",
        "description": "Compare an explicit 3 A discharge against the 2 A baseline.",
        "base_reality_revision": 1,
        "changes": [{"component_id": "cell", "property_name": "current_a", "value": 3.0, "unit": "A"}],
    }
    status, _, battery_branch_record = request(opener, base, "/v1/branches", "POST", {
        "project_id": battery_project_id, "payload": battery_branch,
    })
    require(status == 201, f"Versioned engineering branch could not be saved: {battery_branch_record}")
    battery_branch_id = battery_branch_record["data"]["id"]

    for mass, capacity, voltage in ((2, 16000, 22.2), (5, 30000, 44.4)):
        scenario = {
            "schema": "helqix.drone-scenario.v1",
            "name": f"{mass} kg battery",
            "description": "Configuration only; no simulation result",
            "base_specification": SPECIFICATION,
            "parameters": {"battery_mass_kg": mass, "battery_capacity_mah": capacity,
                           "battery_voltage_v": voltage, "payload_mass_kg": 2, "wind_speed_m_s": 12},
            "execution_state": "not_run",
        }
        status, _, saved = request(opener, base, "/v1/branches", "POST", {
            "project_id": project_id, "payload": scenario
        })
        require(status == 201, f"Scenario configuration did not persist: {saved}")

    query = urllib.parse.urlencode({"project_id": project_id, "limit": 100})
    status, _, scenarios = request(opener, base, f"/v1/branches?{query}")
    require(status == 200 and len(scenarios["data"]) == 2, "Saved scenario variants did not round-trip")
    require({row["payload"]["parameters"]["battery_mass_kg"] for row in scenarios["data"]} == {2, 5},
            "The 2 kg / 5 kg scenario values changed in storage")
    require({row["payload"]["parameters"]["battery_capacity_mah"] for row in scenarios["data"]} == {16000, 30000}
            and {row["payload"]["parameters"]["battery_voltage_v"] for row in scenarios["data"]} == {22.2, 44.4},
            "Explicit pack capacity or nominal voltage did not round-trip")
    require(all(row["payload"]["execution_state"] == "not_run" for row in scenarios["data"]),
            "A configuration-only scenario must never be reported as executed")

    verification_fixture = {
        "id": "validation-fixture-not-an-engineering-result",
        "code": "TEST-ONLY",
        "requirementTitle": "Validation harness fixture (not an engineering verification)",
        "governingStandard": "Not configured",
        "conditionFormula": "Not evaluated",
        "observedValue": "Not measured",
        "limitThreshold": "Not configured",
        "marginPercent": 0,
        "verdict": "inconclusive",
    }
    status, _, saved_verification = request(opener, base, "/v1/verifications", "POST", {
        "project_id": project_id,
        "payload": verification_fixture,
    })
    require(status == 201, f"Verification record could not be saved: {saved_verification}")
    status, _, fetched_verification = request(opener, base, f"/v1/verifications?project_id={project_id}&limit=100")
    require(status == 200 and any(row["payload"].get("code") == "TEST-ONLY" for row in fetched_verification["data"]),
            "Project-scoped verification resource could not be fetched")

    status, _, inventory = request(opener, base, "/v1/integrations")
    require(status == 200, f"Tool inventory failed: {inventory}")
    pybamm = next((tool for tool in inventory["integrations"] if tool["id"] == "pybamm"), None)
    for _ in range(30):
        if not pybamm or pybamm.get("status") != "checking":
            break
        time.sleep(0.5)
        status, _, inventory = request(opener, base, "/v1/integrations")
        require(status == 200, f"Tool inventory retry failed: {inventory}")
        pybamm = next((tool for tool in inventory["integrations"] if tool["id"] == "pybamm"), None)
    pybamm_result = "skipped; PyBaMM runtime is unavailable"
    if pybamm and pybamm["available"]:
        status, _, battery_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_discharge",
            "inputs": {"current_a": 5, "cutoff_voltage_v": 3, "project_id": project_id},
        })
        require(status == 200, f"PyBaMM benchmark did not execute: {battery_run}")
        battery_result = battery_run["run"]["result"]
        require(battery_result.get("model") == "SPM"
                and battery_result.get("parameter_set") == "Chen2020"
                and battery_result.get("duration_h", 0) > 0
                and battery_result.get("discharged_capacity_ah", 0) > 0
                and battery_result.get("sample_count", 0) >= 2,
                f"PyBaMM returned an incomplete discharge report: {battery_result}")
        require(battery_run["run"].get("project_id") == project_id,
                "PyBaMM run is not linked to its source project")
        status, _, ev_run_response = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_ev_drive_cycle", "inputs": {"project_id": ev_project_id},
        })
        require(status == 200, f"Saved EV model did not execute through the PyBaMM adapter: {ev_run_response}")
        ev_run = ev_run_response["run"]
        ev_snapshot = ev_run.get("project_snapshot", {}).get("models", {}).get("ev_vehicle", {})
        ev_digest = hashlib.sha256(json.dumps(saved_ev_configuration, sort_keys=True, separators=(",", ":"),
                                    allow_nan=False).encode("utf-8")).hexdigest()
        require(ev_run.get("inputs", {}).get("ev_vehicle_configuration") == saved_ev_configuration
                and ev_snapshot.get("sha256") == ev_digest
                and ev_run.get("result", {}).get("model_input_sha256") == ev_digest,
                f"EV run did not preserve its exact validated model inputs and digest: {ev_run}")
        ev_series = ev_run.get("result", {}).get("series", [])
        require(len(ev_series) >= 2
                and all(right["distance_m"] >= left["distance_m"] for left, right in zip(ev_series, ev_series[1:]))
                and "Prescribed-speed" in ev_run["result"].get("applicability", ""),
                f"EV run output is missing its recorded trajectory or scope: {ev_run}")
        heavier_ev_configuration = {**EV_CONFIGURATION, "vehicleMassKg": 1800}
        status, _, updated_ev_project = request(opener, base, f"/v1/projects/{ev_project_id}", "PUT", {
            "payload": {**ev_project["data"]["payload"], "ev_vehicle": heavier_ev_configuration},
        }, headers={"If-Match": f'"{ev_project["data"]["version"]}"'})
        require(status == 200, f"EV alternative configuration did not save against its current project version: {updated_ev_project}")
        saved_heavier_ev_configuration = updated_ev_project["data"]["payload"]["ev_vehicle"]
        status, _, heavier_ev_response = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_ev_drive_cycle", "inputs": {"project_id": ev_project_id},
        })
        require(status == 200, f"Second saved EV configuration did not execute: {heavier_ev_response}")
        heavier_ev_run = heavier_ev_response["run"]
        heavier_digest = hashlib.sha256(json.dumps(saved_heavier_ev_configuration, sort_keys=True, separators=(",", ":"),
                                           allow_nan=False).encode("utf-8")).hexdigest()
        require(heavier_ev_run.get("inputs", {}).get("ev_vehicle_configuration") == saved_heavier_ev_configuration
                and heavier_ev_run.get("project_snapshot", {}).get("models", {}).get("ev_vehicle", {}).get("sha256") == heavier_digest
                and heavier_ev_run.get("result", {}).get("model_input_sha256") == heavier_digest
                and heavier_ev_run["result"].get("peak_pack_power_w", 0) > ev_run["result"].get("peak_pack_power_w", 0)
                and heavier_ev_run["result"].get("distance_m") == ev_run["result"].get("distance_m"),
                f"Mass variant should change computed load while preserving the same prescribed trajectory: {heavier_ev_run}")
        status, _, incompatible_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_bound_cell", "inputs": {"project_id": generic_project_id}
        })
        require(status == 422, f"The PyBaMM adapter must reject an incompatible pump model binding: {incompatible_run}")
        status, _, bound_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_bound_cell", "inputs": {"project_id": battery_project_id, "binding_id": "cell-spm"}
        })
        require(status == 200, f"Bound PyBaMM single-cell run did not execute: {bound_run}")
        bound_result = bound_run["run"]["result"]
        require(bound_result.get("status") == "completed"
                and bound_result.get("component_id") == "cell"
                and bound_result.get("model_version") == "Chen2020"
                and bound_result.get("inputs", {}).get("current_a") == 2.0
                and bound_result.get("duration_h", 0) > 0
                and 2 <= len(bound_result.get("series", [])) <= 201
                and bound_result["series"][-1]["time_h"] == bound_result["duration_h"]
                and "not a battery-pack" in bound_result.get("applicability", "")
                and bound_run["run"].get("project_snapshot", {}).get("models", {}).get("engineering_reality", {}).get("sha256"),
                f"Bound PyBaMM result is incomplete or missing model provenance/applicability: {bound_run}")
        status, _, branch_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_bound_cell",
            "inputs": {"project_id": battery_project_id, "binding_id": "cell-spm", "branch_id": battery_branch_id},
        })
        require(status == 200, f"Compatible versioned branch did not execute: {branch_run}")
        branch_result = branch_run["run"]["result"]
        require(branch_result.get("inputs", {}).get("current_a") == 3.0
                and branch_result.get("branch", {}).get("changes") == [{"component_id": "cell", "property_name": "current_a", "base_value": 2.0, "value": 3.0, "unit": "A"}]
                and branch_result.get("branch", {}).get("sha256")
                and branch_result.get("status") == "completed"
                and branch_result.get("duration_h", 0) > 0
                and branch_run["run"].get("project_snapshot", {}).get("models", {}).get("engineering_reality", {}).get("sha256")
                == bound_run["run"].get("project_snapshot", {}).get("models", {}).get("engineering_reality", {}).get("sha256")
                and branch_result.get("duration_h", 0) < bound_result.get("duration_h", 0),
                f"Branch run did not apply and trace the declared current delta: {branch_run}")
        low_current_branch_payload = {
            "schema": "helqix.engineering-branch.v1", "name": "Search candidate · 1 A",
            "description": "Explicit candidate generated by a bounded current sweep.",
            "base_reality_revision": 1,
            "changes": [{"component_id": "cell", "property_name": "current_a", "value": 1.0, "unit": "A"}],
        }
        status, _, low_current_branch = request(opener, base, "/v1/branches", "POST", {
            "project_id": battery_project_id, "payload": low_current_branch_payload,
        })
        require(status == 201, f"Bounded search candidate branch could not be persisted: {low_current_branch}")
        status, _, low_current_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_bound_cell", "inputs": {"project_id": battery_project_id,
                "binding_id": "cell-spm", "branch_id": low_current_branch["data"]["id"]},
        })
        require(status == 200 and low_current_run["run"]["result"].get("status") == "completed",
                f"Bounded search candidate did not produce a real PyBaMM result: {low_current_run}")
        search_candidates = [
            {"current_a": 1.0, "state": "completed", "branch_id": low_current_branch["data"]["id"],
             "run_id": low_current_run["run"]["id"], "duration_h": low_current_run["run"]["result"]["duration_h"],
             "discharged_capacity_ah": low_current_run["run"]["result"]["discharged_capacity_ah"],
             "final_voltage_v": low_current_run["run"]["result"]["final_voltage_v"]},
            {"current_a": 2.0, "state": "completed", "run_id": bound_run["run"]["id"],
             "duration_h": bound_result["duration_h"], "discharged_capacity_ah": bound_result["discharged_capacity_ah"],
             "final_voltage_v": bound_result["final_voltage_v"]},
            {"current_a": 3.0, "state": "completed", "branch_id": battery_branch_id,
             "run_id": branch_run["run"]["id"], "duration_h": branch_result["duration_h"],
             "discharged_capacity_ah": branch_result["discharged_capacity_ah"],
             "final_voltage_v": branch_result["final_voltage_v"]},
        ]
        best_search_candidate = max(search_candidates, key=lambda item: item["discharged_capacity_ah"])
        search_payload = {
            "schema": "helqix.bounded-search.v1", "name": "Single-cell current sweep",
            "status": "completed", "base_reality_revision": 1, "binding_id": "cell-spm",
            "variable": {"component_id": "cell", "property_name": "current_a", "unit": "A", "values": [1.0, 2.0, 3.0]},
            "objective": {"metric": "discharged_capacity_ah", "direction": "maximize"},
            "constraint": {"metric": "duration_h", "operator": ">=", "value_h": None},
            "candidate_count": 3, "completed_count": 3,
            "best_evaluated_candidate_a": best_search_candidate["current_a"],
            "candidates": search_candidates,
            "applicability": "Finite PyBaMM single-cell sweep only; no global optimum or pack prediction.",
        }
        status, _, saved_search = request(opener, base, "/v1/searches", "POST", {
            "project_id": battery_project_id, "payload": search_payload,
        })
        require(status == 201 and saved_search["data"]["payload"]["candidate_count"] == 3
                and saved_search["data"]["payload"]["best_evaluated_candidate_a"] == best_search_candidate["current_a"],
                f"Executed candidate sweep and objective ranking were not saved: {saved_search}")
        status, _, listed_searches = request(opener, base, f"/v1/searches?project_id={battery_project_id}&limit=100")
        require(status == 200 and any(record["id"] == saved_search["data"]["id"] for record in listed_searches["data"]),
                f"Saved bounded search did not round-trip through the project API: {listed_searches}")
        run_query = urllib.parse.urlencode({"project_id": battery_project_id, "limit": 20})
        status, _, battery_history = request(opener, base, f"/v1/integrations/runs?{run_query}")
        require(status == 200 and any(item["id"] == bound_run["run"]["id"] for item in battery_history["runs"])
                and any(item["id"] == branch_run["run"]["id"] and item["inputs"].get("branch_id") == battery_branch_id for item in battery_history["runs"]),
                f"Project history does not contain baseline and branch runs: {battery_history}")
        status, _, verification_run = request(opener, base, "/v1/integrations/helqix_reality/execute", "POST", {
            "operation": "verify_bound_run",
            "inputs": {"project_id": battery_project_id, "run_id": bound_run["run"]["id"]},
        })
        require(status == 200, f"Explicit requirements could not be evaluated against the bound run: {verification_run}")
        checked_verdicts = {check["payload"]["requirement_id"]: check["payload"]["verdict"]
                            for check in verification_run["run"]["result"]["checks"]}
        require(checked_verdicts == {"capacity-minimum": "pass", "unattainable-duration": "fail"},
                f"Threshold verification returned incorrect results: {verification_run}")
        status, _, persisted_checks = request(opener, base, f"/v1/verifications?project_id={battery_project_id}&limit=100")
        require(status == 200 and len(persisted_checks["data"]) == 2,
                f"Verification results were not persisted to the project: {persisted_checks}")
        require(all(item["payload"].get("schema") == "helqix.verification-check.v1"
                    and item["payload"].get("source_run_id") == bound_run["run"]["id"]
                    and item["payload"].get("source_model_sha256") == item["payload"].get("current_model_sha256")
                    and isinstance(item["payload"].get("observed_value"), (int, float))
                    for item in persisted_checks["data"]),
                f"Verification records must retain structured measurements and source-model provenance: {persisted_checks}")
        status, _, revised_battery = request(opener, base, f"/v1/projects/{battery_project_id}")
        require(status == 200, f"Battery project could not be reloaded before editing: {revised_battery}")
        changed_reality = {**battery_reality, "revision": 2, "description": "Changed after source execution"}
        status, _, updated_battery = request(opener, base, f"/v1/projects/{battery_project_id}", "PUT", {
            "payload": {**revised_battery["data"]["payload"], "engineering_reality": changed_reality}
        }, {"If-Match": f'"{revised_battery["data"]["version"]}"'})
        require(status == 200, f"Concurrent-safe model revision update failed: {updated_battery}")
        status, _, stale_verification = request(opener, base, "/v1/integrations/helqix_reality/execute", "POST", {
            "operation": "verify_bound_run",
            "inputs": {"project_id": battery_project_id, "run_id": bound_run["run"]["id"]},
        })
        require(status == 200 and stale_verification["run"]["result"]["source_model_revision_matches"] is False
                and {check["payload"]["verdict"] for check in stale_verification["run"]["result"]["checks"]} == {"inconclusive"},
                f"Verifier must mark results inconclusive after the model revision changes: {stale_verification}")
        status, _, stale_branch_run = request(opener, base, "/v1/integrations/pybamm/execute", "POST", {
            "operation": "simulate_bound_cell",
            "inputs": {"project_id": battery_project_id, "branch_id": battery_branch_id},
        })
        require(status == 409, f"A branch based on an older engineering-reality revision must not execute: {stale_branch_run}")
        status, _, battery_evidence = request(opener, base, f"/v1/projects/{battery_project_id}/evidence/export")
        require(status == 200 and len(battery_evidence["resources"]["verifications"]) == 4
                and len(battery_evidence["resources"]["searches"]) == 1
                and len([run for run in battery_evidence["integration_runs"] if run["operation"] == "verify_bound_run"]) == 2,
                f"Evidence export omitted the model-bound verification chain: {battery_evidence}")
        exported_run_ids = {run["id"] for run in battery_evidence["integration_runs"]}
        require({bound_run["run"]["id"], branch_run["run"]["id"], low_current_run["run"]["id"]}.issubset(exported_run_ids),
                "Evidence export omitted one or more PyBaMM baseline, branch, or search candidate runs")
        pybamm_result = "executed real baseline, versioned branch, and bounded three-candidate PyBaMM SPM runs, persisted search ranking, verified explicit pass/fail thresholds and stale-model handling, and exported the evidence"
    blender = next((tool for tool in inventory["integrations"] if tool["id"] == "blender"), None)
    if blender and blender["available"]:
        asset_path = f"data/validation/{suffix}-quadrotor.blend"
        status, _, generated = request(opener, base, "/v1/integrations/blender/execute", "POST", {
            "operation": "create_quadrotor",
            # This untrusted override must be ignored in favor of the saved project design.
            "inputs": {"project_id": project_id, "output_path": asset_path,
                       "specification": {**SPECIFICATION, "frameDiagonalMm": 1200, "batteryMassKg": 40}},
        })
        require(status == 200, f"Blender export failed: {generated}")
        result = generated["run"]["result"]
        require(result["mesh_count"] >= 15 and result["engineering_status"].startswith("visual draft only"),
                "Blender output report is missing expected geometry or draft limitations")
        require(generated["run"].get("project_id") == project_id,
                "Blender integration output is not linked to its source project")
        require(abs(result["configured_mass_kg"] - 5.5) < 1e-9
                and abs(result["dimensions_basis"]["frame_diagonal_m"] - 0.65) < 1e-9
                and generated["run"]["inputs"]["specification"]["batteryMassKg"] == 1.1,
                "Blender used browser-supplied values instead of the saved project design")
        status, _, inspected = request(opener, base, "/v1/integrations/blender/execute", "POST", {
            "operation": "inspect_asset", "inputs": {"asset_path": asset_path}
        })
        require(status == 200 and inspected["run"]["result"]["mesh_count"] >= 15,
                f"Blender could not reopen and inspect the exported design: {inspected}")
        status, _, history = request(opener, base, f"/v1/integrations/runs?limit=12&project_id={project_id}")
        require(status == 200 and any(run.get("project_id") == project_id for run in history["runs"]),
                "Project-scoped integration run history did not return the generated asset run")
        blender_result = "generated and reopened Blender .blend"
    else:
        blender_result = "skipped; Blender is not installed"

    status, export_headers, evidence_bundle = request(
        opener, base, f"/v1/projects/{project_id}/evidence/export"
    )
    require(status == 200 and evidence_bundle.get("schema") == "helqix.project-evidence-bundle.v1",
            f"Project evidence bundle export failed: {evidence_bundle}")
    require(evidence_bundle["project"]["id"] == project_id
            and len(evidence_bundle["resources"]["branches"]) == 2
            and len(evidence_bundle["resources"]["verifications"]) == 1,
            "Evidence export omitted the source project, saved scenarios, or verification record")
    exported_check = evidence_bundle["resources"]["verifications"][0]["payload"]
    require(exported_check.get("code") == "TEST-ONLY" and exported_check.get("verdict") == "inconclusive",
            "Evidence export changed or misrepresented the stored verification record")
    unsigned_bundle = {key: value for key, value in evidence_bundle.items() if key != "bundle_sha256"}
    digest = hashlib.sha256(json.dumps(unsigned_bundle, ensure_ascii=False, sort_keys=True,
        separators=(",", ":"), allow_nan=False).encode("utf-8")).hexdigest()
    require(digest == evidence_bundle["bundle_sha256"], "Evidence bundle SHA-256 does not match its content")
    require(any(key.lower() == "content-disposition" and project_id in value
                for key, value in export_headers.items()), "Evidence export did not return a project-specific download filename")
    require(evidence_bundle["assurance"]["status"] == "traceability_only"
            and evidence_bundle["assurance"]["verification_established"] is False,
            "Evidence export must never claim verification without a verification contract")

    # A second account cannot read the first account's project-scoped tool runs.
    second_user = f"helqix-validation-other-{suffix}@example.invalid"
    status, _, second_signup = request(opener, base, "/v1/auth/signup", "POST", {
        "name": "HELQIX isolation check",
        "email": second_user,
        "password": f"Isolation-check-{suffix}-Long!",
        "organization": "Local smoke check",
    })
    require(status == 201 and second_signup.get("user"), f"Second test account could not be created: {second_signup}")
    if blender and blender["available"]:
        status, _, hidden_history = request(opener, base, f"/v1/integrations/runs?limit=12&project_id={project_id}")
        require(status == 404, f"A different account must not resolve the project's tool history, got {status}: {hidden_history}")
    status, _, hidden_bundle = request(opener, base, f"/v1/projects/{project_id}/evidence/export")
    require(status == 404, f"A different account must not export this project's evidence, got {status}: {hidden_bundle}")
    status, _, hidden_activity = request(opener, base, f"/v1/projects/{generic_project_id}/activity")
    require(status == 404, f"A different account must not read this project's activity, got {status}: {hidden_activity}")

    print("PASS: gateway CORS/auth, append-only project activity and owner isolation, explicit intent provenance, generic pump reality validation and dangling-link rejection, revision/digest run provenance, project evidence digest and export, Layer 18 scoped cell catalog and drone validation, project persistence, 2/5 kg scenarios with explicit pack ratings, " + pybamm_result + ", " + blender_result + ", and cross-account isolation")
    print("The check does not run or certify flight simulation.")


if __name__ == "__main__":
    main()
