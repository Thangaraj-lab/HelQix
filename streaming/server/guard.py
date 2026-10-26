"""Session guards for a cloud render session (K7): a hard time cap and an idle timeout.

A session ends when either limit is reached; the server then stops the encoder, closes the peer,
closes the browser and exits, so a forgotten session cannot keep a paid GPU running.
"""
from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Callable


@dataclass
class SessionGuard:
    idle_timeout_s: float
    max_session_s: float
    clock: Callable[[], float] = time.monotonic
    started: float = field(init=False)
    last_input: float = field(init=False)

    def __post_init__(self) -> None:
        if self.idle_timeout_s <= 0 or self.max_session_s <= 0:
            raise ValueError("limits must be positive")
        self.started = self.last_input = self.clock()

    def touch(self) -> None:
        """Records user input (keeps the session alive until the hard cap)."""
        self.last_input = self.clock()

    def state(self) -> tuple[str, float]:
        """('ok', seconds until the nearest limit) or ('hard-cap' | 'idle', 0.0). The hard cap wins."""
        now = self.clock()
        to_cap = self.max_session_s - (now - self.started)
        to_idle = self.idle_timeout_s - (now - self.last_input)
        if to_cap <= 0:
            return "hard-cap", 0.0
        if to_idle <= 0:
            return "idle", 0.0
        return "ok", min(to_cap, to_idle)
