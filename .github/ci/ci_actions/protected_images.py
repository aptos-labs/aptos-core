"""Bounded, data-only handoff from a protected image build to its consumers.

Consumers download by the artifact ID returned by the successful producer,
never by artifact name. Earlier build attempts are valid on a test-only rerun.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import stat
import subprocess
from typing import Callable

from ci_actions.build_identity import IDENTITY_FIELDS, BuildIdentity
from ci_actions.github import ActionError, require_env
from ci_actions.validation import DuplicateKeyError, loads_strict

MAX_BYTES = 32 * 1024
IMAGE_NAMES = ("validator", "tools", "faucet", "forge", "telemetry-service",
               "keyless-pepper-service", "indexer-grpc", "validator-testing", "nft-metadata-crawler")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")
MANIFEST_FIELDS = frozenset(IDENTITY_FIELDS) | {"version", "image_tag", "images", "controller_digest"}


def checked_digest(value: object) -> str:
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        raise ActionError("Invalid protected image digest")
    return value


def registry_digest(reference: str) -> str:
    result = subprocess.run(
        ["docker", "buildx", "imagetools", "inspect", reference, "--format", "{{json .Manifest}}"],
        check=True, capture_output=True, text=True, timeout=120,
    )
    try:
        output = loads_strict(result.stdout)
    except ValueError as error:
        raise ActionError(f"Invalid registry inspect output: {error}") from None
    return checked_digest(output.get("digest"))


def collect(identity: BuildIdentity | dict, resolve: Callable[[str], str] = registry_digest) -> dict:
    identity = BuildIdentity.parse(identity)
    tag = identity.image_tag()
    digests = {name: checked_digest(resolve(f"{identity.artifact_repo}/{name}:{tag}")) for name in IMAGE_NAMES}
    controller = checked_digest(resolve(f"{identity.artifact_repo}/forge:{identity.controller_tag()}"))
    return {"version": 2, **identity.as_dict(), "image_tag": tag, "images": digests, "controller_digest": controller}


def validate(text: str, expected: BuildIdentity | dict) -> dict[str, str]:
    expected = BuildIdentity.parse(expected)
    if len(text.encode()) > MAX_BYTES:
        raise ActionError("Protected image manifest exceeds size limit")
    try:
        manifest = loads_strict(text)
    except DuplicateKeyError:
        raise ActionError("Duplicate protected image manifest key") from None
    except (ValueError, TypeError) as error:
        raise ActionError(f"Invalid protected image manifest: {error}") from None
    if not isinstance(manifest, dict) or set(manifest) != MANIFEST_FIELDS:
        raise ActionError("Invalid protected image manifest fields")
    if type(manifest["version"]) is not int or manifest["version"] != 2:
        raise ActionError("Invalid protected image manifest version")
    actual = BuildIdentity.parse({key: manifest[key] for key in IDENTITY_FIELDS})
    expected.check_same_build(actual, mismatch="Protected image source, variant, registry or run mismatch",
                              future="Protected image build is from a future attempt")
    if manifest["image_tag"] != actual.image_tag():
        raise ActionError("Protected image tag does not match build identity")
    if not isinstance(manifest["images"], dict) or set(manifest["images"]) != set(IMAGE_NAMES):
        raise ActionError("Protected image manifest has missing or unexpected images")
    digests = {f"{expected.artifact_repo}/{name}": checked_digest(digest)
               for name, digest in manifest["images"].items()}
    return {
        "PR_IMAGE_TAG": manifest["image_tag"],
        "PROTECTED_IMAGE_TAG": manifest["image_tag"],
        "PROTECTED_IMAGE_DIGESTS": json.dumps(digests, separators=(",", ":"), sort_keys=True),
        "PROTECTED_TOOLS_IMAGE": f"{expected.artifact_repo}/tools@{manifest['images']['tools']}",
        "PROTECTED_FORGE_IMAGE": f"{expected.artifact_repo}/forge@{checked_digest(manifest['controller_digest'])}",
    }


def main() -> None:
    identity = BuildIdentity.from_env()
    mode = require_env("INPUT_MODE")
    path = Path(require_env("INPUT_MANIFEST_PATH"))
    if mode == "collect":
        manifest = collect(identity)
        # A fresh directory, not a path inside the PR checkout.
        path.parent.mkdir(parents=True, exist_ok=False)
        path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
    elif mode == "verify":
        with path.open("rb") as handle:
            if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
                raise ActionError("Protected image manifest must be a regular file")
            raw = handle.read(MAX_BYTES + 1)
        try:
            env = validate(raw.decode("utf-8"), identity)
        except UnicodeDecodeError:
            raise ActionError("Protected image manifest must be UTF-8") from None
        with open(require_env("GITHUB_ENV"), "a") as handle:
            for key, value in env.items():
                handle.write(f"{key}={value}\n")
    else:
        raise ActionError("Invalid protected image manifest mode")
