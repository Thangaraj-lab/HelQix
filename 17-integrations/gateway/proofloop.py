"""Evidence-aware next-step policy for the HELQIX ProofLoop workflow.

This is a transparent rules engine, not an AI design authority. Every returned
recommendation names the records that triggered it and declares its limits.
"""
from __future__ import annotations

from typing import Any


def recommend_next_step(
    *, project_id: str, reality: dict[str, Any] | None, current_model_sha256: str | None,
    runs: list[dict[str, Any]], checks: list[dict[str, Any]],
) -> dict[str, Any]:
    """Choose the next proof obligation from persisted project state."""
    current_runs = [run for run in runs
                    if run.get("tool") == "pybamm" and run.get("operation") == "simulate_bound_cell"]
    latest_run = current_runs[0] if current_runs else None  # Caller orders newest first.
    accepted_rules = [item for item in (reality or {}).get("requirements", [])
                      if item.get("review_status") == "accepted" and item.get("verification_rule")]
    cell_ids = {component.get("id") for component in (reality or {}).get("components", [])
                if str(component.get("component_type", "")).lower() in {"battery_cell", "lithium_ion_cell"}}
    is_cell_binding = any(
        str(binding.get("model_id", "")).lower() == "pybamm.lithium-ion.spm"
        and str(binding.get("model_version", "")) == "Chen2020"
        and binding.get("scope_component_id") in cell_ids
        for binding in (reality or {}).get("model_bindings", [])
    )
    component_types = {str(component.get("component_type", "")).lower()
                       for component in (reality or {}).get("components", [])}
    has_cell = bool(component_types & {"battery_cell", "lithium_ion_cell"})
    model_evidence = [f"project:{project_id}"]
    if current_model_sha256:
        model_evidence.append(f"engineering-reality-sha256:{current_model_sha256}")

    def action(action_id: str, title: str, reason: str, detail: str, destination: str,
               evidence: list[str], limits: list[str], priority: str = "required") -> dict[str, Any]:
        return {
            "schema": "helqix.proofloop-recommendation.v1", "project_id": project_id,
            "action_id": action_id, "title": title, "reason": reason, "detail": detail,
            "destination": destination, "priority": priority, "evidence_refs": evidence,
            "limitations": limits,
        }

    if not reality:
        return action("define_reality", "Define the engineering reality",
                      "This project has no saved typed engineering reality to evaluate.",
                      "Add the product components, interfaces, units, requirements, and applicable model bindings. Review every imported or inferred value.",
                      "architecture", model_evidence, ["No product behavior has been simulated or verified."])
    if not has_cell or not is_cell_binding:
        return action("bind_applicable_model", "Connect an applicable engineering model",
                      "The saved reality does not contain a battery cell with a PyBaMM SPM / Chen2020 binding.",
                      "Choose a model that matches the component and its declared scope. HELQIX currently executes only the bounded single-cell PyBaMM path; it cannot evaluate a battery pack or vehicle here.",
                      "model", model_evidence,
                      ["The available PyBaMM adapter is single-cell only.", "No pack, BMS, inverter, motor, or vehicle behavior is established."])
    if not accepted_rules:
        return action("define_acceptance_criteria", "Set an explicit measurable requirement",
                      "There are no accepted requirements with supported numeric verification rules.",
                      "Add a measurable threshold such as minimum discharge duration or minimum discharged capacity, choose its unit and acceptance rule, and review it before running verification.",
                      "architecture", model_evidence,
                      ["A threshold check covers only the named metric; it is not product certification."])
    latest_result = (latest_run or {}).get("result") or {}
    snapshot = (latest_run or {}).get("project_snapshot") or {}
    source_digest = (((snapshot.get("models") or {}).get("engineering_reality") or {}).get("sha256"))
    current_run = bool(latest_run and latest_result.get("status") == "completed"
                       and current_model_sha256 and source_digest == current_model_sha256)
    if not current_run:
        reason = ("No project-bound single-cell solver run is recorded." if not latest_run else
                  "The latest solver run is incomplete or belongs to a different engineering-reality revision.")
        return action("run_current_model", "Run the current cell model",
                      reason,
                      "Execute the saved PyBaMM SPM / Chen2020 binding using the current model revision, then review the recorded inputs and voltage/capacity outputs.",
                      "integrations", model_evidence + ([f"run:{latest_run['id']}"] if latest_run else []),
                      ["This run models one cell, not an assembled battery pack or vehicle."])

    run_id = latest_run["id"]
    current_checks = [check for check in checks if check.get("source_run_id") == run_id
                      and check.get("source_model_sha256") == current_model_sha256
                      and check.get("current_model_sha256") == current_model_sha256]
    checked_requirement_ids = {check.get("requirement_id") for check in current_checks}
    unchecked = [requirement for requirement in accepted_rules if requirement.get("id") not in checked_requirement_ids]
    if unchecked:
        return action("verify_current_run", "Check this run against accepted requirements",
                      f"The current run has no revision-matched check for {len(unchecked)} accepted requirement(s).",
                      "Evaluate the saved run against the project's explicit thresholds. HELQIX will record each observed value, verdict, run ID, and model digest.",
                      "execution", model_evidence + [f"run:{run_id}"],
                      ["Only supported numeric thresholds are evaluated.", "Passing checks do not establish pack or vehicle safety."])

    failed = [check for check in current_checks if check.get("verdict") == "fail"]
    inconclusive = [check for check in current_checks if check.get("verdict") == "inconclusive"]
    if failed or inconclusive:
        refs = model_evidence + [f"run:{run_id}"] + [f"requirement:{check.get('requirement_id')}" for check in failed + inconclusive]
        return action("investigate_failed_checks", "Investigate the unmet or inconclusive checks",
                      f"{len(failed)} requirement check(s) failed and {len(inconclusive)} are inconclusive for the current run.",
                      "Review the observed values, confirm model applicability and input provenance, then create a controlled branch or revise the model before rerunning. Do not treat a passing subset as overall approval.",
                      "verify", refs,
                      ["This policy does not diagnose root cause automatically.", "No hardware measurements are connected."])

    return action("plan_bench_validation", "Plan a supervised cell bench validation",
                  "All accepted thresholds recorded for the current model run pass; the next proof step is to compare the model with measured cell data.",
                  "Record the cell make/model/serial, calibration status, ambient and cell temperature, current profile, voltage samples, and test start/stop conditions. Compare measurements only after confirming matching units and test conditions.",
                  "evidence", model_evidence + [f"run:{run_id}"] + [f"requirement:{check.get('requirement_id')}" for check in current_checks],
                  ["HELQIX does not currently ingest live bench or CAN telemetry.",
                   "The current solver result is single-cell evidence and does not certify a pack or vehicle."],
                  priority="recommended")
