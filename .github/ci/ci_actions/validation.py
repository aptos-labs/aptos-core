"""Shared strict JSON parsing and value checks for ci_actions.

This module imports only the standard library, so report_schema and the
producer tests that import it stay independent of the GitHub client."""

from __future__ import annotations

import json
import math
import re
from typing import Any

MAX_SAFE_INTEGER = 2**53 - 1
SHA40 = re.compile(r"[0-9a-f]{40}")
REPOSITORY = re.compile(r"([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)")


class DuplicateKeyError(ValueError):
    """A JSON object repeats a key."""


class NonFiniteNumberError(ValueError):
    """A JSON number is NaN, an infinity, or overflows to an infinity."""


def is_safe_positive_int(value: object) -> bool:
    return type(value) is int and 0 < value <= MAX_SAFE_INTEGER


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> Any:
    raise NonFiniteNumberError(f"JSON number must be finite: {token}")


def _finite_float(token: str) -> float:
    value = float(token)
    if not math.isfinite(value):
        raise NonFiniteNumberError(f"JSON number must be finite: {token}")
    return value


def loads_strict(data: str | bytes | bytearray) -> Any:
    """Parse one JSON document. Bytes must be strict UTF-8; a BOM is rejected.
    For str or bytes input, every failure is a ValueError: JSONDecodeError, UnicodeDecodeError,
    DuplicateKeyError, NonFiniteNumberError, or ValueError for excessive nesting."""
    text = bytes(data).decode("utf-8") if isinstance(data, (bytes, bytearray)) else data
    try:
        return json.loads(text, object_pairs_hook=_unique_object,
                          parse_constant=_reject_constant, parse_float=_finite_float)
    except RecursionError:
        raise ValueError("JSON nesting is too deep") from None
