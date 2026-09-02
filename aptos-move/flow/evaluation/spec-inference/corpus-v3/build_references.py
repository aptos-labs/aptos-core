#!/usr/bin/env python3
"""Materialize the reference packages from the generated corpus package.

A reference is the corpus package with one module's complete specification
written in. It must be a whole package because the prover verifies packages,
which means a committed reference would carry Etna function bodies -- and
`aptos-core` is public while Etna is not.

So only the specification is committed, as a zero-context patch under
`references/`. Those patches add lines and never remove or modify one, so they
contain our specification text and line numbers, and no Etna source. The
assembled packages land in `references/build/`, which is gitignored exactly as
`package/sources/` is.

    python3 corpus-v3/build_references.py            # assemble every reference
    python3 corpus-v3/build_references.py --verify   # assemble and check digests
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PACKAGE = ROOT / "package"
PATCHES = ROOT / "references"
BUILD = PATCHES / "build"


def module_names() -> list[str]:
    return sorted(p.stem for p in PATCHES.glob("*.patch"))


def assemble(module: str) -> Path:
    """Copy the package and apply this module's specification patch."""
    if not (PACKAGE / "Move.toml").is_file():
        raise SystemExit(
            f"the corpus package is not built: {PACKAGE}\n"
            "run `python3 corpus-v3/build.py` first -- it needs the private Etna sources"
        )
    destination = BUILD / module
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(PACKAGE, destination, ignore=shutil.ignore_patterns("build"))
    patch = PATCHES / f"{module}.patch"
    result = subprocess.run(
        ["git", "apply", "--unidiff-zero", "--whitespace=nowarn", str(patch)],
        cwd=destination,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit(
            f"specification patch does not apply to the generated package: {patch}\n"
            f"{result.stderr.strip()}\n"
            "the package was probably regenerated from different Etna sources"
        )
    return destination


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verify",
        action="store_true",
        help="compare each assembled tree against the digest its mutants recorded",
    )
    parser.add_argument("--module", help="assemble only this module")
    args = parser.parse_args()

    sys.path.insert(0, str(ROOT.parent))
    from harness.artifacts import tree_hash

    modules = [args.module] if args.module else module_names()
    if not modules:
        raise SystemExit(f"no specification patches under {PATCHES}")

    recorded: dict[str, set[str]] = {}
    for manifest in sorted((ROOT / "mutants").glob("*/mutants.json")):
        data = json.loads(manifest.read_text(encoding="utf-8"))
        digest = data.get("reference_sha256")
        if digest:
            recorded.setdefault(digest, set()).add(data["task_id"])

    failures = 0
    for module in modules:
        destination = assemble(module)
        digest = tree_hash(destination)
        note = ""
        if args.verify:
            if digest in recorded:
                note = f"  matches {len(recorded[digest])} task(s)"
            else:
                note = "  *** no mutant manifest records this digest ***"
                failures += 1
        print(f"{module}: {destination.relative_to(ROOT.parent)}  {digest[:12]}{note}")
    if failures:
        raise SystemExit(
            f"{failures} reference(s) do not match any recorded digest; "
            "re-run harness.validate_mutants for the affected tasks"
        )


if __name__ == "__main__":
    main()
