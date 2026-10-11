"""Behaviour declarations for HELQIX realities.

These classes describe behavior configuration only; they never execute it.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Mapping

from .reality import JsonValue, RealityValidationError, _freeze_json, _is_json_value


@dataclass(frozen=True, slots=True)
class BehaviourRef:
    id: str
    type: str
    config: JsonValue | None = None

    def __post_init__(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if not _is_json_value(self.config):
            raise RealityValidationError([("config", "must contain only finite JSON values")])
        object.__setattr__(self, "config", _freeze_json(self.config))

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"id": self.id, "type": self.type}
        if self.config is not None:
            result["config"] = _thaw_json(self.config)
        return result

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> BehaviourRef:
        _require_mapping_keys(payload, {"id", "type"}, {"config"}, "behaviour reference")
        return cls(payload["id"], payload["type"], payload.get("config"))


@dataclass(frozen=True, slots=True)
class BehaviourDefinition:
    id: str
    type: str
    version: int
    config: JsonValue
    description: str | None = None

    def __post_init__(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if isinstance(self.version, bool) or not isinstance(self.version, int) or self.version < 1:
            raise RealityValidationError([("version", "must be a positive integer")])
        if self.description is not None and (not isinstance(self.description, str) or not self.description.strip()):
            raise RealityValidationError([("description", "must not be empty when provided")])
        if not _is_json_value(self.config):
            raise RealityValidationError([("config", "must contain only finite JSON values")])
        object.__setattr__(self, "config", _freeze_json(self.config))

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "id": self.id,
            "type": self.type,
            "version": self.version,
            "config": _thaw_json(self.config),
        }
        if self.description is not None:
            result["description"] = self.description
        return result

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> BehaviourDefinition:
        _require_mapping_keys(payload, {"id", "type", "version", "config"}, {"description"}, "behaviour definition")
        return cls(payload["id"], payload["type"], payload["version"], payload["config"], payload.get("description"))


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
