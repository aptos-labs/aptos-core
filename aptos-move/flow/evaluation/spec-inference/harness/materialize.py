"""Materialize one task from the shared corpus package and its overlay patch."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from .artifacts import copy_snapshot, tree_hash, validate_symlinks


def materialize_task(
    shared_package: Path,
    preparation_patch: Path,
    destination: Path,
    expected_sha256: str | None = None,
) -> str:
    """Create an independent task tree and verify its deterministic identity."""
    if destination.exists():
        raise FileExistsError(f"refusing to overwrite materialized task: {destination}")
    if not (shared_package / "Move.toml").is_file():
        raise ValueError(f"shared package is missing Move.toml: {shared_package}")
    if not preparation_patch.is_file():
        raise ValueError(f"preparation patch is missing: {preparation_patch}")
    copy_snapshot(shared_package, destination)
    if preparation_patch.stat().st_size:
        result = subprocess.run(
            ["git", "apply", "--whitespace=nowarn", str(preparation_patch)],
            cwd=destination,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            shutil.rmtree(destination)
            raise ValueError(
                f"preparation patch does not apply to shared package: "
                f"{result.stderr.strip()}"
            )
    # `copy_snapshot` validated the source, but the patch is applied after
    # that and `git apply` can create a symlink. Screening later copies this
    # tree with `symlinks=False`, which would dereference an escaping link and
    # ingest host files, and a manifest recording the resulting hash would make
    # that tree look canonical. Re-check what the patch actually produced.
    try:
        validate_symlinks(destination)
    except ValueError:
        shutil.rmtree(destination)
        raise
    actual = tree_hash(destination)
    if expected_sha256 is not None and actual != expected_sha256:
        shutil.rmtree(destination)
        raise ValueError(
            f"materialized task hash mismatch: expected {expected_sha256}, got {actual}"
        )
    return actual
