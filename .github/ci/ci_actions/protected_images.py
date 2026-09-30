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

from ci_actions.docker_plan import load_manifest
from ci_actions.github import ActionError, require_env

MAX_BYTES = 32 * 1024
IMAGE_NAMES = ("validator", "tools", "faucet", "forge", "telemetry-service",
               "keyless-pepper-service", "indexer-grpc", "validator-testing", "nft-metadata-crawler")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")
IDENTITY_KEYS = {"source_repository", "source_sha", "pr_number", "run_id", "run_attempt", "variant", "artifact_repo"}


def identity_checked(identity: dict) -> dict:
    patterns = {
        "source_repository": r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+",
        "source_sha": r"[0-9a-f]{40}",
        "pr_number": r"[1-9][0-9]{0,9}",
        "run_id": r"[1-9][0-9]{0,19}",
        "run_attempt": r"[1-9][0-9]{0,5}",
        "variant": r"[a-z][a-z0-9-]*",
        "artifact_repo": r"[a-z0-9-]+-docker\.pkg\.dev/[a-z0-9-]+/[a-z0-9_-]+",
    }
    if not isinstance(identity, dict) or set(identity) != IDENTITY_KEYS:
        raise ActionError("Invalid protected image identity fields")
    for key, pattern in patterns.items():
        if not isinstance(identity[key], str) or not re.fullmatch(pattern, identity[key]):
            raise ActionError(f"Invalid protected image {key}")
    variants = {variant["id"]: variant for variant in load_manifest().variants}
    if identity["variant"] not in variants:
        raise ActionError("Unknown protected image variant")
    return variants[identity["variant"]]


def image_tag(identity: dict) -> str:
    variant = identity_checked(identity)
    prefix = f"pr-{identity['pr_number']}_"
    if variant["profile"] != "release":
        prefix += variant["profile"] + "_"
    if variant["features"]:
        prefix += re.sub(r"[^a-zA-Z0-9]", "_", variant["features"]) + "_"
    return f"{prefix}r{identity['run_id']}-a{identity['run_attempt']}_{identity['source_sha']}"


def checked_digest(value: object) -> str:
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        raise ActionError("Invalid protected image digest")
    return value


def registry_digest(reference: str) -> str:
    result = subprocess.run(
        ["docker", "buildx", "imagetools", "inspect", reference, "--format", "{{json .Manifest}}"],
        check=True, capture_output=True, text=True, timeout=120,
    )
    return checked_digest(json.loads(result.stdout).get("digest"))


def collect(identity: dict, resolve: Callable[[str], str] = registry_digest) -> dict:
    tag = image_tag(identity)
    digests = {name: checked_digest(resolve(f"{identity['artifact_repo']}/{name}:{tag}")) for name in IMAGE_NAMES}
    return {"version": 1, **identity, "image_tag": tag, "images": digests}


def unique_object(pairs: list) -> dict:
    value = dict(pairs)
    if len(value) != len(pairs):
        raise ActionError("Duplicate protected image manifest key")
    return value


def validate(text: str, expected: dict) -> dict[str, str]:
    identity_checked(expected)
    if len(text.encode()) > MAX_BYTES:
        raise ActionError("Protected image manifest exceeds size limit")
    try:
        manifest = json.loads(text, object_pairs_hook=unique_object)
    except (ValueError, TypeError) as error:
        raise ActionError(f"Invalid protected image manifest: {error}") from None
    if not isinstance(manifest, dict) or set(manifest) != IDENTITY_KEYS | {"version", "image_tag", "images"}:
        raise ActionError("Invalid protected image manifest fields")
    if type(manifest["version"]) is not int or manifest["version"] != 1:
        raise ActionError("Invalid protected image manifest version")
    actual = {key: manifest[key] for key in IDENTITY_KEYS}
    identity_checked(actual)
    if any(actual[key] != expected[key] for key in IDENTITY_KEYS - {"run_attempt"}):
        raise ActionError("Protected image source, variant, registry or run mismatch")
    if int(actual["run_attempt"]) > int(expected["run_attempt"]):
        raise ActionError("Protected image build is from a future attempt")
    if manifest["image_tag"] != image_tag(actual):
        raise ActionError("Protected image tag does not match build identity")
    if not isinstance(manifest["images"], dict) or set(manifest["images"]) != set(IMAGE_NAMES):
        raise ActionError("Protected image manifest has missing or unexpected images")
    digests = {f"{expected['artifact_repo']}/{name}": checked_digest(digest)
               for name, digest in manifest["images"].items()}
    return {
        "PR_IMAGE_TAG": manifest["image_tag"],
        "PROTECTED_IMAGE_TAG": manifest["image_tag"],
        "PROTECTED_IMAGE_DIGESTS": json.dumps(digests, separators=(",", ":"), sort_keys=True),
        "PROTECTED_TOOLS_IMAGE": f"{expected['artifact_repo']}/tools@{manifest['images']['tools']}",
    }


def main() -> None:
    identity = {key: require_env("INPUT_" + key.upper()) for key in IDENTITY_KEYS}
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
