"""Versioned, transport-safe HELQIX reality documents."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from datetime import datetime
from types import MappingProxyType
from typing import Any, Mapping, TypeAlias

REALITY_SCHEMA_VERSION = 1
JsonPrimitive: TypeAlias = str | int | float | bool | None
JsonValue: TypeAlias = JsonPrimitive | list["JsonValue"] | dict[str, "JsonValue"]


class RealityValidationError(ValueError):
    """Raised when a reality payload violates the public SDK contract."""

    def __init__(self, issues: list[tuple[str, str]]) -> None:
        self.issues = tuple(issues)
        message = "; ".join(f"{path}: {detail}" for path, detail in issues)
        super().__init__(f"invalid reality document: {message}")


@dataclass(frozen=True, slots=True)
class RealityMetadata:
    name: str
    description: str | None = None
    tags: tuple[str, ...] = ()
    created_at: str | None = None
    updated_at: str | None = None
    extensions: Mapping[str, JsonValue] = field(default_factory=dict)

    def __post_init__(self) -> None:
        object.__setattr__(self, "tags", tuple(self.tags))
        object.__setattr__(self, "extensions", _freeze_json(dict(self.extensions)))
        self.validate()

    def validate(self) -> None:
        issues: list[tuple[str, str]] = []
        _validate_token("metadata.name", self.name, issues)
        if self.description is not None and not isinstance(self.description, str):
            issues.append(("metadata.description", "must be a string when provided"))
        if not isinstance(self.tags, tuple) or any(not isinstance(tag, str) for tag in self.tags):
            issues.append(("metadata.tags", "must be a sequence of strings"))
        for index, tag in enumerate(self.tags):
            if isinstance(tag, str):
                _validate_token(f"metadata.tags[{index}]", tag, issues)
        for name, value in (("created_at", self.created_at), ("updated_at", self.updated_at)):
            if value is not None and not _is_timestamp(value):
                issues.append((f"metadata.{name}", "must be an ISO-8601 timestamp with timezone"))
        if not isinstance(self.extensions, Mapping) or not _is_json_value(dict(self.extensions)):
            issues.append(("metadata.extensions", "must contain only JSON values"))
        if issues:
            raise RealityValidationError(issues)


@dataclass(frozen=True, slots=True)
class RealityDocument:
    id: str
    metadata: RealityMetadata
    components: tuple[Mapping[str, Any], ...] = ()
    behaviours: tuple[Mapping[str, Any], ...] = ()
    actions: tuple[Mapping[str, Any], ...] = ()
    schema_version: int = REALITY_SCHEMA_VERSION
    extensions: Mapping[str, JsonValue] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.metadata, RealityMetadata):
            raise TypeError("metadata must be a RealityMetadata instance")
        # Detach data from caller-owned mappings and make nested structures read-only.
        object.__setattr__(self, "components", tuple(_freeze_json(item) for item in self.components))
        object.__setattr__(self, "behaviours", tuple(_freeze_json(item) for item in self.behaviours))
        object.__setattr__(self, "actions", tuple(_freeze_json(item) for item in self.actions))
        object.__setattr__(self, "extensions", _freeze_json(dict(self.extensions)))
        self.validate()

    def validate(self) -> None:
        issues: list[tuple[str, str]] = []
        _validate_token("id", self.id, issues)
        if type(self.schema_version) is not int or self.schema_version != REALITY_SCHEMA_VERSION:
            issues.append(("schema_version", f"must equal {REALITY_SCHEMA_VERSION}"))
        try:
            self.metadata.validate()
        except RealityValidationError as error:
            issues.extend(error.issues)
        _validate_refs("components", self.components, "data", issues)
        _validate_refs("behaviours", self.behaviours, "config", issues)
        _validate_refs("actions", self.actions, "input", issues)
        if not _is_json_value(dict(self.extensions)):
            issues.append(("extensions", "must contain only JSON values"))
        if issues:
            raise RealityValidationError(issues)

    def to_dict(self) -> dict[str, Any]:
        """Return a detached JSON-compatible mapping."""
        metadata: dict[str, Any] = {
            "name": self.metadata.name,
            "tags": list(self.metadata.tags),
            "extensions": _thaw_json(self.metadata.extensions),
        }
        if self.metadata.description is not None:
            metadata["description"] = self.metadata.description
        if self.metadata.created_at is not None:
            metadata["createdAt"] = self.metadata.created_at
        if self.metadata.updated_at is not None:
            metadata["updatedAt"] = self.metadata.updated_at
        return {
            "schemaVersion": self.schema_version,
            "id": self.id,
            "metadata": metadata,
            "components": [_thaw_json(item) for item in self.components],
            "behaviours": [_thaw_json(item) for item in self.behaviours],
            "actions": [_thaw_json(item) for item in self.actions],
            "extensions": _thaw_json(self.extensions),
        }

    def to_json(self) -> str:
        self.validate()
        return json.dumps(self.to_dict(), ensure_ascii=False, allow_nan=False, separators=(",", ":"))

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> RealityDocument:
        if not isinstance(payload, Mapping):
            raise RealityValidationError([("$", "must be an object")])
        required = {"schemaVersion", "id", "metadata", "components"}
        optional = {"behaviours", "actions", "extensions"}
        missing = required.difference(payload)
        unknown = set(payload).difference(required | optional)
        issues = [(f"$.{key}", "is required") for key in sorted(missing)]
        issues.extend((f"$.{key}", "is not a supported field") for key in sorted(unknown, key=str))
        if issues:
            raise RealityValidationError(issues)
        metadata_data = payload.get("metadata")
        if not isinstance(metadata_data, Mapping):
            raise RealityValidationError([("metadata", "must be an object")])
        for collection_name in ("components", "behaviours", "actions"):
            entries = payload.get(collection_name, ())
            if not isinstance(entries, (list, tuple)):
                raise RealityValidationError([(collection_name, "must be an array")])
        tags = metadata_data.get("tags", ())
        if not isinstance(tags, (list, tuple)):
            raise RealityValidationError([("metadata.tags", "must be an array of strings")])
        metadata_extension_data = metadata_data.get("extensions", {})
        extension_data = payload.get("extensions", {})
        if not isinstance(metadata_extension_data, Mapping):
            raise RealityValidationError([("metadata.extensions", "must be an object")])
        if not isinstance(extension_data, Mapping):
            raise RealityValidationError([("extensions", "must be an object")])
        known_metadata = {"name", "description", "tags", "createdAt", "updatedAt", "extensions"}
        unknown_metadata = set(metadata_data).difference(known_metadata)
        if unknown_metadata:
            raise RealityValidationError([
                (f"metadata.{key}", "is not a supported field; place custom values in metadata.extensions")
                for key in sorted(unknown_metadata, key=str)
            ])
        metadata = RealityMetadata(
            name=metadata_data.get("name"),
            description=metadata_data.get("description"),
            tags=tuple(tags),
            created_at=metadata_data.get("createdAt"),
            updated_at=metadata_data.get("updatedAt"),
            extensions=dict(metadata_extension_data),
        )
        return cls(
            id=payload.get("id"),
            metadata=metadata,
            components=tuple(payload.get("components", ())),
            behaviours=tuple(payload.get("behaviours", ())),
            actions=tuple(payload.get("actions", ())),
            schema_version=payload.get("schemaVersion", REALITY_SCHEMA_VERSION),
            extensions=extension_data,
        )

    @classmethod
    def from_json(cls, serialized: str) -> RealityDocument:
        try:
            payload = json.loads(serialized, parse_constant=_reject_json_constant)
        except (json.JSONDecodeError, TypeError) as error:
            raise ValueError("reality payload is not valid JSON") from error
        return cls.from_dict(payload)


def _validate_refs(path: str, values: tuple[Mapping[str, Any], ...], payload_key: str, issues: list[tuple[str, str]]) -> None:
    seen: set[str] = set()
    for index, entry in enumerate(values):
        entry_path = f"{path}[{index}]"
        if not isinstance(entry, Mapping):
            issues.append((entry_path, "must be an object"))
            continue
        unknown = set(entry).difference({"id", "type", payload_key})
        issues.extend((f"{entry_path}.{key}", "is not a supported field") for key in sorted(unknown, key=str))
        identity = entry.get("id")
        kind = entry.get("type")
        _validate_token(f"{entry_path}.id", identity, issues)
        _validate_token(f"{entry_path}.type", kind, issues)
        if isinstance(identity, str):
            if identity in seen:
                issues.append((f"{entry_path}.id", f"duplicate id {identity!r}"))
            seen.add(identity)
        if payload_key in entry and not _is_json_value(entry[payload_key]):
            issues.append((f"{entry_path}.{payload_key}", "must contain only JSON values"))


def _validate_token(path: str, value: Any, issues: list[tuple[str, str]]) -> None:
    if not isinstance(value, str) or not value.strip() or value.strip() != value:
        issues.append((path, "must be a non-empty trimmed string"))


def _is_timestamp(value: Any) -> bool:
    if not isinstance(value, str) or re.fullmatch(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})", value
    ) is None:
        return False
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        return parsed.tzinfo is not None and parsed.utcoffset() is not None
    except ValueError:
        return False


def _is_json_value(value: Any, ancestors: set[int] | None = None) -> bool:
    if value is None or isinstance(value, (str, bool, int)):
        return True
    if isinstance(value, float):
        return value == value and value not in (float("inf"), float("-inf"))
    if not isinstance(value, (list, tuple, Mapping)):
        return False
    ancestors = set() if ancestors is None else ancestors
    identity = id(value)
    if identity in ancestors:
        return False
    ancestors.add(identity)
    if isinstance(value, Mapping):
        valid = all(isinstance(key, str) and _is_json_value(child, ancestors) for key, child in value.items())
    else:
        valid = all(_is_json_value(child, ancestors) for child in value)
    ancestors.remove(identity)
    return valid


def _freeze_json(value: Any) -> Any:
    if not _is_json_value(value):
        raise TypeError("value must contain only finite JSON data with string object keys")
    if isinstance(value, Mapping):
        return MappingProxyType({str(key): _freeze_json(item) for key, item in value.items()})
    if isinstance(value, (list, tuple)):
        return tuple(_freeze_json(item) for item in value)
    if not _is_json_value(value):
        raise TypeError("value must contain only finite JSON data")
    return value


def _thaw_json(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {key: _thaw_json(item) for key, item in value.items()}
    if isinstance(value, tuple):
        return [_thaw_json(item) for item in value]
    return value


def _reject_json_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant {value!r} is not allowed")
