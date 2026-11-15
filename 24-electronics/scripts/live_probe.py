#!/usr/bin/env python
"""Live probe: sensor readings through the serial pair and the message bus.

A seeded sensor trace is sent down both virtual transports into a consumer.
Every scenario injects one transport fault on the receiving side and compares
what the consumer actually read with what the documented fault model predicts
from the same seed; the combined scenario asserts conservation only, because
the five draws interleave. Everything runs in one process: no sockets, no
threads, no network.
"""

from __future__ import annotations

import argparse
import hashlib
import random
import struct
import sys
from dataclasses import asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from helqix_io import Broker, Client, IOTimeoutError, serial_pair  # noqa: E402
from helqix_sensors import load_model, simulate, synthesize_signal  # noqa: E402

MAGIC = 0x4851
RECORD = struct.Struct(">H4sId")
DRAIN_TIMEOUT_S = 0.02
SCENARIOS = ("clean", "drop", "corrupt", "delay", "reorder", "duplicate", "combined")


class ProbeError(RuntimeError):
    """The consumer saw something the fault model says cannot happen."""


def _tag(sensor_id: str) -> bytes:
    try:
        return sensor_id.encode("ascii")[:4].ljust(4, b"\0")
    except UnicodeEncodeError as exc:
        raise ProbeError(f"sensor id {sensor_id!r} must be ASCII for the wire record") from exc


def encode_reading(sensor_id: str, index: int, value: float) -> bytes:
    return RECORD.pack(MAGIC, _tag(sensor_id), index, float(value))


def decode_reading(payload: bytes) -> tuple[str, int, float]:
    if len(payload) != RECORD.size:
        raise ProbeError(f"reading is {len(payload)} bytes, the record is {RECORD.size}")
    magic, tag, index, value = RECORD.unpack(payload)
    if magic != MAGIC:
        raise ProbeError(f"reading starts with 0x{magic:04x}, expected 0x{MAGIC:04x}")
    return tag.rstrip(b"\0").decode("ascii"), index, value


def consume(fetch, expected_values, sensor_id: str, *, timeout_s: float = DRAIN_TIMEOUT_S) -> dict:
    """Read until the transport times out; check every reading against the sensor."""
    _tag(sensor_id)
    expected_tag = sensor_id[:4]
    order: list[int] = []
    seen: set[int] = set()
    duplicates = 0
    out_of_order = 0
    last_first = -1
    timeouts = 0
    while True:
        try:
            payload = fetch(timeout_s)
        except IOTimeoutError:
            timeouts += 1
            break
        tag, index, value = decode_reading(payload)
        if tag != expected_tag:
            raise ProbeError(f"reading {index} is tagged {tag!r}, expected {sensor_id!r}")
        if index < 0 or index >= len(expected_values):
            raise ProbeError(f"reading index {index} is outside 0..{len(expected_values) - 1}")
        if value != expected_values[index]:
            raise ProbeError(
                f"reading {index} arrived as {value!r}, the sensor reported {expected_values[index]!r}"
            )
        if index in seen:
            duplicates += 1
        else:
            if index < last_first:
                out_of_order += 1
            last_first = max(last_first, index)
            seen.add(index)
        order.append(index)
    return {
        "order": order,
        "unique": len(seen),
        "duplicates": duplicates,
        "out_of_order": out_of_order,
        "timeouts": timeouts,
    }


def predict_delivery(count: int, *, seed: int, drop: float = 0.0, corrupt: float = 0.0,
                     duplicate: float = 0.0) -> tuple[list[int], list[int], int, int]:
    """Mirror of the documented per-frame draw order: drop, corrupt, duplicate.

    One draw is taken only for the faults that are configured, matching the
    receiving side. Reorder draws depend on queue state and are not predicted;
    the reorder scenario uses certainties instead.
    """
    rng = random.Random(seed)
    delivered: list[int] = []
    duplicated: list[int] = []
    dropped = corrupted = 0
    for index in range(count):
        if drop and rng.random() < drop:
            dropped += 1
            continue
        if corrupt and rng.random() < corrupt:
            corrupted += 1
            continue
        delivered.append(index)
        if duplicate and rng.random() < duplicate:
            duplicated.append(index)
    return delivered, duplicated, dropped, corrupted


def run_serial(values, sensor_id: str, *, faults: dict, timeout_s: float,
               probe_timeout_s: float | None = None) -> tuple[dict, dict, bool]:
    left, right = serial_pair(capacity_frames=len(values) + 16)
    right.configure_faults(**faults)
    for index, value in enumerate(values):
        left.write(encode_reading(sensor_id, index, value), timeout_s=5.0)
    delay_probe = False
    if probe_timeout_s is not None:
        try:
            right.read(probe_timeout_s)
            raise ProbeError(
                f"a delayed reading arrived before the {probe_timeout_s:g} s probe timeout"
            )
        except IOTimeoutError:
            delay_probe = True
    consumed = consume(right.read, values, sensor_id, timeout_s=timeout_s)
    return consumed, asdict(right.stats), delay_probe


def run_bus(values, sensor_id: str, *, faults: dict, timeout_s: float,
            probe_timeout_s: float | None = None) -> tuple[dict, dict, bool]:
    broker = Broker()
    subscriber = Client("consumer", broker, capacity_messages=len(values) + 16)
    topic = f"plant/{sensor_id}/reading"
    subscriber.subscribe(topic)
    subscriber.configure_faults(**faults)
    publisher = Client("publisher", broker)
    for index, value in enumerate(values):
        publisher.publish(topic, encode_reading(sensor_id, index, value), qos=0, timeout_s=5.0)
    delay_probe = False
    if probe_timeout_s is not None:
        try:
            subscriber.receive(probe_timeout_s)
            raise ProbeError(
                f"a delayed message arrived before the {probe_timeout_s:g} s probe timeout"
            )
        except IOTimeoutError:
            delay_probe = True
    consumed = consume(lambda wait: subscriber.receive(wait).payload, values, sensor_id,
                       timeout_s=timeout_s)
    return consumed, asdict(subscriber.stats), delay_probe


def _check(where: str, observed: dict, expected: dict) -> None:
    for key, value in expected.items():
        if observed[key] != value:
            raise ProbeError(
                f"{where}: {key} is {observed[key]!r}, the documented fault model predicts {value!r}"
            )


def run_scenario(channel, values, sensor_id: str, scenario: str, seed: int) -> dict:
    count = len(values)
    identity = list(range(count))
    zero_stats = {"dropped": 0, "corrupted": 0, "duplicated": 0, "reordered": 0}
    probe_timeout = None
    timeout_s = DRAIN_TIMEOUT_S
    exact = True

    if scenario == "clean":
        faults: dict = {"seed": seed}
        expected_order = identity
        expected = {"unique": count, "duplicates": 0, "out_of_order": 0}
        expected_stats = {"received": count, **zero_stats}
    elif scenario == "drop":
        faults = {"seed": seed, "drop": 0.5}
        expected_order, _, dropped, _ = predict_delivery(count, seed=seed, drop=0.5)
        expected = {"unique": len(expected_order), "duplicates": 0, "out_of_order": 0}
        expected_stats = {"received": len(expected_order), "dropped": dropped,
                          "corrupted": 0, "duplicated": 0, "reordered": 0}
    elif scenario == "corrupt":
        faults = {"seed": seed, "corrupt": 0.5}
        expected_order, _, _, corrupted = predict_delivery(count, seed=seed, corrupt=0.5)
        expected = {"unique": len(expected_order), "duplicates": 0, "out_of_order": 0}
        expected_stats = {"received": len(expected_order), "dropped": 0,
                          "corrupted": corrupted, "duplicated": 0, "reordered": 0}
    elif scenario == "delay":
        faults = {"seed": seed, "delay_s": 0.05}
        probe_timeout = 0.01
        timeout_s = 0.1
        expected_order = identity
        expected = {"unique": count, "duplicates": 0, "out_of_order": 0}
        expected_stats = {"received": count, **zero_stats}
    elif scenario == "reorder":
        faults = {"seed": seed, "reorder": 1.0}
        expected_order = list(range(1, count)) + [0]
        expected = {"unique": count, "duplicates": 0, "out_of_order": 1}
        expected_stats = {"received": count, "dropped": 0, "corrupted": 0,
                          "duplicated": 0, "reordered": count - 1}
    elif scenario == "duplicate":
        faults = {"seed": seed, "duplicate": 1.0}
        delivered, duplicated, _, _ = predict_delivery(count, seed=seed, duplicate=1.0)
        expected_order = [index for index in delivered for _ in range(2 if index in duplicated else 1)]
        expected = {"unique": count, "duplicates": len(duplicated), "out_of_order": 0}
        expected_stats = {"received": len(expected_order), "dropped": 0, "corrupted": 0,
                          "duplicated": len(duplicated), "reordered": 0}
    elif scenario == "combined":
        exact = False
        faults = {"seed": seed, "drop": 0.1, "corrupt": 0.1, "duplicate": 0.1, "reorder": 0.1}
        timeout_s = 0.05
    else:
        raise ProbeError(f"unknown scenario {scenario!r}")

    consumed, stats, delay_probe = channel(
        values, sensor_id, faults=faults, timeout_s=timeout_s, probe_timeout_s=probe_timeout
    )
    where = f"{channel.__name__} {scenario}"
    if exact:
        if consumed["order"] != expected_order:
            raise ProbeError(
                f"{where}: the consumer read {consumed['order'][:8]}..."
                f" but the documented fault model predicts {expected_order[:8]}..."
            )
        _check(where, consumed, expected)
        _check(where, stats, expected_stats)
    else:
        if consumed["unique"] + stats["dropped"] + stats["corrupted"] != count:
            raise ProbeError(
                f"{where}: {consumed['unique']} unique + {stats['dropped']} dropped + "
                f"{stats['corrupted']} corrupted is not the {count} readings sent"
            )
        if stats["received"] != len(consumed["order"]):
            raise ProbeError(f"{where}: the transport counted {stats['received']} deliveries, "
                             f"the consumer read {len(consumed['order'])}")
        if stats["received"] != consumed["unique"] + consumed["duplicates"]:
            raise ProbeError(f"{where}: received does not equal unique plus duplicates")
        if stats["duplicated"] != consumed["duplicates"]:
            raise ProbeError(f"{where}: transport and consumer disagree on duplicates")
    if probe_timeout is not None and not delay_probe:
        raise ProbeError(f"{where}: the delayed readings were not held back")
    return {
        "faults": faults,
        "delivered": len(consumed["order"]),
        "unique": consumed["unique"],
        "duplicates": consumed["duplicates"],
        "out_of_order": consumed["out_of_order"],
        "order": consumed["order"],
        "delay_probe_timed_out": delay_probe,
        "stats": stats,
        "checks": "exact" if exact else "conservation",
    }


def run_probe(model, signal, *, seed: int) -> dict:
    sensor = load_model(model)
    series = synthesize_signal(signal, sensor.sample_rate_hz)
    trace = simulate(sensor, series, seed=seed)
    values = [float(value) for value in trace.reported]
    sensor_id = trace.model.id
    report = {
        "probe": "live/0.1",
        "seed": seed,
        "sensor": {
            "id": sensor_id,
            "kind": trace.model.kind,
            "unit": trace.model.unit,
            "samples": len(values),
            "values_sha": _digest(values),
        },
        "channels": {},
        "labels": {
            "delivered": "MEASURED",
            "dropped": "MEASURED",
            "corrupted": "MEASURED",
            "duplicates": "MEASURED",
            "reordered": "MEASURED",
            "predicted_order": "CALCULATED",
        },
        "outcome": "ok",
    }
    for name, channel in (("serial", run_serial), ("bus", run_bus)):
        report["channels"][name] = {
            scenario: run_scenario(channel, values, sensor_id, scenario, seed)
            for scenario in SCENARIOS
        }
    return report


def _digest(values) -> str:
    blob = b"".join(struct.pack(">d", value) for value in values)
    return hashlib.sha256(blob).hexdigest()[:16]


def _demo_model(samples: int, sample_rate_hz: float) -> tuple[dict, dict]:
    duration = samples / sample_rate_hz
    model = {
        "id": "PRB1",
        "kind": "voltage",
        "unit": "V",
        "range": {"min": -1.0, "max": 1.0},
        "resolution": {"bits": 12},
        "noise_density": 1e-3,
        "bias": 0.0,
        "drift_per_hour": 0.0,
        "bandwidth_hz": sample_rate_hz / 20.0,
        "latency_s": 0.0,
        "sample_rate_hz": sample_rate_hz,
        "saturation": {"mode": "clip"},
        "source": "probe-demo",
    }
    signal = {
        "type": "sine",
        "duration_s": duration,
        "amplitude": 0.5,
        "frequency_hz": 5.0,
        "offset": 0.0,
        "phase_deg": 0.0,
    }
    return model, signal


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, default=11)
    parser.add_argument("--samples", type=int, default=120)
    parser.add_argument("--sample-rate-hz", type=float, default=1000.0)
    args = parser.parse_args(argv)
    if args.samples < 2 or args.sample_rate_hz <= 0:
        print("live probe: FAIL samples must be at least 2 and the rate positive")
        return 1
    model, signal = _demo_model(args.samples, args.sample_rate_hz)
    try:
        report = run_probe(model, signal, seed=args.seed)
    except ProbeError as exc:
        print(f"live probe: FAIL {exc}")
        return 1
    header = (f"{'CHANNEL':<8}{'SCENARIO':<11}{'READ':>7}{'UNIQUE':>8}{'DROPPED':>9}"
              f"{'CORRUPT':>9}{'DUP':>6}{'REORDER':>9}  CHECKS")
    print(header)
    print("-" * len(header))
    for channel, scenarios in report["channels"].items():
        for name, scenario in scenarios.items():
            stats = scenario["stats"]
            print(f"{channel:<8}{name:<11}{scenario['delivered']:>7}{scenario['unique']:>8}"
                  f"{stats['dropped']:>9}{stats['corrupted']:>9}{scenario['duplicates']:>6}"
                  f"{stats['reordered']:>9}  {scenario['checks']}")
    print("-" * len(header))
    print(f"seed {report['seed']}, sensor {report['sensor']['id']} "
          f"({report['sensor']['kind']}, {report['sensor']['samples']} samples); "
          "counts MEASURED, predicted order CALCULATED")
    print("OVERALL PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
