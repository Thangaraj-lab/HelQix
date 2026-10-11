"""Versioned components and component identities for HELQIX realities."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Mapping

from .reality import JsonValue, RealityValidationError, _freeze_json, _is_json_value


@dataclass(frozen=True, slots=True)
class ComponentRef:
    id: str
    type: str
    data: JsonValue | None = None

    def __post_init__(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if not _is_json_value(self.data):
            raise RealityValidationError([("data", "must contain only finite JSON values")])
        object.__setattr__(self, "data", _freeze_json(self.data))

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"id": self.id, "type": self.type}
        if self.data is not None:
            result["data"] = _thaw_json(self.data)
        return result

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> ComponentRef:
        _require_mapping_keys(payload, {"id", "type"}, {"data"}, "component reference")
        return cls(payload["id"], payload["type"], payload.get("data"))


@dataclass(frozen=True, slots=True)
class Component:
    id: str
    type: str
    version: int
    data: JsonValue
    metadata: Mapping[str, JsonValue] = field(default_factory=dict)

    def __post_init__(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if isinstance(self.version, bool) or not isinstance(self.version, int) or self.version < 1:
            raise RealityValidationError([("version", "must be a positive integer")])
        if not _is_json_value(self.data):
            raise RealityValidationError([("data", "must contain only finite JSON values")])
        if not isinstance(self.metadata, Mapping) or not _is_json_value(dict(self.metadata)):
            raise RealityValidationError([("metadata", "must contain only JSON values with string keys")])
        object.__setattr__(self, "data", _freeze_json(self.data))
        object.__setattr__(self, "metadata", _freeze_json(dict(self.metadata)))

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "type": self.type,
            "version": self.version,
            "data": _thaw_json(self.data),
            "metadata": _thaw_json(self.metadata),
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> Component:
        _require_mapping_keys(payload, {"id", "type", "version", "data"}, {"metadata"}, "component")
        metadata = payload.get("metadata", {})
        if not isinstance(metadata, Mapping):
            raise RealityValidationError([("metadata", "must be an object")])
        return cls(payload["id"], payload["type"], payload["version"], payload["data"], metadata)


@dataclass(frozen=True, slots=True)
class ComponentKey:
    owner_id: str
    component_id: str

    def __post_init__(self) -> None:
        _require_token("owner_id", self.owner_id)
        _require_token("component_id", self.component_id)

    def to_dict(self) -> dict[str, str]:
        return {"ownerId": self.owner_id, "componentId": self.component_id}


def _require_token(name: str, value: Any) -> None:
    if not isinstance(value, str) or not value.strip() or value.strip() != value:
        raise RealityValidationError([(name, "must be a non-empty trimmed string")])


def _require_mapping_keys(payload: Mapping[str, Any], required: set[str], optional: set[str], label: str) -> None:
    if not isinstance(payload, Mapping):
        raise RealityValidationError([(label, "must be an object")])
    missing = required.difference(payload)
    unknown = set(payload).difference(required | optional)
    issues = [(f"{label}.{key}", "is required") for key in sorted(missing)]
    issues.extend((f"{label}.{key}", "is not supported") for key in sorted(unknown))
    if issues:
        raise RealityValidationError(issues)


def _thaw_json(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {key: _thaw_json(item) for key, item in value.items()}
    if isinstance(value, tuple):
        return [_thaw_json(item) for item in value]
    return value
