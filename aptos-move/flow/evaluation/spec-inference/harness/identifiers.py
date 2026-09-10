"""Values that come from a manifest and go on to build a path.

A task id, a round id and their like are read from JSON the harness does not
author -- a corpus manifest, a provenance record, a staged recipe -- and are
then joined into a directory or interpolated into a filename that is created,
written or replaced. That makes every one of them a path-injection sink: an
absolute value discards the directory it was joined to, and a climbing one
escapes it, in both cases before any run manifest has been validated.

These names are always plain names in practice (`MD-median-015`, `round-07`),
so the check is an allowlist rather than a hunt for dangerous characters: a
denylist of `/`, `\\` and NUL admits `..`, and a charset that admits `.` and
`-` admits `..` too unless it is excluded outright.
"""

from __future__ import annotations

from pathlib import Path


ALLOWED_CHARACTERS = frozenset(
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
)


def require_plain_name(value: object, label: str) -> str:
    """Return `value` if it is a plain name, else raise `ValueError`.

    A plain name is a non-empty string of letters, digits, `.`, `_` and `-`
    that is neither `.` nor `..`. It therefore names exactly one entry inside
    the directory it is joined to, whatever that directory turns out to be.
    """
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be a non-empty string, got {value!r}")
    if value in {".", ".."}:
        raise ValueError(f"{label} `{value}` is not a plain name")
    disallowed = sorted(set(value) - ALLOWED_CHARACTERS)
    if disallowed:
        raise ValueError(
            f"{label} `{value}` must contain only letters, digits, '.', '_' or '-'; "
            f"found {', '.join(repr(character) for character in disallowed)}"
        )
    return value


def resolve_within(
    root: Path, relative: object, label: str, *, allow_root: bool = False
) -> Path:
    """Resolve `relative` under `root`, refusing anything that leaves it.

    The companion to `require_plain_name`, for the manifest fields that are
    genuinely relative paths rather than single names -- a sample directory, a
    preparation patch. Resolution follows symlinks, so a link inside `root`
    that points outside it is refused too.

    `root` itself is refused unless `allow_root` is set. Containment alone is
    not the property a caller wants when it is about to write or delete what it
    resolved: `.` is inside `root` and is also `root`, so a manifest naming it
    for a sample directory would have the caller remove the whole corpus. Only
    a caller that genuinely means the root -- a package that sits at the top of
    its own tree -- opts in.
    """
    if not isinstance(relative, str) or not relative:
        raise ValueError(f"{label} must be a non-empty string, got {relative!r}")
    resolved_root = root.resolve()
    resolved = (resolved_root / relative).resolve()
    if resolved == resolved_root:
        if allow_root:
            return resolved
        raise ValueError(
            f"{label} `{relative}` is `{resolved_root}` itself, which this caller "
            "must not write or remove; name an entry inside it"
        )
    if not resolved.is_relative_to(resolved_root):
        raise ValueError(f"{label} `{relative}` escapes `{resolved_root}`")
    return resolved
