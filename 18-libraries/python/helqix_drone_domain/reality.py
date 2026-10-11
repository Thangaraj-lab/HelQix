"""Domain-neutral, versioned contract for an engineering reality.

The contract describes declared product structure and its links to models. It
does not assert physical feasibility, model validity, or verification status.
"""

from __future__ import annotations

import math
from typing import Literal

from pydantic import Field, StrictBool, StrictFloat, StrictInt, StrictStr, model_validator

from .models import StrictDomainModel


EngineeringValue = StrictBool | StrictInt | StrictFloat | StrictStr


class EngineeringProperty(StrictDomainModel):
    name: str = Field(min_length=1, max_length=120)
    value: EngineeringValue
    unit: str | None = Field(default=None, min_length=1, max_length=40)
    origin: Literal["user", "catalog", "import", "derived", "model"]
    source_ref: str | None = Field(default=None, max_length=500)


class RealityComponent(StrictDomainModel):
    id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    name: str = Field(min_length=1, max_length=160)
    component_type: str = Field(min_length=1, max_length=160)
    properties: list[EngineeringProperty] = Field(default_factory=list, max_length=500)


class RealityInterface(StrictDomainModel):
    id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    component_id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    name: str = Field(min_length=1, max_length=160)
    direction: Literal["in", "out", "bidirectional"]
    domain: str = Field(min_length=1, max_length=80)


class RealityConnection(StrictDomainModel):
    id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    source_interface_id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    target_interface_id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    connection_type: str = Field(min_length=1, max_length=100)


class VerificationRule(StrictDomainModel):
    metric: Literal["duration_h", "discharged_capacity_ah", "final_voltage_v"]
    operator: Literal[">=", "<=", ">", "<", "=="]
    threshold: StrictInt | StrictFloat
    unit: Literal["h", "Ah", "V"]

    @model_validator(mode="after")
    def threshold_and_unit_are_valid(self) -> "VerificationRule":
        if not math.isfinite(float(self.threshold)):
            raise ValueError("verification threshold must be finite")
        expected = {"duration_h": "h", "discharged_capacity_ah": "Ah", "final_voltage_v": "V"}[self.metric]
        if self.unit != expected:
            raise ValueError(f"metric '{self.metric}' requires unit '{expected}'")
        return self


class EngineeringBranchChange(StrictDomainModel):
    component_id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    property_name: str = Field(min_length=1, max_length=120)
    value: EngineeringValue
    unit: str | None = Field(default=None, max_length=40)


class EngineeringBranch(StrictDomainModel):
    schema_version: Literal["helqix.engineering-branch.v1"] = Field(alias="schema")
    name: str = Field(min_length=1, max_length=120)
    description: str = Field(default="", max_length=1000)
    base_reality_revision: int = Field(ge=1, le=2_147_483_647)
    changes: list[EngineeringBranchChange] = Field(min_length=1, max_length=200)

    @model_validator(mode="after")
    def one_change_per_property(self) -> "EngineeringBranch":
        keys = [(change.component_id, change.property_name) for change in self.changes]
        if len(keys) != len(set(keys)):
            raise ValueError("a branch may change a component property only once")
        return self


class RealityRequirement(StrictDomainModel):
    id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    statement: str = Field(min_length=1, max_length=2000)
    priority: Literal["must", "should", "could"]
    review_status: Literal["draft", "accepted"] = "draft"
    source_ref: str | None = Field(default=None, max_length=500)
    verification_rule: VerificationRule | None = None


class ModelBinding(StrictDomainModel):
    id: str = Field(pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    model_id: str = Field(min_length=1, max_length=200)
    model_version: str = Field(min_length=1, max_length=100)
    scope_component_id: str | None = Field(default=None, pattern=r"^[A-Za-z][A-Za-z0-9._:-]{0,127}$")
    declared_fidelity: str = Field(min_length=1, max_length=200)


class EngineeringReality(StrictDomainModel):
    """A product/system definition shared by domain packs and platform layers."""

    schema_version: Literal["helqix.engineering-reality.v1"] = Field(alias="schema")
    name: str = Field(min_length=1, max_length=200)
    domain: str = Field(pattern=r"^[a-z][a-z0-9._-]{0,79}$")
    description: str = Field(default="", max_length=4000)
    revision: int = Field(default=1, ge=1, le=2_147_483_647)
    units_system: Literal["SI", "imperial", "custom"] = "SI"
    components: list[RealityComponent] = Field(default_factory=list, max_length=10000)
    interfaces: list[RealityInterface] = Field(default_factory=list, max_length=50000)
    connections: list[RealityConnection] = Field(default_factory=list, max_length=100000)
    requirements: list[RealityRequirement] = Field(default_factory=list, max_length=10000)
    model_bindings: list[ModelBinding] = Field(default_factory=list, max_length=10000)

    @model_validator(mode="after")
    def references_are_resolvable(self) -> "EngineeringReality":
        components = {component.id for component in self.components}
        interfaces = {interface.id: interface for interface in self.interfaces}
        all_record_ids = [
            *(component.id for component in self.components),
            *(interface.id for interface in self.interfaces),
            *(connection.id for connection in self.connections),
            *(requirement.id for requirement in self.requirements),
            *(binding.id for binding in self.model_bindings),
        ]
        if len(set(all_record_ids)) != len(all_record_ids):
            raise ValueError("record IDs must be unique across an engineering reality")

        for component in self.components:
            property_names = [prop.name for prop in component.properties]
            if len(set(property_names)) != len(property_names):
                raise ValueError(f"component '{component.id}' has duplicate property names")

        for interface in self.interfaces:
            if interface.component_id not in components:
                raise ValueError(f"interface '{interface.id}' references an unknown component")
        for connection in self.connections:
            if connection.source_interface_id not in interfaces:
                raise ValueError(f"connection '{connection.id}' references an unknown source interface")
            if connection.target_interface_id not in interfaces:
                raise ValueError(f"connection '{connection.id}' references an unknown target interface")
            source = interfaces[connection.source_interface_id]
            target = interfaces[connection.target_interface_id]
            if source.direction == "in":
                raise ValueError(f"connection '{connection.id}' uses an input-only interface as its source")
            if target.direction == "out":
                raise ValueError(f"connection '{connection.id}' uses an output-only interface as its target")
            if source.domain.casefold() != target.domain.casefold():
                raise ValueError(f"connection '{connection.id}' links incompatible interface domains '{source.domain}' and '{target.domain}'")
        for binding in self.model_bindings:
            if binding.scope_component_id is not None and binding.scope_component_id not in components:
                raise ValueError(f"model binding '{binding.id}' references an unknown component")
        return self
