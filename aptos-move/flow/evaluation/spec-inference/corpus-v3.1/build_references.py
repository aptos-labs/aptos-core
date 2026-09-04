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
import re
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


#: The only pragma a reference specification needs. `opaque` is the proof
#: boundary -- it controls how callers reason about the function and does not
#: disable verification of its body -- and every committed reference uses it.
ALLOWED_REFERENCE_PRAGMAS = frozenset({"opaque"})



def _inserted_blocks(lines: list[str]) -> list[str]:
    """Each contiguous run of added lines, as the text it inserts.

    The unit is the run rather than the patch: a run is inserted at one point
    in the file, so it is the thing whose lexical effect on its surroundings
    can be judged. Two runs are two insertions at two places, and text that
    spans them is not one construct to the compiler.
    """
    blocks: list[str] = []
    current: list[str] = []
    for line in lines:
        if line.startswith("+") and not line.startswith("+++"):
            current.append(line[1:])
        elif current:
            blocks.append("\n".join(current))
            current = []
    if current:
        blocks.append("\n".join(current))
    return blocks


def _without_comments(text: str, patch_name: str) -> str:
    """One inserted block as the Move lexer sees it, with comments removed.

    Also the check that the block is lexically self-contained, which is what
    makes reading only the additions sound in the first place. A patch that
    adds and never removes can still change untouched code: `git apply` will
    open a block comment above the target and close it below, leaving the
    original implementation commented out and a replacement in its place, with
    every line of the patch an addition. Balance across the whole patch does
    not catch that -- the `/*` and the `*/` are exactly what the two runs add.
    A run that closes what it did not open, or leaves open what it did, is
    refused; the untouched lines then lex as they did before.

    Whole runs rather than single lines, because a comment spans newlines: a
    per-line scan reads `pragma /* c` and `*/ intrinsic;` as two unremarkable
    lines and the compiler reads them as one pragma. Block comments nest in
    Move, so the depth is a count and not a flag, and `//` inside one is inert.
    """
    out: list[str] = []
    depth = 0
    index = 0
    while index < len(text):
        pair = text[index : index + 2]
        if depth == 0 and pair == "//":
            newline = text.find("\n", index)
            out.append(" ")
            index = len(text) if newline < 0 else newline
            continue
        if pair == "/*":
            if depth == 0:
                # A separator, so that removing `/*c*/` from `pragma/*c*/x`
                # leaves two tokens rather than manufacturing one.
                out.append(" ")
            depth += 1
            index += 2
            continue
        if pair == "*/":
            if depth == 0:
                raise SystemExit(
                    f"{patch_name} closes a block comment it did not open, which "
                    "would end a comment in the untouched source above it and "
                    "expose whatever follows as code"
                )
            depth -= 1
            index += 2
            continue
        if depth == 0:
            out.append(text[index])
        index += 1
    if depth:
        raise SystemExit(
            f"{patch_name} opens a block comment it does not close, which would "
            "comment out the untouched executable code below it -- a reference "
            "patch that only adds lines can still replace an implementation "
            "that way"
        )
    return "".join(out)


def _require_specification_only(patch: Path) -> None:
    """Refuse a reference patch that changes executable code.

    A reference is the corpus package with one module's specification written
    in; that is what makes its proof evidence about *this* task. A patch that
    removes or rewrites a line could prove a different implementation, and the
    screen would record the result as this target's solvability.

    The module docstring has always said these patches only add lines. This
    checks it rather than trusting it: `git apply` is perfectly willing to
    remove one.
    """
    lines = patch.read_text(encoding="utf-8").splitlines()
    removals = [
        line for line in lines if line.startswith("-") and not line.startswith("---")
    ]
    if removals:
        raise SystemExit(
            f"{patch.name} removes {len(removals)} line(s); a reference patch adds "
            "specification and never changes executable code, or its proof is "
            "about a different implementation"
        )
    # Adding lines is not enough of a restriction on its own: a patch that only
    # adds can still add something that excuses the prover from its
    # obligations, and the target then proves while establishing less than the
    # contract claims.
    #
    # This allows rather than forbids, deliberately. A list of banned pragmas
    # has to be extended every time someone finds another way to be excused --
    # `verify = false`, `aborts_if_is_partial`, `intrinsic`, and whatever comes
    # next. A reference adds a specification, and the only pragma a
    # specification here needs is the proof boundary, so anything else is
    # refused whether or not its effect was anticipated.
    # Comment removal is only sound if the text has no string literals: `//`
    # inside `b"http://x"` is not a comment, and treating it as one hides
    # whatever follows on that line. A reference adds a specification and no
    # committed one contains a quote, so rather than lex string literals
    # correctly the guard refuses text it cannot confidently read. That closes
    # the whole class of literal-versus-comment ambiguity instead of the one
    # spelling that prompted it. Only the double quote: Move has no character
    # literal, and an apostrophe belongs to prose in a doc comment.
    for block in _inserted_blocks(lines):
        if '"' in block:
            raise SystemExit(
                f"{patch.name} adds a string literal; a reference specification "
                "does not need one, and its quoting would make the scan for "
                "verification-suppressing directives unreliable"
            )
        added = _without_comments(block, patch.name)
        # Every `pragma` in the block, and every property of each: a statement
        # can carry a comma-separated list, and several statements can share a
        # line. Reading the first of either lets an allowed pragma escort a
        # forbidden one.
        for keyword in re.finditer(r"\bpragma\b", added):
            statement = added[keyword.end():].split(";", 1)[0]
            for clause in statement.split(","):
                name = re.match(r"\s*([a-z_]+)", clause, flags=re.DOTALL)
                if name and name.group(1) not in ALLOWED_REFERENCE_PRAGMAS:
                    raise SystemExit(
                        f"{patch.name} adds `pragma {name.group(1)}`, which is "
                        "not one a reference specification needs; only "
                        + ", ".join(sorted(ALLOWED_REFERENCE_PRAGMAS))
                        + " is allowed, so a pragma that excuses the prover "
                        "cannot be introduced by a patch"
                    )
        if re.search(r"\bassume\b", added):
            raise SystemExit(
                f"{patch.name} adds an `assume`, which discharges the contract "
                "by assumption rather than proving it"
            )


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
    _require_specification_only(patch)
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
