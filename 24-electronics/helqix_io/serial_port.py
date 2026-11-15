"""Serial-like virtual port pair.

Two endpoints share an in-process link. ``write`` sends one framed message,
``read`` returns exactly that payload or raises a timeout. Queues are bounded,
so a receiver that never reads eventually applies back-pressure to the writer.
"""

from __future__ import annotations

import time
from collections import deque
from typing import Deque, Tuple

from .errors import BackpressureError, DisconnectedError, IOTimeoutError
from .faults import FaultProfile, PortStats, decode_frame, encode_frame

_POLL_S = 0.001


class SerialPort:
    def __init__(self, name: str, capacity_frames: int = 4096) -> None:
        if capacity_frames < 1:
            raise ValueError("capacity_frames must be at least 1")
        self.name = name
        self.capacity_frames = capacity_frames
        self.faults = FaultProfile()
        self.stats = PortStats()
        self._incoming: Deque[Tuple[bytes, float]] = deque()
        self._duplicates: Deque[bytes] = deque()
        self._peer: SerialPort | None = None
        self._closed = False
        self._rng = self.faults.rng()

    @property
    def closed(self) -> bool:
        return self._closed or (self._peer is not None and self._peer._closed)

    def configure_faults(self, *, seed: int = 0, drop: float = 0.0, corrupt: float = 0.0,
                         delay_s: float = 0.0, duplicate: float = 0.0, reorder: float = 0.0) -> None:
        """Faults on this port affect what this port receives."""
        self.faults = FaultProfile(
            seed=seed, drop=drop, corrupt=corrupt, delay_s=delay_s,
            duplicate=duplicate, reorder=reorder,
        )
        self._rng = self.faults.rng()

    def write(self, payload: bytes, timeout_s: float = 1.0) -> None:
        if self.closed:
            raise DisconnectedError(f"port {self.name!r} is disconnected")
        assert self._peer is not None
        raw = encode_frame(payload)
        deadline = time.monotonic() + timeout_s
        while len(self._peer._incoming) >= self._peer.capacity_frames:
            if self.closed:
                raise DisconnectedError(f"port {self.name!r} disconnected while waiting for space")
            if time.monotonic() >= deadline:
                raise BackpressureError(
                    f"port {self._peer.name!r} queue is full ({self._peer.capacity_frames} frames) after {timeout_s:g} s"
                )
            time.sleep(_POLL_S)
        self._peer._incoming.append((raw, time.monotonic() + self._peer.faults.delay_s))
        self.stats.sent += 1

    def read(self, timeout_s: float = 1.0) -> bytes:
        if self.closed:
            raise DisconnectedError(f"port {self.name!r} is disconnected")
        deadline = time.monotonic() + timeout_s
        while True:
            now = time.monotonic()
            if self._duplicates:
                self.stats.duplicated += 1
                self.stats.received += 1
                return self._duplicates.popleft()
            while self._incoming and self._incoming[0][1] <= now:
                raw, ready_at = self._incoming.popleft()
                if (self.faults.reorder and self._incoming
                        and self._incoming[0][1] <= now
                        and self._rng.random() < self.faults.reorder):
                    other_raw, other_at = self._incoming.popleft()
                    self._incoming.appendleft((raw, ready_at))
                    raw, ready_at = other_raw, other_at
                    self.stats.reordered += 1
                if self.faults.drop and self._rng.random() < self.faults.drop:
                    self.stats.dropped += 1
                    continue
                if self.faults.corrupt and self._rng.random() < self.faults.corrupt:
                    raw = raw[:-1] + bytes([raw[-1] ^ 0xFF])
                    self.stats.corrupted += 1
                    continue
                try:
                    payload = decode_frame(raw)
                except ValueError:
                    self.stats.corrupted += 1
                    continue
                if self.faults.duplicate and self._rng.random() < self.faults.duplicate:
                    self._duplicates.append(payload)
                self.stats.received += 1
                return payload
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                self.stats.timeouts += 1
                raise IOTimeoutError(f"port {self.name!r} received nothing within {timeout_s:g} s")
            time.sleep(min(_POLL_S, remaining))

    def disconnect(self) -> None:
        self._closed = True
        self._incoming.clear()
        self._duplicates.clear()
        if self._peer is not None:
            self._peer._incoming.clear()
            self._peer._duplicates.clear()

    def reconnect(self) -> None:
        self._closed = False
        self._incoming.clear()
        self._duplicates.clear()


def serial_pair(capacity_frames: int = 4096) -> tuple[SerialPort, SerialPort]:
    """Two endpoints wired to each other, as two ends of one cable."""
    left = SerialPort("left", capacity_frames)
    right = SerialPort("right", capacity_frames)
    left._peer = right
    right._peer = left
    return left, right
