"""Framing and fault injection shared by both virtual port styles."""

from __future__ import annotations

import random
import struct
import zlib
from dataclasses import dataclass


def encode_frame(payload: bytes) -> bytes:
    """Length-prefixed frame with a CRC32, so a corrupted byte is detected."""
    body = bytes(payload)
    return struct.pack(">I", len(body)) + body + struct.pack(">I", zlib.crc32(body))


def decode_frame(raw: bytes) -> bytes:
    if len(raw) < 8:
        raise ValueError("frame is shorter than its header")
    (length,) = struct.unpack(">I", raw[:4])
    if len(raw) != 8 + length:
        raise ValueError("frame length does not match its header")
    body = raw[4:4 + length]
    (checksum,) = struct.unpack(">I", raw[4 + length:])
    if zlib.crc32(body) != checksum:
        raise ValueError("frame checksum does not match")
    return body


@dataclass
class FaultProfile:
    """Deterministic fault injection applied by the receiving side.

    drop, corrupt, duplicate and reorder are probabilities in [0, 1];
    delay_s is added to the arrival time of every frame. A corrupted frame
    always fails its CRC and is counted, never delivered. A duplicated frame
    waits in a small queue that skips drop and corrupt, so it is always seen.
    A reordered read swaps the two oldest ready frames.
    """

    seed: int = 0
    drop: float = 0.0
    corrupt: float = 0.0
    delay_s: float = 0.0
    duplicate: float = 0.0
    reorder: float = 0.0

    def __post_init__(self) -> None:
        for name in ("drop", "corrupt", "duplicate", "reorder"):
            value = getattr(self, name)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not 0.0 <= value <= 1.0:
                raise ValueError(f"{name} must be a probability between 0 and 1")
        if self.delay_s < 0:
            raise ValueError("delay_s must be zero or greater")

    def rng(self) -> random.Random:
        return random.Random(self.seed)


@dataclass
class PortStats:
    sent: int = 0
    received: int = 0
    dropped: int = 0
    corrupted: int = 0
    timeouts: int = 0
    retries: int = 0
    acks: int = 0
    duplicated: int = 0
    reordered: int = 0
