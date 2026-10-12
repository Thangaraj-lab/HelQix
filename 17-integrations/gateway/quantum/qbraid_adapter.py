"""HELQIX -> qBraid quantum runtime adapter.

The adapter is optional and provider-agnostic at the HELQIX boundary. It uses
qBraid's native runtime provider for device discovery, OpenQASM 3 validation,
submission, lifecycle control, and result retrieval. No quantum result is
fabricated when the SDK or credentials are absent.
"""
from __future__ import annotations

import os
import time
from dataclasses import dataclass
from typing import Any

try:
    from qbraid import QbraidProvider
    from qbraid.programs.gate_model.qasm3 import OpenQasm3Program
    from qbraid.runtime.native.job import QbraidJob
except ImportError:  # Optional dependency; the gateway remains usable without it.
    QbraidProvider = None  # type: ignore[assignment,misc]
    OpenQasm3Program = None  # type: ignore[assignment,misc]
    QbraidJob = None  # type: ignore[assignment,misc]


class QbraidUnavailable(RuntimeError):
    """Raised when qBraid is not configured in the current runtime."""


class QbraidExecutionError(RuntimeError):
    """Raised when qBraid rejects or fails a quantum execution."""


@dataclass(frozen=True)
class QuantumDevice:
    qrn: str
    status: str
    provider: str | None
    device_type: str | None
    num_qubits: int | None


@dataclass(frozen=True)
class QuantumSubmission:
    job_qrn: str
    device_qrn: str
    submitted_at: float
    metadata: dict[str, Any]


def _require_sdk() -> None:
    if QbraidProvider is None or OpenQasm3Program is None or QbraidJob is None:
        raise QbraidUnavailable("qBraid SDK is not installed. Install the optional quantum requirements.")
    if not os.environ.get("QBRAID_API_KEY") and os.environ.get("HELQIX_ALLOW_QBRAID_CONFIG", "false").lower() != "true":
        raise QbraidUnavailable("QBRAID_API_KEY is not configured.")


def _provider() -> Any:
    _require_sdk()
    try:
        return QbraidProvider()
    except Exception as exc:
        raise QbraidUnavailable(f"qBraid authentication failed: {exc}") from exc


def _job(provider: Any, job_qrn: str) -> Any:
    model = provider.client.get_job(job_qrn)
    device = provider.get_device(str(model.deviceQrn))
    return QbraidJob(job_id=job_qrn, device=device, client=provider.client)


def list_devices() -> list[QuantumDevice]:
    """Return discoverable qBraid devices without submitting work."""
    provider = _provider()
    try:
        devices = provider.get_devices()
    except Exception as exc:
        raise QbraidExecutionError(f"qBraid device discovery failed: {exc}") from exc

    result: list[QuantumDevice] = []
    for device in devices:
        profile = getattr(device, "profile", {}) or {}
        result.append(
            QuantumDevice(
                qrn=str(getattr(device, "id", "")),
                status=str(getattr(device, "status", "UNKNOWN")),
                provider=str(profile.get("provider")) if profile.get("provider") is not None else None,
                device_type=str(profile.get("device_type")) if profile.get("device_type") is not None else None,
                num_qubits=int(profile["num_qubits"]) if profile.get("num_qubits") is not None else None,
            )
        )
    return result


def submit_qasm3(
    qasm3: str,
    device_qrn: str,
    *,
    shots: int = 1024,
    name: str | None = None,
    tags: dict[str, str | int | bool] | None = None,
    runtime_options: dict[str, Any] | None = None,
) -> QuantumSubmission:
    """Validate and submit one OpenQASM 3 program through qBraid."""
    _require_sdk()
    if not qasm3.strip():
        raise ValueError("qasm3 must be non-empty")
    if not device_qrn.strip():
        raise ValueError("device_qrn must be non-empty")
    if not isinstance(shots, int) or shots < 1 or shots > 10_000_000:
        raise ValueError("shots must be an integer in the range 1..10,000,000")

    provider = _provider()
    try:
        device = provider.get_device(device_qrn)
        program = OpenQasm3Program(qasm3)
        program.validate()
        job = device.submit(
            program.serialize(),
            shots=shots,
            name=name,
            tags=tags or {},
            runtime_options=dict(runtime_options or {}),
        )
    except Exception as exc:
        raise QbraidExecutionError(f"qBraid submission failed: {exc}") from exc

    return QuantumSubmission(
        job_qrn=str(job.id),
        device_qrn=device_qrn,
        submitted_at=time.time(),
        metadata={"shots": shots, "name": name, "tags": tags or {}, "provider": "qbraid"},
    )


def get_job(job_qrn: str) -> dict[str, Any]:
    """Return normalized qBraid job lifecycle metadata."""
    provider = _provider()
    if not job_qrn.strip():
        raise ValueError("job_qrn must be non-empty")
    try:
        model = provider.client.get_job(job_qrn)
    except Exception as exc:
        raise QbraidExecutionError(f"qBraid job lookup failed: {exc}") from exc
    return {
        "job_qrn": str(getattr(model, "jobQrn", job_qrn)),
        "device_qrn": str(getattr(model, "deviceQrn", "")),
        "status": str(getattr(model, "status", "UNKNOWN")),
        "status_message": getattr(model, "statusMsg", None),
        "cost": getattr(model, "cost", None),
        "timestamps": getattr(model, "timeStamps", None),
    }


def cancel_job(job_qrn: str) -> dict[str, Any]:
    """Request cancellation and return the resulting lifecycle state."""
    provider = _provider()
    if not job_qrn.strip():
        raise ValueError("job_qrn must be non-empty")
    try:
        job = _job(provider, job_qrn)
        job.cancel()
    except Exception as exc:
        raise QbraidExecutionError(f"qBraid cancellation failed: {exc}") from exc
    return get_job(job_qrn)


def result(job_qrn: str, *, timeout: int | None = None) -> dict[str, Any]:
    """Fetch a terminal qBraid result and normalize its serializable payload."""
    provider = _provider()
    if not job_qrn.strip():
        raise ValueError("job_qrn must be non-empty")
    try:
        job = _job(provider, job_qrn)
        raw = job.result(timeout=timeout)
        model = provider.client.get_job(job_qrn)
    except Exception as exc:
        raise QbraidExecutionError(f"qBraid result retrieval failed: {exc}") from exc

    payload: dict[str, Any] = {
        "job_qrn": job_qrn,
        "device_qrn": str(model.deviceQrn),
        "status": str(getattr(model, "status", "UNKNOWN")),
        "success": bool(getattr(raw, "success", False)),
        "cost": getattr(raw, "cost", None),
        "timestamps": getattr(raw, "time_stamps", None),
    }
    data = getattr(raw, "data", None)
    if data is not None:
        if hasattr(data, "get_counts"):
            payload["counts"] = data.get_counts()
        else:
            payload["data"] = data
    return payload
