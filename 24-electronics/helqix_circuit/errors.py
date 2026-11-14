"""Plain-language errors raised at the package boundary."""

from __future__ import annotations


class HelqixError(Exception):
    code = "error"

    def __init__(self, message: str, code: str | None = None):
        super().__init__(message)
        if code is not None:
            self.code = code


class InputError(HelqixError):
    """The case itself is wrong; the platform should exit 2."""

    code = "invalid_input"


class SolverError(HelqixError):
    """The equations could not be solved; the platform should exit 3."""

    code = "solver_failure"
