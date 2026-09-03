"""The provider credential, and the one way this harness keeps it out of artifacts.

The evaluated agent holds its own credential -- the process it runs in needs it
-- so anything the agent influences can carry it: a response the SDK persists,
a comment in a source file that is copied and diffed, a compiler diagnostic
quoting that source back through the controller's stdout. Every artifact writer
that can receive agent-influenced bytes redacts through this module, so the set
of variables and the replacement are stated once.

This is best-effort by construction: it recognises the credential's exact
bytes, and an agent that splits or encodes the value defeats it. `sandbox/
README.md` records that limit and the design that would close it.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any


CREDENTIAL_VARIABLES = ("ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY")
REPLACEMENT = "[REDACTED]"


def configured_credentials() -> list[str]:
    """Return the credential values present in this process's environment."""
    return [value for value in (os.environ.get(name) for name in CREDENTIAL_VARIABLES) if value]


def redact(value: Any) -> Any:
    """Replace the credential wherever it appears in a string, list or mapping."""
    secrets = configured_credentials()
    if not secrets:
        return value

    def walk(item: Any) -> Any:
        if isinstance(item, str):
            for secret in secrets:
                item = item.replace(secret, REPLACEMENT)
            return item
        if isinstance(item, dict):
            return {key: walk(inner) for key, inner in item.items()}
        if isinstance(item, list):
            return [walk(inner) for inner in item]
        return item

    return walk(value)


def redact_tree(root: Path) -> None:
    """Replace the credential wherever it landed in a written artifact tree.

    Swept once, after everything has been written: the writers cannot see a
    credential the agent placed in a file they merely copy or diff.
    """
    secrets = [secret.encode() for secret in configured_credentials()]
    if not secrets:
        return
    replacement = REPLACEMENT.encode()
    for path in root.rglob("*"):
        if path.is_symlink() or not path.is_file():
            continue
        data = path.read_bytes()
        if not any(secret in data for secret in secrets):
            continue
        for secret in secrets:
            data = data.replace(secret, replacement)
        path.write_bytes(data)
