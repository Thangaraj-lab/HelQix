"""Versioned component type library used by the Reality Studio builder.

Entries describe structural classifications only. They are not vendor parts,
engineering recommendations, parameter defaults, or proof of model support.
"""

from __future__ import annotations

from functools import lru_cache
import re
from typing import Literal

from pydantic import Field, StrictStr

from .models import StrictDomainModel


class SupportedModel(StrictDomainModel):
    model_id: StrictStr = Field(min_length=1, max_length=200)
    version: StrictStr = Field(min_length=1, max_length=120)
    scope: StrictStr = Field(min_length=1, max_length=300)
    required_properties: list[StrictStr] = Field(default_factory=list, max_length=32)


class ComponentTypeDefinition(StrictDomainModel):
    schema_version: Literal["helqix.component-type.v1"] = Field(alias="schema")
    id: StrictStr = Field(pattern=r"^[a-z][a-z0-9_]{1,79}$")
    label: StrictStr = Field(min_length=1, max_length=100)
    category: StrictStr = Field(min_length=1, max_length=80)
    domains: list[StrictStr] = Field(min_length=1, max_length=12)
    description: StrictStr = Field(min_length=1, max_length=300)
    supported_models: list[SupportedModel] = Field(default_factory=list, max_length=8)


_UNIVERSAL = (
    ("structural_member", "Structural member", "Structure", "A load-bearing or frame element. Geometry and material properties must be declared separately."),
    ("enclosure", "Enclosure", "Structure", "A protective housing or casing. Dimensions, material, and environmental ratings are not supplied."),
    ("electrical_power_source", "Electrical power source", "Electrical", "A source of electrical energy. Chemistry, topology, and ratings remain project inputs."),
    ("battery_cell", "Battery cell", "Electrical", "A single electrochemical cell. The Chen2020 model is applicable only when its cell parameterization matches the physical cell; chemistry is not inferred."),
    ("lithium_ion_cell", "Lithium-ion cell", "Electrical", "A single lithium-ion cell. Chen2020 applicability and input ranges must match the physical cell; the model is not pack-level."),
    ("battery_pack", "Battery pack", "Electrical", "An assembly of cells and interconnects. Pack behavior requires a compatible pack-level model, which is not currently provided."),
    ("electric_motor", "Electric motor", "Actuation", "An electromechanical actuator. Motor constants, thermal behavior, and performance are not inferred."),
    ("motor_controller", "Motor controller", "Control", "A controller or power-electronics assembly. Control laws and switching models must be supplied separately."),
    ("sensor", "Sensor", "Sensing", "A measurement device. Range, accuracy, bandwidth, and calibration are not assumed."),
    ("actuator", "Actuator", "Actuation", "A device that produces controlled motion or force. Its constitutive model is not included."),
    ("fluid_pump", "Fluid pump", "Fluid systems", "A pump component. Flow curves, fluid properties, and operating limits must be declared or linked."),
    ("heat_exchanger", "Heat exchanger", "Thermal", "A thermal transfer component. Geometry, materials, and transfer coefficients are not assumed."),
    ("custom_component", "Custom component", "Other", "A user-defined component type. Add explicit properties and link validated representations as needed."),
)

_DOMAIN_TYPES: dict[str, tuple[tuple[str, str, str, str], ...]] = {
    "battery": (
        ("battery_module", "Battery module", "Electrical", "A grouped cell assembly. Interconnect and thermal behavior require explicit models."),
        ("battery_management_system", "Battery management system", "Control", "A battery monitoring/control subsystem. Protection logic is not generated."),
        ("thermal_management_system", "Thermal management system", "Thermal", "A subsystem for battery heat transfer. No thermal solution is implied by this type."),
    ),
    "vehicle": (
        ("vehicle_body", "Vehicle body", "Structure", "A body or chassis assembly. This type does not include structural CAD or crash analysis."),
        ("traction_inverter", "Traction inverter", "Electrical", "A traction power-electronics component. Switching and efficiency models are not supplied."),
        ("wheel_tire_assembly", "Wheel and tire assembly", "Vehicle systems", "A rolling assembly. Tire force behavior and road interaction require separate models."),
        ("vehicle_thermal_system", "Vehicle thermal system", "Thermal", "A thermal subsystem. Cooling performance remains unmodeled until linked to a solver."),
    ),
    "drone": (
        ("airframe", "Airframe", "Structure", "An aircraft structural assembly. Geometry, materials, fatigue, and airworthiness are not supplied."),
        ("propeller", "Propeller", "Propulsion", "A rotor/propeller component. Thrust and aerodynamic performance require validated data or a solver."),
        ("flight_controller", "Flight controller", "Control", "An onboard flight-control computer. Firmware and control behavior are not inferred."),
        ("electronic_speed_controller", "Electronic speed controller", "Electrical", "A motor drive component. Electrical and thermal behavior require a compatible model."),
        ("power_distribution_board", "Power distribution board", "Electrical", "An electrical distribution component. Wiring, current limits, and losses must be declared."),
    ),
    "industrial": (
        ("process_pump", "Process pump", "Fluid systems", "A pump in a process system. Pump curves and fluid conditions are not supplied."),
        ("process_motor", "Process motor", "Actuation", "A motor in an industrial system. Nameplate values and dynamic behavior are not inferred."),
        ("heat_exchanger", "Heat exchanger", "Thermal", "A thermal transfer component. Geometry and heat-transfer performance are not assumed."),
    ),
}


def _normalize_domain(domain: str | None) -> str:
    value = re.sub(r"[^a-z0-9]+", "_", (domain or "").strip().lower()).strip("_")
    tokens = set(value.split("_"))
    if tokens.intersection({"drone", "uav", "quadrotor", "aircraft"}):
        return "drone"
    if tokens.intersection({"vehicle", "automotive", "ev"}) or "electric_vehicle" in value or "electric_car" in value:
        return "vehicle"
    if tokens.intersection({"battery", "cell", "energy", "storage"}) or "energy_storage" in value:
        return "battery"
    if tokens.intersection({"industrial", "pump", "process"}):
        return "industrial"
    return "general"


@lru_cache(maxsize=1)
def _universal_definitions() -> tuple[ComponentTypeDefinition, ...]:
    return tuple(ComponentTypeDefinition(
        schema="helqix.component-type.v1", id=identifier, label=label,
        category=category, domains=["general"], description=description,
        supported_models=[SupportedModel(
            model_id="pybamm.lithium-ion.spm", version="Chen2020",
            scope="single-cell discharge only; use only when the physical cell matches the Chen2020 parameterization; not pack- or product-level",
            required_properties=["current_a", "cutoff_voltage_v"],
        )] if identifier in {"battery_cell", "lithium_ion_cell"} else [],
    ) for identifier, label, category, description in _UNIVERSAL)


def component_type_catalog(domain: str | None = None) -> dict[str, object]:
    """Return safe type definitions for a domain, never supplier components."""
    resolved = _normalize_domain(domain)
    definitions = list(_universal_definitions())
    for identifier, label, category, description in _DOMAIN_TYPES.get(resolved, ()):
        if any(item.id == identifier for item in definitions):
            continue
        definitions.append(ComponentTypeDefinition(
            schema="helqix.component-type.v1", id=identifier, label=label,
            category=category, domains=[resolved], description=description,
        ))
    return {
        "schema": "helqix.component-type-catalog.v1",
        "domain": resolved,
        "entries": [item.model_dump(by_alias=True, mode="json") for item in definitions],
        "scope": "Type classifications only; no vendor parts, specification defaults, recommendations, or unlisted solver support.",
    }
