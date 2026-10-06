"""Identity of one protected PR image build.

Every construction path, including dataclasses.replace, runs the same
checks, so a BuildIdentity value is always valid."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any

from ci_actions.docker_plan import load_manifest
from ci_actions.github import ActionError, require_env
from ci_actions.validation import MAX_SAFE_INTEGER, REPOSITORY, SHA40

IDENTITY_FIELDS = ("source_repository", "source_sha", "base_sha", "pr_number",
                   "run_id", "run_attempt", "variant", "artifact_repo")
_PATTERNS = {
    "source_repository": REPOSITORY,
    "source_sha": SHA40,
    "base_sha": SHA40,
    "pr_number": re.compile(r"[1-9][0-9]{0,9}"),
    "run_id": re.compile(r"[1-9][0-9]{0,19}"),
    "run_attempt": re.compile(r"[1-9][0-9]{0,5}"),
    "variant": re.compile(r"[a-z][a-z0-9-]*"),
    "artifact_repo": re.compile(r"[a-z0-9-]+-docker\.pkg\.dev/[a-z0-9-]+/[a-z0-9_-]+"),
}


@dataclass(frozen=True)
class BuildIdentity:
    source_repository: str
    source_sha: str
    base_sha: str
    pr_number: str
    run_id: str
    run_attempt: str
    variant: str
    artifact_repo: str
    # Resolved from the Docker capability manifest in __post_init__.
    profile: str = field(init=False, repr=False, compare=False)
    features: str = field(init=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        for key in IDENTITY_FIELDS:
            value = getattr(self, key)
            if not isinstance(value, str) or not _PATTERNS[key].fullmatch(value):
                raise ActionError(f"Invalid protected image {key}")
        if int(self.run_id) > MAX_SAFE_INTEGER:
            raise ActionError("Invalid protected image run_id")
        variants = {variant["id"]: variant for variant in load_manifest().variants}
        variant = variants.get(self.variant)
        if variant is None:
            raise ActionError("Unknown protected image variant")
        object.__setattr__(self, "profile", variant["profile"])
        object.__setattr__(self, "features", variant["features"])

    @classmethod
    def parse(cls, value: Any) -> BuildIdentity:
        if isinstance(value, cls):
            return value
        if not isinstance(value, dict) or set(value) != set(IDENTITY_FIELDS):
            raise ActionError("Invalid protected image identity fields")
        return cls(**{key: value[key] for key in IDENTITY_FIELDS})

    @classmethod
    def from_env(cls) -> BuildIdentity:
        return cls(**{key: require_env("INPUT_" + key.upper()) for key in IDENTITY_FIELDS})

    def as_dict(self) -> dict[str, str]:
        return {key: getattr(self, key) for key in IDENTITY_FIELDS}

    def image_tag(self) -> str:
        prefix = f"pr-{self.pr_number}_"
        if self.profile != "release":
            prefix += self.profile + "_"
        if self.features:
            prefix += re.sub(r"[^a-zA-Z0-9]", "_", self.features) + "_"
        return f"{prefix}r{self.run_id}-a{self.run_attempt}_{self.source_sha}"

    def controller_tag(self) -> str:
        return f"controller-r{self.run_id}-a{self.run_attempt}_{self.base_sha}"

    def check_same_build(self, actual: BuildIdentity, *, mismatch: str, future: str) -> None:
        """`actual` may come from an earlier attempt of this run, never a later one."""
        if any(getattr(actual, key) != getattr(self, key) for key in IDENTITY_FIELDS if key != "run_attempt"):
            raise ActionError(mismatch)
        if int(actual.run_attempt) > int(self.run_attempt):
            raise ActionError(future)
