"""Serial-like port pair: framing, ordering, timeouts, faults, back-pressure."""

import time

import pytest

from helqix_io import (
    BackpressureError,
    DisconnectedError,
    IOTimeoutError,
    decode_frame,
    encode_frame,
    serial_pair,
)


def test_ten_thousand_frames_round_trip_exactly_and_in_order():
    left, right = serial_pair(capacity_frames=10001)
    payloads = [f"frame-{index:05d}".encode() + bytes([index % 251]) for index in range(10000)]
    started = time.perf_counter()
    for payload in payloads:
        left.write(payload, timeout_s=5.0)
    received = [right.read(timeout_s=5.0) for _ in range(10000)]
    elapsed = time.perf_counter() - started
    assert received == payloads
    assert elapsed < 2.0, f"10000 frames took {elapsed:.2f} s"


def test_framing_preserves_empty_and_binary_payloads():
    assert decode_frame(encode_frame(b"")) == b""
    blob = bytes(range(256)) * 4
    assert decode_frame(encode_frame(blob)) == blob
    with pytest.raises(ValueError):
        decode_frame(b"\x00\x00\x00\x04ab")
    bad = bytearray(encode_frame(b"hello"))
    bad[-1] ^= 0xFF
    with pytest.raises(ValueError):
        decode_frame(bytes(bad))


def test_read_timeout_is_bounded():
    _, right = serial_pair()
    started = time.perf_counter()
    with pytest.raises(IOTimeoutError):
        right.read(timeout_s=0.05)
    elapsed = time.perf_counter() - started
    assert 0.03 <= elapsed <= 1.0


def test_back_pressure_raises_after_the_deadline():
    left, right = serial_pair(capacity_frames=2)
    left.write(b"one")
    left.write(b"two")
    started = time.perf_counter()
    with pytest.raises(BackpressureError):
        left.write(b"three", timeout_s=0.05)
    assert time.perf_counter() - started >= 0.03
    assert right.read(timeout_s=0.5) == b"one"
    left.write(b"three", timeout_s=0.5)
    assert right.read(timeout_s=0.5) == b"two"
    assert right.read(timeout_s=0.5) == b"three"


def test_drop_injection_is_seeded_and_counted():
    def run():
        left, right = serial_pair()
        right.configure_faults(seed=7, drop=0.5)
        for index in range(1000):
            left.write(bytes([index % 256]))
        received = 0
        while True:
            try:
                right.read(timeout_s=0.005)
            except IOTimeoutError:
                break
            received += 1
        return received, right.stats.dropped

    first = run()
    second = run()
    assert first == second, "the same seed must drop the same frames"
    received, dropped = first
    assert received + dropped == 1000
    assert 0 < received < 1000


def test_corrupt_injection_never_delivers_bad_bytes():
    left, right = serial_pair()
    right.configure_faults(seed=3, corrupt=0.5)
    payloads = [f"payload-{index:03d}".encode() for index in range(200)]
    for payload in payloads:
        left.write(payload)
    received = []
    while True:
        try:
            received.append(right.read(timeout_s=0.005))
        except IOTimeoutError:
            break
    assert right.stats.corrupted > 0
    assert all(payload in payloads for payload in received)
    assert len(received) + right.stats.corrupted == len(payloads)


def test_delay_holds_the_frame_until_its_time():
    left, right = serial_pair()
    right.configure_faults(delay_s=0.1)
    left.write(b"slow")
    with pytest.raises(IOTimeoutError):
        right.read(timeout_s=0.02)
    assert right.read(timeout_s=0.5) == b"slow"


def test_write_only_reaches_the_peer():
    left, right = serial_pair()
    left.write(b"one-way")
    with pytest.raises(IOTimeoutError):
        left.read(timeout_s=0.02)
    assert right.read(timeout_s=0.5) == b"one-way"


def test_disconnect_refuses_traffic_and_reconnect_clears():
    left, right = serial_pair()
    left.write(b"in flight")
    right.disconnect()
    with pytest.raises(DisconnectedError):
        left.write(b"blocked")
    with pytest.raises(DisconnectedError):
        right.read(timeout_s=0.1)
    right.reconnect()
    with pytest.raises(IOTimeoutError):
        right.read(timeout_s=0.02)
    left.write(b"after")
    assert right.read(timeout_s=0.5) == b"after"
