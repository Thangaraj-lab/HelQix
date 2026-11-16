"""MQTT-style in-process message bus.

A ``Broker`` routes by topic filter. Clients publish and subscribe; delivery is
FIFO per client with a bounded queue for back-pressure. Faults are injected on
the receiving client. QoS 1 keeps a message pending until the subscriber acks
it, so a message published while the subscriber is disconnected is resent by
``retry_unacked`` after reconnecting.
"""

from __future__ import annotations

import random
import time
import zlib
from collections import deque
from dataclasses import dataclass
from typing import Deque

from .errors import BackpressureError, DisconnectedError, IOTimeoutError
from .faults import FaultProfile, PortStats

_POLL_S = 0.001


def topic_matches(subscription: str, topic: str) -> bool:
    """MQTT wildcards: ``+`` is one level, ``#`` is the rest of the tree."""
    subscription_levels = subscription.split("/")
    topic_levels = topic.split("/")
    for index, level in enumerate(subscription_levels):
        if level == "#":
            return index == len(subscription_levels) - 1
        if index >= len(topic_levels):
            return False
        if level != "+" and level != topic_levels[index]:
            return False
    return len(subscription_levels) == len(topic_levels)


def _validate_topic(topic: str, *, allow_wildcards: bool) -> None:
    if not isinstance(topic, str) or not topic:
        raise ValueError("topic must be a non-empty string")
    levels = topic.split("/")
    if any(level == "" for level in levels):
        raise ValueError(f"topic {topic!r} has an empty level")
    if allow_wildcards:
        for index, level in enumerate(levels):
            if level == "#" and index != len(levels) - 1:
                raise ValueError("# must be the last level of a subscription")
            if level not in ("+", "#") and any(char in level for char in "+#"):
                raise ValueError(f"wildcard characters must occupy a whole level in {topic!r}")
    elif "+" in topic or "#" in topic:
        raise ValueError(f"publish topic {topic!r} must not contain wildcards")


@dataclass(frozen=True)
class Message:
    message_id: int
    topic: str
    payload: bytes
    checksum: int
    available_at: float

    def payload_is_valid(self) -> bool:
        return zlib.crc32(self.payload) == self.checksum


@dataclass
class _Pending:
    topic: str
    payload: bytes


class Broker:
    def __init__(self) -> None:
        self._clients: list[Client] = []
        self._next_id = 1
        self._acked: set[int] = set()

    def register(self, client: "Client") -> None:
        if client not in self._clients:
            self._clients.append(client)

    def topic_clients(self, topic: str) -> list["Client"]:
        return [
            client for client in self._clients
            if client.connected and any(topic_matches(sub, topic) for sub in client.subscriptions)
        ]

    def publish(self, topic: str, payload: bytes, sender: "Client | None", qos: int) -> int:
        targets = self.topic_clients(topic)
        full = [client for client in targets if len(client._incoming) >= client.capacity_messages]
        if full:
            raise BackpressureError(
                f"full queue for {full[0].client_id!r} ({full[0].capacity_messages} messages); "
                f"nothing was published"
            )
        message_id = self._next_id
        self._next_id += 1
        data = bytes(payload)
        checksum = zlib.crc32(data)
        for client in targets:
            client._incoming.append(
                Message(message_id, topic, data, checksum, time.monotonic() + client.faults.delay_s)
            )
            client.stats.sent += 1
        return message_id

    def record_ack(self, message_id: int) -> None:
        self._acked.add(message_id)

    def is_acked(self, message_id: int) -> bool:
        return message_id in self._acked


class Client:
    def __init__(self, client_id: str, broker: Broker, capacity_messages: int = 4096) -> None:
        if capacity_messages < 1:
            raise ValueError("capacity_messages must be at least 1")
        self.client_id = client_id
        self.broker = broker
        self.capacity_messages = capacity_messages
        self.subscriptions: list[str] = []
        self.faults = FaultProfile()
        self.stats = PortStats()
        self._incoming: Deque[Message] = deque()
        self._duplicates: Deque[Message] = deque()
        self._pending: dict[int, _Pending] = {}
        self._closed = False
        self._rng: random.Random = self.faults.rng()
        broker.register(self)

    @property
    def connected(self) -> bool:
        return not self._closed

    def configure_faults(self, *, seed: int = 0, drop: float = 0.0, corrupt: float = 0.0,
                         delay_s: float = 0.0, duplicate: float = 0.0, reorder: float = 0.0) -> None:
        self.faults = FaultProfile(
            seed=seed, drop=drop, corrupt=corrupt, delay_s=delay_s,
            duplicate=duplicate, reorder=reorder,
        )
        self._rng = self.faults.rng()

    def subscribe(self, subscription: str) -> None:
        _validate_topic(subscription, allow_wildcards=True)
        if subscription not in self.subscriptions:
            self.subscriptions.append(subscription)

    def publish(self, topic: str, payload: bytes, qos: int = 0, timeout_s: float = 1.0) -> int:
        if self._closed:
            raise DisconnectedError(f"client {self.client_id!r} is disconnected")
        if qos not in (0, 1):
            raise ValueError("qos must be 0 or 1")
        _validate_topic(topic, allow_wildcards=False)
        deadline = time.monotonic() + timeout_s
        while True:
            try:
                message_id = self.broker.publish(topic, payload, self, qos)
                break
            except BackpressureError:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(_POLL_S)
        if qos == 1:
            self._pending[message_id] = _Pending(topic, bytes(payload))
        return message_id

    def receive(self, timeout_s: float = 1.0) -> Message:
        if self._closed:
            raise DisconnectedError(f"client {self.client_id!r} is disconnected")
        deadline = time.monotonic() + timeout_s
        while True:
            now = time.monotonic()
            if self._duplicates:
                self.stats.duplicated += 1
                self.stats.received += 1
                return self._duplicates.popleft()
            while self._incoming and self._incoming[0].available_at <= now:
                message = self._incoming.popleft()
                if (self.faults.reorder and self._incoming
                        and self._incoming[0].available_at <= now
                        and self._rng.random() < self.faults.reorder):
                    other = self._incoming.popleft()
                    self._incoming.appendleft(message)
                    message = other
                    self.stats.reordered += 1
                if self.faults.drop and self._rng.random() < self.faults.drop:
                    self.stats.dropped += 1
                    continue
                if not message.payload_is_valid():
                    self.stats.corrupted += 1
                    continue
                if self.faults.corrupt and self._rng.random() < self.faults.corrupt:
                    self.stats.corrupted += 1
                    continue
                if self.faults.duplicate and self._rng.random() < self.faults.duplicate:
                    self._duplicates.append(message)
                self.stats.received += 1
                return message
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                self.stats.timeouts += 1
                raise IOTimeoutError(f"client {self.client_id!r} received nothing within {timeout_s:g} s")
            time.sleep(min(_POLL_S, remaining))

    def ack(self, message_id: int) -> None:
        if self._closed:
            raise DisconnectedError(f"client {self.client_id!r} is disconnected")
        self.broker.record_ack(message_id)
        self.stats.acks += 1

    def retry_unacked(self) -> int:
        """Resend every QoS 1 message that has not been acked yet."""
        if self._closed:
            raise DisconnectedError(f"client {self.client_id!r} is disconnected")
        resent = 0
        for message_id in list(self._pending):
            if self.broker.is_acked(message_id):
                del self._pending[message_id]
                continue
            pending = self._pending[message_id]
            new_id = self.broker.publish(pending.topic, pending.payload, self, 1)
            del self._pending[message_id]
            self._pending[new_id] = pending
            resent += 1
            self.stats.retries += 1
        return resent

    def disconnect(self) -> None:
        self._closed = True
        self._incoming.clear()
        self._duplicates.clear()

    def reconnect(self) -> None:
        self._closed = False
        self._incoming.clear()
        self._duplicates.clear()
