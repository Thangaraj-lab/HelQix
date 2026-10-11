"""Public Python SDK for HELQIX reality documents and typed references."""

from .action import (
    ActionAccepted,
    ActionFailed,
    ActionFailure,
    ActionInvocation,
    ActionOutcome,
    ActionSucceeded,
    ActionRef,
    action_outcome_from_dict,
    action_outcome_to_dict,
    validate_action_outcome,
)
from .behaviour import BehaviourDefinition, BehaviourRef
from .component import Component, ComponentKey, ComponentRef
from .reality import (
    REALITY_SCHEMA_VERSION,
    JsonValue,
    RealityDocument,
    RealityMetadata,
    RealityValidationError,
)

__version__ = "0.1.0"

__all__ = [
    "REALITY_SCHEMA_VERSION",
    "ActionAccepted",
    "ActionFailed",
    "ActionFailure",
    "ActionInvocation",
    "ActionOutcome",
    "ActionSucceeded",
    "ActionRef",
    "action_outcome_from_dict",
    "action_outcome_to_dict",
    "BehaviourDefinition",
    "BehaviourRef",
    "Component",
    "ComponentKey",
    "ComponentRef",
    "JsonValue",
    "RealityDocument",
    "RealityMetadata",
    "RealityValidationError",
    "validate_action_outcome",
]
