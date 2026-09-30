"""Transfer Docker images as checked data between isolated protected jobs.

The build artifact is untrusted, even though its producer runs on protected
runners. The publisher never loads it into Docker or runs its contents.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tarfile

from ci_actions.github import ActionError, require_env
from ci_actions.protected_images import IMAGE_NAMES, identity_checked, image_tag, unique_object

IDENTITY_KEYS = frozenset({"source_repository", "source_sha", "base_sha", "pr_number",
                           "run_id", "run_attempt", "variant", "artifact_repo"})
METADATA_NAME = "image-archives.json"
MAX_METADATA_BYTES = 32 * 1024
MAX_DOCKER_MANIFEST_BYTES = 256 * 1024
MAX_ARCHIVE_BYTES = 50 * 1024 * 1024 * 1024
MAX_TAR_MEMBERS = 10_000
SHA256 = re.compile(r"[0-9a-f]{64}")
SAFE_MEMBER = re.compile(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*")
E2E_ALIASES = ("pr", "devnet", "testnet", "mainnet")


def archive_names(mode: str) -> dict[str, str]:
    if mode == "build":
        return {role: f"{role}.tar" for role in IMAGE_NAMES}
    if mode == "e2e":
        return {alias: f"tools-{alias}.tar" for alias in E2E_ALIASES}
    raise ActionError("Invalid image archive type")


def source_tag(identity: dict, name: str, mode: str) -> str:
    if mode == "build":
        return f"aptos-core/{name}:{identity['source_sha']}-from-local"
    if mode == "e2e":
        return f"aptos-ci/tools:{name}"
    raise ActionError("Invalid image archive type")


def regular_file(path: Path) -> None:
    try:
        info = path.lstat()
    except OSError as error:
        raise ActionError(f"Missing image archive file: {path.name}") from error
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise ActionError(f"Image archive must be a regular unlinked file: {path.name}")


def digest_file(path: Path) -> str:
    regular_file(path)
    if path.stat().st_size > MAX_ARCHIVE_BYTES:
        raise ActionError("Image archive exceeds size limit")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
            raise ActionError("Image archive is not a regular file")
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def safe_tar_path(path: str) -> bool:
    return bool(SAFE_MEMBER.fullmatch(path.rstrip("/"))) and all(
        part not in (".", "..") for part in path.rstrip("/").split("/"))


def validate_docker_archive(path: Path, expected_tag: str) -> None:
    """Check docker-save structure before skopeo reads an untrusted archive."""
    try:
        with tarfile.open(path, "r:") as archive:
            members: dict[str, tarfile.TarInfo] = {}
            for count, member in enumerate(archive, start=1):
                if count > MAX_TAR_MEMBERS:
                    raise ActionError("Docker archive has too many members")
                if not safe_tar_path(member.name) or member.name.rstrip("/") in members:
                    raise ActionError("Unsafe or duplicate Docker archive path")
                if not (member.isfile() or member.isdir()):
                    raise ActionError("Docker archive has a link or special member")
                members[member.name.rstrip("/")] = member
            manifest_info = members.get("manifest.json")
            if manifest_info is None or not manifest_info.isfile() or manifest_info.size > MAX_DOCKER_MANIFEST_BYTES:
                raise ActionError("Missing or oversized Docker archive manifest")
            manifest_file = archive.extractfile(manifest_info)
            if manifest_file is None:
                raise ActionError("Missing Docker archive manifest")
            manifest = json.load(manifest_file, object_pairs_hook=unique_object)
            if not isinstance(manifest, list) or len(manifest) != 1 or not isinstance(manifest[0], dict):
                raise ActionError("Docker archive must contain exactly one image")
            image = manifest[0]
            if set(image) != {"Config", "RepoTags", "Layers"} or image["RepoTags"] != [expected_tag]:
                raise ActionError("Docker archive source tag does not match expected image")
            if not isinstance(image["Config"], str) or not isinstance(image["Layers"], list) or not image["Layers"]:
                raise ActionError("Invalid Docker archive manifest fields")
            for name in [image["Config"], *image["Layers"]]:
                if not isinstance(name, str) or not safe_tar_path(name):
                    raise ActionError("Unsafe Docker archive manifest path")
                part = members.get(name)
                if part is None or not part.isfile():
                    raise ActionError("Docker archive manifest references a missing file")
    except (tarfile.TarError, OSError, ValueError, UnicodeError, TypeError) as error:
        raise ActionError(f"Invalid Docker archive: {error}") from None


def export_archives(directory: Path, identity: dict, mode: str) -> None:
    identity_checked(identity)
    directory.mkdir(parents=True, exist_ok=False)
    files = archive_names(mode)
    if mode == "e2e":
        protected_tools = require_env("PROTECTED_TOOLS_IMAGE")
        prefix = identity["artifact_repo"] + "/tools@sha256:"
        if not protected_tools.startswith(prefix) or not SHA256.fullmatch(protected_tools[len(prefix):]):
            raise ActionError("Invalid protected tools digest reference")
        references = {"pr": protected_tools}
        references.update({name: f"{identity['artifact_repo']}/tools:{name}" for name in E2E_ALIASES if name != "pr"})
    hashes = {}
    for name, filename in files.items():
        local = source_tag(identity, name, mode)
        if mode == "e2e":
            subprocess.run(["docker", "pull", references[name]], check=True, timeout=1800)
            subprocess.run(["docker", "tag", references[name], local], check=True, timeout=120)
        path = directory / filename
        subprocess.run(["docker", "save", "--output", str(path), local], check=True, timeout=1800)
        hashes[name] = digest_file(path)
    metadata = {"version": 1, "type": mode, "identity": identity, "archives": hashes}
    (directory / METADATA_NAME).write_text(json.dumps(metadata, sort_keys=True) + "\n")


def validate_archives(directory: Path, expected: dict, mode: str) -> dict[str, Path]:
    identity_checked(expected)
    if not directory.is_dir() or directory.is_symlink():
        raise ActionError("Image archive directory must be a real directory")
    expected_files = archive_names(mode)
    if {entry.name for entry in directory.iterdir()} != {METADATA_NAME, *expected_files.values()}:
        raise ActionError("Image archive has missing or unexpected files")
    metadata_path = directory / METADATA_NAME
    regular_file(metadata_path)
    with metadata_path.open("rb") as handle:
        raw = handle.read(MAX_METADATA_BYTES + 1)
    if len(raw) > MAX_METADATA_BYTES:
        raise ActionError("Image archive metadata exceeds size limit")
    try:
        metadata = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    except (UnicodeError, ValueError, TypeError) as error:
        raise ActionError(f"Invalid image archive metadata: {error}") from None
    if not isinstance(metadata, dict) or set(metadata) != {"version", "type", "identity", "archives"}:
        raise ActionError("Invalid image archive metadata fields")
    if type(metadata["version"]) is not int or metadata["version"] != 1 or metadata["type"] != mode:
        raise ActionError("Invalid image archive metadata version or type")
    actual = metadata["identity"]
    if not isinstance(actual, dict):
        raise ActionError("Invalid image archive identity")
    identity_checked(actual)
    if any(actual[key] != expected[key] for key in IDENTITY_KEYS - {"run_attempt"}):
        raise ActionError("Image archive identity does not match approved build")
    if int(actual["run_attempt"]) > int(expected["run_attempt"]):
        raise ActionError("Image archive is from a future attempt")
    if not isinstance(metadata["archives"], dict) or set(metadata["archives"]) != set(expected_files):
        raise ActionError("Image archive has missing or unexpected digests")
    paths = {}
    for name, filename in expected_files.items():
        digest = metadata["archives"][name]
        if not isinstance(digest, str) or not SHA256.fullmatch(digest):
            raise ActionError("Invalid image archive digest")
        path = directory / filename
        if digest_file(path) != digest:
            raise ActionError("Image archive digest mismatch")
        validate_docker_archive(path, source_tag(expected, name, mode))
        paths[name] = path
    return paths


def publish_archives(directory: Path, identity: dict) -> None:
    paths = validate_archives(directory, identity, "build")
    tag = image_tag(identity)
    for role, path in paths.items():
        source = source_tag(identity, role, "build")
        destination = f"{identity['artifact_repo']}/{role}:{tag}"
        subprocess.run(["skopeo", "copy", f"docker-archive:{path}:{source}", f"docker://{destination}"],
                       check=True, timeout=1800)


def load_e2e(directory: Path, identity: dict) -> None:
    paths = validate_archives(directory, identity, "e2e")
    for alias in E2E_ALIASES:
        subprocess.run(["docker", "load", "--input", str(paths[alias])], check=True, timeout=1800)
    with open(require_env("GITHUB_ENV"), "a", encoding="utf-8") as handle:
        handle.write("PR_IMAGE_TAG=pr\nGCP_DOCKER_ARTIFACT_REPO=aptos-ci\nAPTOS_E2E_OFFLINE_IMAGES=true\n")


def main() -> None:
    mode = require_env("INPUT_MODE")
    directory = Path(require_env("INPUT_DIRECTORY"))
    identity = {key: require_env("INPUT_" + key.upper()) for key in IDENTITY_KEYS}
    if mode == "export-build":
        export_archives(directory, identity, "build")
    elif mode == "publish-build":
        publish_archives(directory, identity)
    elif mode == "prepare-e2e":
        export_archives(directory, identity, "e2e")
    elif mode == "load-e2e":
        load_e2e(directory, identity)
    else:
        raise ActionError("Invalid image archive mode")
