"""Shared structural contracts for user-authored quadrotor design drafts.

These bounds catch malformed or out-of-range input only. They do not establish
physical feasibility, airworthiness, or performance.
"""

from __future__ import annotations

import math
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, StrictFloat, StrictInt, StrictStr, field_validator, model_validator


def _camel_case(name: str) -> str:
    first, *rest = name.split("_")
    return first + "".join(part[:1].upper() + part[1:] for part in rest)


class StrictDomainModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        populate_by_name=True,
        alias_generator=_camel_case,
        str_strip_whitespace=True,
    )


class DroneSpecification(StrictDomainModel):
    schema_version: Literal["helqix.drone-specification.v1"] = Field(alias="schema")
    aircraft: Literal["quadrotor"]
    frame_diagonal_mm: float = Field(ge=100, le=2000, allow_inf_nan=False)
    dry_mass_kg: float = Field(gt=0, le=100, allow_inf_nan=False)
    payload_mass_kg: float = Field(ge=0, le=100, allow_inf_nan=False)
    battery_mass_kg: float = Field(gt=0, le=50, allow_inf_nan=False)
    battery_capacity_mah: float = Field(ge=100, le=100000, allow_inf_nan=False)
    battery_voltage_v: float = Field(ge=3, le=100, allow_inf_nan=False)
    propeller_diameter_in: float = Field(ge=2, le=40, allow_inf_nan=False)
    motor_kv_rpm_per_volt: float = Field(ge=10, le=5000, allow_inf_nan=False)


class ScenarioParameters(StrictDomainModel):
    battery_mass_kg: float = Field(gt=0, le=50, allow_inf_nan=False)
    # Optional for legacy scenarios. Required by the UI for newly authored battery variants.
    # Capacity and voltage are user inputs; neither is inferred from mass.
    battery_capacity_mah: float | None = Field(default=None, ge=100, le=100000, allow_inf_nan=False)
    battery_voltage_v: float | None = Field(default=None, ge=3, le=100, allow_inf_nan=False)
    payload_mass_kg: float = Field(ge=0, le=100, allow_inf_nan=False)
    wind_speed_m_s: float = Field(ge=0, le=100, allow_inf_nan=False)


class DroneScenario(StrictDomainModel):
    schema_version: Literal["helqix.drone-scenario.v1"] = Field(alias="schema")
    name: str = Field(min_length=1, max_length=100)
    description: str = Field(max_length=500)
    base_specification: DroneSpecification
    parameters: ScenarioParameters
    execution_state: Literal["not_run"]

    @field_validator("name")
    @classmethod
    def name_is_nonblank(cls, value: str) -> str:
        if not value:
            raise ValueError("scenario name must not be blank")
        return value


class BoundedSearchCandidate(StrictDomainModel):
    current_a: StrictInt | StrictFloat
    state: Literal["completed", "failed"]
    branch_id: StrictStr | None = None
    run_id: StrictStr | None = None
    duration_h: StrictInt | StrictFloat | None = None
    discharged_capacity_ah: StrictInt | StrictFloat | None = None
    final_voltage_v: StrictInt | StrictFloat | None = None
    error: StrictStr | None = Field(default=None, max_length=1000)

    @field_validator("current_a")
    @classmethod
    def current_in_model_range(cls, value: float) -> float:
        if not math.isfinite(value) or not 0.05 <= value <= 100:
            raise ValueError("candidate current must be finite and between 0.05 and 100 A")
        return value

    @model_validator(mode="after")
    def result_matches_state(self) -> "BoundedSearchCandidate":
        metrics = (self.duration_h, self.discharged_capacity_ah, self.final_voltage_v)
        if self.state == "completed":
            if not self.run_id or any(value is None or not math.isfinite(value) for value in metrics):
                raise ValueError("completed candidates require a run id and finite solver metrics")
            if any(value <= 0 for value in metrics if value is not None):
                raise ValueError("completed candidate solver metrics must be positive")
            if self.error is not None:
                raise ValueError("completed candidates cannot contain an error")
        elif not self.error:
            raise ValueError("failed candidates must preserve an error message")
        return self


class BoundedSearch(StrictDomainModel):
    schema_version: Literal["helqix.bounded-search.v1"] = Field(alias="schema")
    name: StrictStr = Field(min_length=1, max_length=160)
    status: Literal["completed", "partial", "failed"]
    base_reality_revision: StrictInt = Field(ge=1, le=2_147_483_647)
    binding_id: StrictStr = Field(min_length=1, max_length=128)
    variable: dict
    objective: dict
    constraint: dict
    candidate_count: StrictInt = Field(ge=2, le=5)
    completed_count: StrictInt = Field(ge=0, le=5)
    best_evaluated_candidate_a: StrictInt | StrictFloat | None = None
    candidates: list[BoundedSearchCandidate] = Field(min_length=2, max_length=5)
    applicability: StrictStr = Field(min_length=1, max_length=1000)

    @model_validator(mode="after")
    def search_is_self_consistent(self) -> "BoundedSearch":
        variable = self.variable
        if variable != {"component_id": variable.get("component_id"), "property_name": "current_a",
                        "unit": "A", "values": variable.get("values")}:
            raise ValueError("search variable must declare a component, current_a, unit A, and candidate values")
        if not isinstance(variable.get("component_id"), str) or not variable["component_id"]:
            raise ValueError("search variable component_id is required")
        values = variable.get("values")
        if (not isinstance(values, list) or len(values) != self.candidate_count
                or any(isinstance(value, bool) or not isinstance(value, (int, float))
                       or not math.isfinite(float(value)) or not 0.05 <= value <= 100 for value in values)
                or len(set(values)) != len(values)):
            raise ValueError("search values must be unique finite currents matching candidate_count")
        if self.objective != {"metric": "discharged_capacity_ah", "direction": "maximize"}:
            raise ValueError("supported objective is maximize discharged_capacity_ah")
        if (not isinstance(self.constraint, dict) or set(self.constraint) != {"metric", "operator", "value_h"}
                or self.constraint.get("metric") != "duration_h" or self.constraint.get("operator") != ">="):
            raise ValueError("supported constraint must be duration_h >= an optional value_h")
        minimum_duration = self.constraint.get("value_h")
        if minimum_duration is not None and (isinstance(minimum_duration, bool)
                or not isinstance(minimum_duration, (int, float)) or not math.isfinite(float(minimum_duration))
                or minimum_duration < 0):
            raise ValueError("minimum duration must be a finite nonnegative number or null")
        if [candidate.current_a for candidate in self.candidates] != values:
            raise ValueError("candidate results must preserve the declared candidate order and values")
        completed = [candidate for candidate in self.candidates if candidate.state == "completed"]
        if self.completed_count != len(completed):
            raise ValueError("completed_count must match candidate states")
        expected_status = "completed" if len(completed) == len(self.candidates) else "partial" if completed else "failed"
        if self.status != expected_status:
            raise ValueError("search status must match candidate execution states")
        eligible = [candidate for candidate in completed
                    if minimum_duration is None or candidate.duration_h >= minimum_duration]
        best = max(eligible, key=lambda candidate: candidate.discharged_capacity_ah) if eligible else None
        expected_best = best.current_a if best else None
        if self.best_evaluated_candidate_a != expected_best:
            raise ValueError("best evaluated current must match the declared objective and constraint")
        return self


class ExplicitVerificationCheck(StrictDomainModel):
    schema_version: Literal["helqix.verification-check.v1"] = Field(alias="schema")
    requirement_id: StrictStr = Field(min_length=1, max_length=128)
    code: StrictStr = Field(min_length=1, max_length=128)
    requirement_title: StrictStr = Field(alias="requirementTitle", min_length=1, max_length=2000)
    governing_standard: StrictStr = Field(alias="governingStandard", min_length=1, max_length=500)
    condition_formula: StrictStr = Field(alias="conditionFormula", min_length=1, max_length=300)
    metric: Literal["duration_h", "discharged_capacity_ah", "final_voltage_v"]
    operator: Literal[">=", "<=", ">", "<", "=="]
    threshold: StrictInt | StrictFloat
    unit: Literal["h", "Ah", "V"]
    observed_value: StrictInt | StrictFloat | None = Field(default=None, alias="observedMetricValue")
    observed_label: StrictStr = Field(alias="observedValue", min_length=1, max_length=200)
    limit_label: StrictStr = Field(alias="limitThreshold", min_length=1, max_length=200)
    margin_percent: StrictInt | StrictFloat | None = Field(alias="marginPercent", default=None)
    verdict: Literal["pass", "fail", "inconclusive"]
    method: StrictStr = Field(min_length=1, max_length=500)
    source_run_id: StrictStr = Field(min_length=1, max_length=128)
    source_model_sha256: StrictStr | None = Field(default=None, pattern=r"^[0-9a-f]{64}$")
    current_model_sha256: StrictStr | None = Field(default=None, pattern=r"^[0-9a-f]{64}$")

    @model_validator(mode="after")
    def verdict_is_supported_by_evidence(self) -> "ExplicitVerificationCheck":
        expected_unit = {"duration_h": "h", "discharged_capacity_ah": "Ah", "final_voltage_v": "V"}[self.metric]
        if self.unit != expected_unit:
            raise ValueError(f"{self.metric} requires unit {expected_unit}")
        threshold = float(self.threshold)
        if not math.isfinite(threshold):
            raise ValueError("verification threshold must be finite")
        if self.margin_percent is not None and not math.isfinite(float(self.margin_percent)):
            raise ValueError("verification margin must be finite")
        if self.observed_value is None:
            if self.verdict != "inconclusive":
                raise ValueError("checks without an observed solver value must be inconclusive")
            return self
        observed = float(self.observed_value)
        if not math.isfinite(observed):
            raise ValueError("observed solver value must be finite")
        if not self.source_model_sha256 or self.source_model_sha256 != self.current_model_sha256:
            raise ValueError("conclusive checks require matching source and current model digests")
        operators = {
            ">=": observed >= threshold, "<=": observed <= threshold, ">": observed > threshold,
            "<": observed < threshold, "==": math.isclose(observed, threshold, rel_tol=1e-9, abs_tol=1e-12),
        }
        expected_verdict = "pass" if operators[self.operator] else "fail"
        if self.verdict != expected_verdict:
            raise ValueError("verification verdict does not match the stored observation and rule")
        return self


class ProofLoopPlan(StrictDomainModel):
    """A user-recorded next engineering action with its recommendation provenance."""
    schema_version: Literal["helqix.proofloop-plan.v1"] = Field(alias="schema")
    status: Literal["planned"]
    action_id: StrictStr = Field(min_length=1, max_length=100)
    title: StrictStr = Field(min_length=1, max_length=300)
    reason: StrictStr = Field(min_length=1, max_length=2000)
    detail: StrictStr = Field(min_length=1, max_length=3000)
    destination: Literal["architecture", "model", "integrations", "execution", "verify", "evidence"]
    priority: Literal["required", "recommended"]
    evidence_refs: list[StrictStr] = Field(max_length=100)
    limitations: list[StrictStr] = Field(min_length=1, max_length=20)
    model_sha256: StrictStr | None = Field(default=None, pattern=r"^[0-9a-f]{64}$")
    project_revision: StrictInt = Field(ge=1)
    recorded_at: StrictStr = Field(min_length=20, max_length=40)


class EVDriveCyclePoint(StrictDomainModel):
    time_s: float = Field(ge=0, le=3600, allow_inf_nan=False)
    speed_m_s: float = Field(ge=0, le=60, allow_inf_nan=False)
    road_grade_percent: float = Field(default=0, ge=-20, le=20, allow_inf_nan=False)


class EVBatteryPack(StrictDomainModel):
    series_cells: int = Field(ge=1, le=200)
    parallel_cells: int = Field(ge=1, le=100)
    cell_capacity_ah: float = Field(ge=0.5, le=50, allow_inf_nan=False)
    cell_nominal_voltage_v: float = Field(gt=2.5, le=4.5, allow_inf_nan=False)
    cell_cutoff_voltage_v: float = Field(ge=2.5, lt=4.3, allow_inf_nan=False)
    cell_max_continuous_current_a: float = Field(gt=0, le=500, allow_inf_nan=False)
    initial_soc: float = Field(gt=0, le=1, allow_inf_nan=False)

    @model_validator(mode="after")
    def cutoff_below_nominal(self) -> "EVBatteryPack":
        if self.cell_cutoff_voltage_v >= self.cell_nominal_voltage_v:
            raise ValueError("cell cutoff voltage must be lower than nominal voltage")
        return self


class EVBatteryVehicleConfiguration(StrictDomainModel):
    """Explicit vehicle and pack inputs for the bounded HELQIX EV workflow."""
    schema_version: Literal["helqix.ev-battery-vehicle.v1"] = Field(alias="schema")
    vehicle_name: StrictStr = Field(min_length=1, max_length=120)
    vehicle_mass_kg: float = Field(gt=100, le=10000, allow_inf_nan=False, description="Total running mass including battery and occupants")
    drag_coefficient: float = Field(gt=0.05, le=1.5, allow_inf_nan=False)
    frontal_area_m2: float = Field(gt=0.3, le=12, allow_inf_nan=False)
    rolling_resistance_coefficient: float = Field(ge=0.001, le=0.08, allow_inf_nan=False)
    drivetrain_efficiency: float = Field(gt=0.5, le=1, allow_inf_nan=False)
    accessory_power_w: float = Field(ge=0, le=10000, allow_inf_nan=False)
    battery_pack: EVBatteryPack
    drive_cycle: list[EVDriveCyclePoint] = Field(min_length=2, max_length=200)
    model_id: Literal["pybamm.lithium-ion.spm"] = "pybamm.lithium-ion.spm"
    parameter_set: Literal["Chen2020"] = "Chen2020"

    @model_validator(mode="after")
    def drive_cycle_is_ordered_and_resolvable(self) -> "EVBatteryVehicleConfiguration":
        times = [point.time_s for point in self.drive_cycle]
        if times[0] != 0:
            raise ValueError("drive cycle must start at time zero")
        if any(right - left < 1 for left, right in zip(times, times[1:])):
            raise ValueError("drive-cycle points must be at least one second apart")
        return self


def validate_project_payload(payload: dict) -> dict:
    """Validate shared product reality and optional drone-domain draft fields."""
    if "engineering_reality" in payload:
        from .reality import EngineeringReality
        EngineeringReality.model_validate(payload["engineering_reality"])
    if "specification" in payload:
        specification = payload["specification"]
        if isinstance(specification, dict) and "schema" not in specification:
            specification = {**specification, "schema": "helqix.drone-specification.v1"}
        DroneSpecification.model_validate(specification)
        payload = {**payload, "specification": specification}
    if "ev_vehicle" in payload:
        validated_ev = EVBatteryVehicleConfiguration.model_validate(payload["ev_vehicle"])
        payload = {**payload, "ev_vehicle": validated_ev.model_dump(by_alias=True, mode="json")}
    return payload


def validate_resource_payload(resource: str, payload: dict) -> dict:
    if resource == "projects":
        return validate_project_payload(payload)
    elif resource == "branches" and payload.get("schema") == "helqix.drone-scenario.v1":
        DroneScenario.model_validate(payload)
    elif resource == "branches" and payload.get("schema") == "helqix.engineering-branch.v1":
        from .reality import EngineeringBranch
        EngineeringBranch.model_validate(payload)
    elif resource == "searches" and payload.get("schema") == "helqix.bounded-search.v1":
        BoundedSearch.model_validate(payload)
    elif resource == "verifications" and payload.get("schema") == "helqix.verification-check.v1":
        ExplicitVerificationCheck.model_validate(payload)
    elif resource == "proofloops" and payload.get("schema") == "helqix.proofloop-plan.v1":
        ProofLoopPlan.model_validate(payload)
    return payload
