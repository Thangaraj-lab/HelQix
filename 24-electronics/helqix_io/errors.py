"""Errors raised by the virtual ports."""

from __future__ import annotations


class IOTimeoutError(TimeoutError):
    """No message arrived before the deadline."""


class BackpressureError(TimeoutError):
    """The receiver queue is full and stayed full until the deadline."""


class DisconnectedError(ConnectionError):
    """The port or its peer is closed."""
