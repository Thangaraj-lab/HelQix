"""Action declarations, invocation requests, and outcomes for the HELQIX SDK."""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Mapping

from .reality import JsonValue, RealityValidationError, _thaw_json


@dataclass(frozen=True, slots=True)
class ActionRef:
    id: str
    type: str
    input: JsonValue | None = None

    def __post_init__(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if not _is_json_value(self.input):
            raise RealityValidationError([("input", "must contain only finite JSON values")])
        object.__setattr__(self, "input", _freeze_json(self.input))

    def validate(self) -> None:
        _require_token("id", self.id)
        _require_token("type", self.type)
        if not _is_json_value(self.input):
            raise RealityValidationError([("input", "must contain only finite JSON values")])

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"id": self.id, "type": self.type}
        if self.input is not None:
            result["input"] = _thaw_json(self.input)
        return result


@dataclass(frozen=True, slots=True)
class ActionInvocation:
    invocation_id: str
    reality_id: str
    action_id: str
    input: JsonValue
    idempotency_key: str
    requested_at: str | None = None
    trace_id: str | None = None

    def __post_init__(self) -> None:
        if not _is_json_value(self.input):
            raise RealityValidationError([("input", "must contain only finite JSON values")])
        _require_token("invocation_id", self.invocation_id)
        _require_token("reality_id", self.reality_id)
        _require_token("action_id", self.action_id)
        _require_token("idempotency_key", self.idempotency_key)
        if self.trace_id is not None:
            _require_token("trace_id", self.trace_id)
        if self.requested_at is not None and not _is_timestamp(self.requested_at):
            raise RealityValidationError([("requested_at", "must be an ISO-8601 timestamp with timezone")])
        object.__setattr__(self, "input", _freeze_json(self.input))

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "invocationId": self.invocation_id,
            "realityId": self.reality_id,
            "actionId": self.action_id,
            "input": _thaw_json(self.input),
            "idempotencyKey": self.idempotency_key,
        }
        if self.requested_at is not None:
            result["requestedAt"] = self.requested_at
        if self.trace_id is not None:
            result["traceId"] = self.trace_id
        return result

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> ActionInvocation:
        _require_mapping_keys(payload, {"invocationId", "realityId", "actionId", "input", "idempotencyKey"},
                              {"requestedAt", "traceId"}, "action invocation")
        return cls(
            invocation_id=payload["invocationId"],
            reality_id=payload["realityId"],
            action_id=payload["actionId"],
            input=payload["input"],
            idempotency_key=payload["idempotencyKey"],
            requested_at=payload.get("requestedAt"),
            trace_id=payload.get("traceId"),
        )


@dataclass(frozen=True, slots=True)
class ActionFailure:
    code: str
    message: str
    retryable: bool
    details: JsonValue | None = None

    def __post_init__(self) -> None:
        self.validate()
        object.__setattr__(self, "details", _freeze_json(self.details))

    def validate(self) -> None:
        issues: list[tuple[str, str]] = []
        if not _is_token(self.code):
            issues.append(("error.code", "must be a non-empty trimmed string"))
        if not isinstance(self.message, str) or not self.message.strip():
            issues.append(("error.message", "must not be empty"))
        if not isinstance(self.retryable, bool):
            issues.append(("error.retryable", "must be a boolean"))
        if not _is_json_value(self.details):
            issues.append(("error.details", "must contain only finite JSON values"))
        if issues:
            raise RealityValidationError(issues)

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"code": self.code, "message": self.message, "retryable": self.retryable}
        if self.details is not None:
            result["details"] = _thaw_json(self.details)
        return result


@dataclass(frozen=True, slots=True)
class ActionSucceeded:
    output: JsonValue
    completed_at: str

    def __post_init__(self) -> None:
        self.validate()
        object.__setattr__(self, "output", _freeze_json(self.output))

    def validate(self) -> None:
        _require_timestamp("completed_at", self.completed_at)
        if not _is_json_value(self.output):
            raise RealityValidationError([("output", "must contain only finite JSON values")])


@dataclass(frozen=True, slots=True)
class ActionAccepted:
    operation_id: str
    accepted_at: str

    def __post_init__(self) -> None:
        self.validate()

    def validate(self) -> None:
        _require_token("operation_id", self.operation_id)
        _require_timestamp("accepted_at", self.accepted_at)


@dataclass(frozen=True, slots=True)
class ActionFailed:
    error: ActionFailure
    completed_at: str

    def __post_init__(self) -> None:
        self.validate()

    def validate(self) -> None:
        self.error.validate()
        _require_timestamp("completed_at", self.completed_at)


ActionOutcome = ActionSucceeded | ActionAccepted | ActionFailed


def validate_action_outcome(value: Any) -> bool:
    """Return whether an outcome has a valid shape and transport-safe values."""
    if not isinstance(value, (ActionSucceeded, ActionAccepted, ActionFailed)):
        return False
    try:
        value.validate()
    except (RealityValidationError, AttributeError, TypeError):
        return False
    return True


def action_outcome_to_dict(value: ActionOutcome) -> dict[str, Any]:
    if not validate_action_outcome(value):
        raise RealityValidationError([("outcome", "is not a valid action outcome")])
    if isinstance(value, ActionSucceeded):
        return {"status": "succeeded", "output": _thaw_json(value.output), "completedAt": value.completed_at}
    if isinstance(value, ActionAccepted):
        return {"status": "accepted", "operationId": value.operation_id, "acceptedAt": value.accepted_at}
    return {"status": "failed", "error": value.error.to_dict(), "completedAt": value.completed_at}


def action_outcome_from_dict(payload: Mapping[str, Any]) -> ActionOutcome:
    if not isinstance(payload, Mapping) or not isinstance(payload.get("status"), str):
        raise RealityValidationError([("outcome", "must be an object with a status")])
    status = payload["status"]
    if status == "succeeded":
        _require_mapping_keys(payload, {"status", "output", "completedAt"}, set(), "succeeded outcome")
        return ActionSucceeded(payload["output"], payload["completedAt"])
    if status == "accepted":
        _require_mapping_keys(payload, {"status", "operationId", "acceptedAt"}, set(), "accepted outcome")
        return ActionAccepted(payload["operationId"], payload["acceptedAt"])
    if status == "failed":
        _require_mapping_keys(payload, {"status", "error", "completedAt"}, set(), "failed outcome")
        error = payload["error"]
        if not isinstance(error, Mapping):
            raise RealityValidationError([("error", "must be an object")])
        _require_mapping_keys(error, {"code", "message", "retryable"}, {"details"}, "action error")
        return ActionFailed(
            ActionFailure(error["code"], error["message"], error["retryable"], error.get("details")),
            payload["completedAt"],
        )
    raise RealityValidationError([("status", "must be succeeded, accepted, or failed")])


def _require_token(name: str, value: Any) -> None:
    if not _is_token(value):
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


def _is_token(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value.strip() == value


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


def _require_timestamp(name: str, value: str) -> None:
    if not _is_timestamp(value):
        raise RealityValidationError([(name, "must be an ISO-8601 timestamp with timezone")])


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
        raise TypeError("value must contain only finite JSON data")
    if isinstance(value, Mapping):
        from types import MappingProxyType
        return MappingProxyType({key: _freeze_json(item) for key, item in value.items()})
    if isinstance(value, (list, tuple)):
        return tuple(_freeze_json(item) for item in value)
    return value


def _thaw_json(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {key: _thaw_json(item) for key, item in value.items()}
    if isinstance(value, tuple):
        return [_thaw_json(item) for item in value]
    return value
