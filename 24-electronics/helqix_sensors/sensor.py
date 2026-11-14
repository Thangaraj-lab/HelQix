"""Deterministic model of what a real sensor would report.

Chain, in order: resample the true signal to the sensor clock, add bias and
drift, add white noise with the stated density, band-limit with a first-order
filter whose -3 dB point is exact at the stated bandwidth, apply the stated
latency, quantize, then clip at the range ends and flag every clipped sample.
The same seed always gives byte-identical output.
"""

from __future__ import annotations

import copy
import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.signal import lfilter

from .errors import InputError

KINDS = (
    "temperature",
    "pressure",
    "acceleration",
    "angular_rate",
    "current",
    "voltage",
    "strain",
    "distance",
    "light",
)
SATURATION_MODES = ("clip",)
SIGNAL_TYPES = ("dc", "sine", "step")
MAX_SAMPLES = 10_000_000

LABELS = {
    "quantization_step": "CALCULATED",
    "quantization_rms_theory": "CALCULATED",
    "cutoff_measured_hz": "MEASURED",
    "cutoff_relative_error": "MEASURED",
    "enbw_hz": "CALCULATED",
    "noise_sigma_predicted": "CALCULATED",
    "noise_sigma_measured": "MEASURED",
    "noise_relative_error": "MEASURED",
    "group_delay_s": "CALCULATED",
    "clipped_samples": "MEASURED",
    "reality_gap_rms": "MEASURED",
    "quantization_error_rms": "MEASURED",
    "noise_error_rms": "MEASURED",
    "delay_error_rms": "MEASURED",
}


def _number(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _integer(value, what: str, minimum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise InputError(f"{what} must be an integer")
    if value < minimum:
        raise InputError(f"{what} must be at least {minimum}")
    return value


@dataclass(frozen=True)
class SensorModel:
    id: str
    kind: str
    unit: str
    min_value: float
    max_value: float
    bits: int | None
    step: float
    noise_density: float
    bias: float
    drift_per_hour: float
    bandwidth_hz: float
    latency_s: float
    sample_rate_hz: float
    saturation_mode: str
    source: str
    raw: dict

    @property
    def span(self) -> float:
        return self.max_value - self.min_value


def _read(source) -> dict:
    if isinstance(source, SensorModel):
        return copy.deepcopy(source.raw)
    if isinstance(source, dict):
        return copy.deepcopy(source)
    if isinstance(source, (str, Path)):
        path = Path(source)
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            raise InputError(f"cannot read sensor model {path}: {exc}") from exc
        try:
            body = json.loads(text)
        except json.JSONDecodeError as exc:
            raise InputError(f"sensor model {path} is not valid JSON: {exc}") from exc
        if not isinstance(body, dict):
            raise InputError("sensor model root must be an object")
        return body
    raise InputError(f"sensor model must be a dict, path or SensorModel, got {type(source).__name__}")


def load_model(source) -> SensorModel:
    body = _read(source)
    sid = body.get("id")
    if not isinstance(sid, str) or not sid.strip():
        raise InputError("sensor model needs a non-empty string id")
    kind = body.get("kind")
    if kind not in KINDS:
        raise InputError(
            f"sensor {sid!r} kind must be one of {list(KINDS)}, got {kind!r} "
            "(other kinds are data, not code)"
        )
    unit = body.get("unit")
    if not isinstance(unit, str) or not unit.strip():
        raise InputError(f"sensor {sid!r} needs a non-empty unit string")
    span_body = body.get("range")
    if not isinstance(span_body, dict):
        raise InputError(f"sensor {sid!r} needs range {{'min': number, 'max': number}}")
    low = _number(span_body.get("min"), f"sensor {sid!r} range min")
    high = _number(span_body.get("max"), f"sensor {sid!r} range max")
    if low >= high:
        raise InputError(f"sensor {sid!r} range needs min < max")

    resolution = body.get("resolution")
    if not isinstance(resolution, dict):
        raise InputError(f"sensor {sid!r} resolution must be {{'bits': n}} or {{'step': x}}")
    has_bits = "bits" in resolution
    has_step = "step" in resolution
    if has_bits == has_step:
        raise InputError(f"sensor {sid!r} resolution needs exactly one of bits or step")
    if has_bits:
        bits = _integer(resolution["bits"], f"sensor {sid!r} resolution bits", 1)
        if bits > 32:
            raise InputError(f"sensor {sid!r} resolution bits must be at most 32")
        step = (high - low) / 2**bits
    else:
        bits = None
        step = _number(resolution["step"], f"sensor {sid!r} resolution step")
        if step <= 0:
            raise InputError(f"sensor {sid!r} resolution step must be greater than zero")
        if step > high - low:
            raise InputError(f"sensor {sid!r} resolution step is larger than the range")

    noise = _number(body.get("noise_density", 0.0), f"sensor {sid!r} noise_density")
    if noise < 0:
        raise InputError(f"sensor {sid!r} noise_density must be zero or greater")
    bias = _number(body.get("bias", 0.0), f"sensor {sid!r} bias")
    drift = _number(body.get("drift_per_hour", 0.0), f"sensor {sid!r} drift_per_hour")
    bandwidth = _number(body.get("bandwidth_hz"), f"sensor {sid!r} bandwidth_hz")
    if bandwidth <= 0:
        raise InputError(f"sensor {sid!r} bandwidth_hz must be greater than zero")
    latency = _number(body.get("latency_s", 0.0), f"sensor {sid!r} latency_s")
    if latency < 0:
        raise InputError(f"sensor {sid!r} latency_s must be zero or greater")
    fs = _number(body.get("sample_rate_hz"), f"sensor {sid!r} sample_rate_hz")
    if fs <= 0:
        raise InputError(f"sensor {sid!r} sample_rate_hz must be greater than zero")
    if bandwidth >= fs / 2.0:
        raise InputError(
            f"sensor {sid!r} bandwidth_hz must be below the Nyquist limit {fs / 2.0:g} Hz"
        )

    saturation = body.get("saturation", {"mode": "clip"})
    if not isinstance(saturation, dict):
        raise InputError(f"sensor {sid!r} saturation must be an object")
    mode = saturation.get("mode", "clip")
    if mode not in SATURATION_MODES:
        raise InputError(
            f"sensor {sid!r} saturation mode must be one of {list(SATURATION_MODES)}, got {mode!r}"
        )
    source_name = body.get("source", "generic")
    if not isinstance(source_name, str) or not source_name.strip():
        raise InputError(f"sensor {sid!r} source must be a non-empty string")

    return SensorModel(
        id=sid,
        kind=kind,
        unit=unit,
        min_value=low,
        max_value=high,
        bits=bits,
        step=step,
        noise_density=noise,
        bias=bias,
        drift_per_hour=drift,
        bandwidth_hz=bandwidth,
        latency_s=latency,
        sample_rate_hz=fs,
        saturation_mode=mode,
        source=source_name,
        raw=body,
    )


def synthesize_signal(spec: dict, sample_rate_hz: float) -> dict:
    if not isinstance(spec, dict):
        raise InputError("signal must be an object")
    kind = spec.get("type", "dc")
    if kind not in SIGNAL_TYPES:
        raise InputError(f"signal type must be one of {list(SIGNAL_TYPES)}, got {kind!r}")
    duration = _number(spec.get("duration_s"), "signal duration_s")
    if duration <= 0:
        raise InputError("signal duration_s must be greater than zero")
    count = max(1, int(round(duration * sample_rate_hz)))
    if count > MAX_SAMPLES:
        raise InputError(
            f"signal needs {count} samples at {sample_rate_hz:g} Hz; the limit is {MAX_SAMPLES}"
        )
    time_s = np.arange(count) / sample_rate_hz
    if kind == "dc":
        value = _number(spec.get("value"), "signal value")
        values = np.full(count, value)
    elif kind == "sine":
        amplitude = _number(spec.get("amplitude"), "signal amplitude")
        frequency = _number(spec.get("frequency_hz"), "signal frequency_hz")
        offset = _number(spec.get("offset", 0.0), "signal offset")
        phase = math.radians(_number(spec.get("phase_deg", 0.0), "signal phase_deg"))
        values = offset + amplitude * np.sin(2.0 * math.pi * frequency * time_s + phase)
    else:
        amplitude = _number(spec.get("amplitude"), "signal amplitude")
        delay = _number(spec.get("delay_s", 0.0), "signal delay_s")
        offset = _number(spec.get("offset", 0.0), "signal offset")
        values = np.where(time_s >= delay, offset + amplitude, offset)
    return {"time_s": time_s, "value": values}


def _series(time_s, value, what: str) -> tuple[np.ndarray, np.ndarray]:
    try:
        times = np.asarray(time_s, dtype=float)
        values = np.asarray(value, dtype=float)
    except (TypeError, ValueError) as exc:
        raise InputError(f"{what} time_s and value must be lists of numbers") from exc
    if times.ndim != 1 or values.ndim != 1:
        raise InputError(f"{what} time_s and value must be one-dimensional")
    if times.size != values.size:
        raise InputError(f"{what} time_s and value must have the same length")
    if times.size < 2:
        raise InputError(f"{what} needs at least two samples")
    if not np.all(np.isfinite(times)) or not np.all(np.isfinite(values)):
        raise InputError(f"{what} time_s and value must be finite")
    if times[0] < 0:
        raise InputError(f"{what} time_s must not be negative")
    if np.any(np.diff(times) <= 0):
        raise InputError(f"{what} time_s must be strictly increasing")
    return times, values


def _sensor_clock(end: float, sample_rate_hz: float) -> np.ndarray:
    count = max(1, int(math.floor(end * sample_rate_hz + 1e-9)) + 1)
    if count > MAX_SAMPLES:
        raise InputError(
            f"signal needs {count} samples at {sample_rate_hz:g} Hz; the limit is {MAX_SAMPLES}"
        )
    return np.arange(count) / sample_rate_hz


def _probe_series(document: dict, probe, unit: str) -> tuple[np.ndarray, np.ndarray]:
    probes = document["probes"]
    if not isinstance(probes, list) or not probes:
        raise InputError("results document needs a non-empty probes list")
    if probe is None:
        if len(probes) != 1:
            ids = [entry.get("id") for entry in probes if isinstance(entry, dict)]
            raise InputError(f"results document has several probes; choose one by id or index: {ids}")
        entry = probes[0]
    elif isinstance(probe, bool) or not isinstance(probe, (int, str)):
        raise InputError("probe must be an id string or an index")
    elif isinstance(probe, int):
        if not 0 <= probe < len(probes):
            raise InputError(f"probe index {probe} is outside 0..{len(probes) - 1}")
        entry = probes[probe]
    else:
        matches = [entry for entry in probes if isinstance(entry, dict) and entry.get("id") == probe]
        if not matches:
            ids = [entry.get("id") for entry in probes if isinstance(entry, dict)]
            raise InputError(f"probe {probe!r} is not in the document; ids are {ids}")
        entry = matches[0]
    if not isinstance(entry, dict) or "time_s" not in entry or "value" not in entry:
        raise InputError("each probe needs time_s and value lists")
    probe_unit = entry.get("unit")
    if probe_unit is not None and probe_unit != unit:
        raise InputError(
            f"probe {entry.get('id')!r} is in {probe_unit!r} but the sensor model reports {unit!r}"
        )
    return _series(entry["time_s"], entry["value"], f"probe {entry.get('id')!r}")


def _truth_series(model: SensorModel, signal, probe) -> tuple[np.ndarray, np.ndarray]:
    fs = model.sample_rate_hz
    if isinstance(signal, dict) and "probes" in signal:
        times, values = _probe_series(signal, probe, model.unit)
        clock = _sensor_clock(float(times[-1]), fs)
        return clock, np.interp(clock, times, values)
    if isinstance(signal, dict):
        if probe is not None:
            raise InputError("probe selection only applies to a results document")
        times, values = _series(signal.get("time_s"), signal.get("value"), "signal")
        clock = _sensor_clock(float(times[-1]), fs)
        return clock, np.interp(clock, times, values)
    try:
        values = np.asarray(signal, dtype=float)
    except (TypeError, ValueError) as exc:
        raise InputError("signal must be a series object or an array of numbers") from exc
    if values.ndim != 1 or values.size < 1:
        raise InputError("signal array must be one-dimensional and non-empty")
    if not np.all(np.isfinite(values)):
        raise InputError("signal array must be finite")
    return np.arange(values.size) / fs, values


def _pole(bandwidth_hz: float, sample_rate_hz: float) -> tuple[float, float]:
    # exact -3 dB placement: |H(e^jw)|^2 = 1/2 at w = 2*pi*fc/fs
    c = math.cos(2.0 * math.pi * bandwidth_hz / sample_rate_hz)
    a = (2.0 - c) - math.sqrt((2.0 - c) ** 2 - 1.0)
    return a, 1.0 - a


def _delay(values: np.ndarray, samples: float) -> np.ndarray:
    if samples == 0.0:
        return values.copy()
    index = np.arange(values.size) - samples
    return np.interp(index, np.arange(values.size), values, left=values[0], right=values[-1])


def _cutoff_hz(a: float, b: float, fs: float) -> float:
    target = 1.0 / math.sqrt(2.0)

    def magnitude(frequency: float) -> float:
        w = 2.0 * math.pi * frequency / fs
        return b / abs(1.0 - a * np.exp(-1j * w))

    low = fs * 1e-9
    high = fs * (0.5 - 1e-12)
    for _ in range(200):
        middle = math.sqrt(low * high)
        if magnitude(middle) > target:
            low = middle
        else:
            high = middle
    return math.sqrt(low * high)


@dataclass
class SensorTrace:
    model: SensorModel
    time_s: np.ndarray
    truth: np.ndarray
    analog: np.ndarray
    reported: np.ndarray
    clipped: np.ndarray
    report: dict

    def to_dict(self) -> dict:
        return {
            "status": "ok",
            "analysis": "sensor",
            "model": {
                "id": self.model.id,
                "kind": self.model.kind,
                "unit": self.model.unit,
                "source": self.model.source,
            },
            "series": {
                "time_s": self.time_s.tolist(),
                "truth": self.truth.tolist(),
                "analog": self.analog.tolist(),
                "reported": self.reported.tolist(),
                "clipped": [bool(flag) for flag in self.clipped],
            },
            "report": self.report,
        }


def simulate(model, signal, seed: int = 0, probe=None) -> SensorTrace:
    model = load_model(model)
    seed = _integer(seed, "seed", 0)
    clock, truth = _truth_series(model, signal, probe)
    fs = model.sample_rate_hz
    n = truth.size

    pre = truth + model.bias + model.drift_per_hour * (clock / 3600.0)
    rng = np.random.default_rng(seed)
    white_sigma = model.noise_density * math.sqrt(fs / 2.0)
    noise = rng.normal(0.0, white_sigma, n) if white_sigma > 0 else np.zeros(n)

    a, b = _pole(model.bandwidth_hz, fs)
    x0 = float(pre[0] + noise[0])
    filtered = lfilter([b], [1.0, -a], pre + noise, zi=[a * x0])[0]

    shift = model.latency_s * fs
    analog = _delay(filtered, shift)
    pre_delayed = _delay(pre, shift)

    quantized = model.step * np.rint(analog / model.step)
    reported = np.clip(quantized, model.min_value, model.max_value)
    clipped = (analog < model.min_value) | (analog > model.max_value)

    cutoff = _cutoff_hz(a, b, fs)
    enbw = (fs / 2.0) * (1.0 - a) / (1.0 + a)
    predicted = model.noise_density * math.sqrt(enbw)
    tau_samples = fs / (2.0 * math.pi * model.bandwidth_hz)
    skip = min(max(0, int(round(10.0 * tau_samples))), n)
    residual = analog[skip:] - pre_delayed[skip:]
    measured = float(np.std(residual)) if residual.size else 0.0
    relative = abs(measured - predicted) / predicted if predicted > 0 else 0.0

    gap = float(np.sqrt(np.mean((reported - truth) ** 2)))
    quant_error = float(np.sqrt(np.mean((reported - analog) ** 2)))
    noise_error = float(np.sqrt(np.mean((analog - pre_delayed) ** 2)))
    delay_error = float(np.sqrt(np.mean((pre_delayed - pre) ** 2)))
    parts = {"noise": noise_error, "quantization": quant_error, "delay": delay_error}
    dominant = max(parts, key=parts.get) if max(parts.values()) > 0 else "none"

    report = {
        "sample_count": int(n),
        "sample_rate_hz": fs,
        "duration_s": float(clock[-1]) if n else 0.0,
        "kind": model.kind,
        "unit": model.unit,
        "source": model.source,
        "seed": seed,
        "quantization_step": model.step,
        "quantization_bits": model.bits,
        "quantization_rms_theory": model.step / math.sqrt(12.0),
        "cutoff_requested_hz": model.bandwidth_hz,
        "cutoff_measured_hz": cutoff,
        "cutoff_relative_error": abs(cutoff - model.bandwidth_hz) / model.bandwidth_hz,
        "enbw_hz": enbw,
        "noise_sigma_predicted": predicted,
        "noise_sigma_measured": measured,
        "noise_relative_error": relative,
        "group_delay_s": (a / (1.0 - a)) / fs,
        "clipped_samples": int(np.count_nonzero(clipped)),
        "reality_gap_rms": gap,
        "quantization_error_rms": quant_error,
        "noise_error_rms": noise_error,
        "delay_error_rms": delay_error,
        "dominant_error": dominant,
        "labels": dict(LABELS),
    }
    return SensorTrace(model, clock, truth, analog, reported, clipped, report)
