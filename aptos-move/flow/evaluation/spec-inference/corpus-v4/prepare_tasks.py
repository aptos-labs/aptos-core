#!/usr/bin/env python3
"""Prepare the tree each task starts from.

A task asks for the target's specification and nothing else. Everything the
target calls in its module comes with the complete reference contract of that
callee, as the corpus-v1.2 samples came with complete dependency contracts:
callers are verified against callee contracts, so a task without them would ask
for the callees too. The target's own reference -- its contract and the loop
invariants in its body -- is withheld, and so is the reference of every other
function of the module: a caller's contract can restate what the target does.
Spec functions are kept only where a kept specification uses them.

A task tree is the package with the target's module replaced by its reference
minus the withheld specifications. It is recorded as a preparation patch under
`patches/`, which the scheduler applies to the package, and the manifest pins
the patch and the tree it yields. Mutants were authored against the package; the
specifications kept before the target shift the code they anchor in, so their
offsets are rebased onto the task tree. Re-run `verify.py` afterwards to
re-validate them.

    python3 corpus-v4/prepare_tasks.py            # write patches, rebase mutants, update manifest
    python3 corpus-v4/prepare_tasks.py --verify   # fail if any of that would change

`build.py` and `build_references.py` must have run, and the pinned `move-flow`
must be on PATH: the call closure comes from its package inventory.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from difflib import SequenceMatcher
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EVALUATION = ROOT.parent
sys.path.insert(0, str(EVALUATION))

from harness.artifacts import copy_snapshot, sha256_file, tree_hash, write_json  # noqa: E402
from harness.identifiers import module_name, require_plain_name  # noqa: E402
from harness.move_source import closing_brace, mask_comments_and_strings  # noqa: E402
from harness.mutants import _implementation_offset  # noqa: E402
from harness.prepare import _verify_patch_reproduction, _write_git_patch  # noqa: E402

PACKAGE = ROOT / "package"
REFERENCES = ROOT / "references"
BUILD = REFERENCES / "build"
PATCHES = ROOT / "patches"
MUTANT_SETS = ("mutants", "mutants-scoring")
SELECTION_POLICY = EVALUATION / "config" / "corpus-selection.json"


def reference_files(module: str) -> list[str]:
    """The source files a module's reference patch touches."""
    text = (REFERENCES / f"{module}.patch").read_text(encoding="utf-8")
    return re.findall(r"^\+\+\+ b/(\S+)$", text, flags=re.M)


def file_module(relative: str) -> str:
    """The module a corpus source file holds: `x.move` and `x.spec.move` hold `x`."""
    return Path(relative).name.removesuffix(".move").removesuffix(".spec")


@dataclass
class Span:
    """A brace-delimited construct of a Move source: `[start, end)`."""

    start: int
    end: int
    kind: str  # "function", "inline", "named", "module", "container"
    name: str | None


def spans_of(text: str) -> list[Span]:
    """Functions, and `spec` blocks: inline in a function body, `spec NAME`
    (with any `proof` block after it), `spec module`, and the `spec ADDR::MODULE`
    container of a specification file."""
    masked = mask_comments_and_strings(text)
    specs: list[Span] = []
    for keyword in re.finditer(r"\bspec\b", masked):
        if any(s.start <= keyword.start() < s.end for s in specs if s.kind != "container"):
            continue
        opening = masked.find("{", keyword.end())
        end = closing_brace(masked, opening) if opening >= 0 else None
        if end is None:
            raise SystemExit(f"unterminated spec block at offset {keyword.start()}")
        header = masked[keyword.end():opening].strip()
        if "::" in header.split("(")[0]:
            specs.append(Span(keyword.start(), end, "container", header))
            continue
        proof = re.match(r"\s*proof\s*\{", masked[end:])
        if proof:
            end = closing_brace(masked, end + proof.end() - 1)
            if end is None:
                raise SystemExit(f"unterminated proof block after spec {header}")
        if header == "module":
            specs.append(Span(keyword.start(), end, "module", None))
        elif header == "":
            specs.append(Span(keyword.start(), end, "inline", None))
        else:
            name = re.match(r"\w+", header)
            if name is None or name.group(0) in ("fun", "schema"):
                raise SystemExit(f"unexpected spec item: spec {header}")
            specs.append(Span(keyword.start(), end, "named", name.group(0)))
    functions = []
    for header in re.finditer(r"\bfun\s+(\w+)", masked):
        if any(s.start <= header.start() < s.end for s in specs):
            continue  # a spec function
        brace = masked.find("{", header.end())
        semicolon = masked.find(";", header.end())
        if brace < 0 or 0 <= semicolon < brace:
            continue  # native
        functions.append(Span(header.start(), closing_brace(masked, brace), "function",
                              header.group(1)))
    return functions + specs


def spec_functions(text: str, block: Span) -> list[Span]:
    """The `fun` declarations of a `spec module` block."""
    masked = mask_comments_and_strings(text)
    opening = masked.find("{", block.start)
    found = []
    for header in re.finditer(r"\bfun\s+(\w+)", masked[:block.end]):
        if header.start() <= opening:
            continue
        brace = masked.find("{", header.end())
        found.append(Span(header.start(), closing_brace(masked, brace), "specfun", header.group(1)))
    return found


def mentioned(names: set[str], text: str) -> set[str]:
    masked = mask_comments_and_strings(text)
    return {name for name in names if re.search(rf"\b{re.escape(name)}\b", masked)}


def task_text(package: str, reference: str, kept: set[str] | None,
              functions: set[str]) -> str:
    """`package` with the specification the reference adds for the functions in
    `kept` (every function when `None`), and the spec functions that uses.
    `functions` are the module's functions: a `spec NAME` block of another
    `NAME` specifies a struct, and is kept.

    A reference only inserts lines. Each inserted line belongs to what it is
    written in: a function's contract or loop, a spec function, or the shell
    of a `spec module` block. Comments and blank lines go with the next
    owned line of their insertion.
    """
    package_lines = package.split("\n")
    reference_lines = reference.split("\n")
    inserted: set[int] = set()
    for tag, i1, i2, j1, j2 in SequenceMatcher(
        None, package_lines, reference_lines, autojunk=False
    ).get_opcodes():
        if tag == "insert":
            inserted.update(range(j1, j2))
        elif tag != "equal":
            raise SystemExit("a reference changes a line of the package; it may only insert")

    masked = mask_comments_and_strings(reference)
    starts = [0]
    for line in reference_lines:
        starts.append(starts[-1] + len(line) + 1)
    spans = spans_of(reference)
    members = {id(m): (block, m) for block in spans if block.kind == "module"
               for m in spec_functions(reference, block)}
    declared = {m.name: reference[m.start:m.end] for _, m in members.values()}

    def owner(index: int) -> tuple[str, object] | None:
        code = masked[starts[index]:starts[index] + len(reference_lines[index])]
        if not code.strip():
            return None
        offset = starts[index] + len(code) - len(code.lstrip())
        for span in spans:
            if span.kind == "function" and span.start <= offset < span.end:
                return ("function", span.name)
        for block, member in members.values():
            if member.start <= offset < member.end:
                return ("specfun", member.name)
        inner = [s for s in spans if s.kind in ("named", "module", "inline")
                 and s.start <= offset < s.end]
        if inner:
            span = min(inner, key=lambda s: s.end - s.start)
            if span.kind == "named":
                return ("function", span.name) if span.name in functions else ("other", span.name)
            return ("shell", (span.start, span.end))
        return ("other", None)

    owners: dict[int, tuple[str, object]] = {}
    runs: list[list[int]] = []
    for index in sorted(inserted):
        if runs and runs[-1][-1] == index - 1:
            runs[-1].append(index)
        else:
            runs.append([index])
    for run in runs:
        found = [owner(index) for index in run]
        following = found[:]
        for k in range(len(run) - 2, -1, -1):
            following[k] = following[k] or following[k + 1]
        preceding = None
        for k, index in enumerate(run):
            preceding = found[k] or preceding
            owners[index] = following[k] or preceding or ("other", None)

    every = kept is None
    def keeps(own: tuple[str, object]) -> bool:
        kind, name = own
        return kind == "other" or (kind == "function" and (every or name in kept))

    used_text = "\n".join(reference_lines[i] for i in sorted(inserted) if keeps(owners[i]))
    used = set(declared) if every else mentioned(set(declared), used_text)
    frontier = set(used)
    while frontier:
        reached = set().union(*(mentioned(set(declared), declared[name]) for name in frontier))
        frontier = reached - used
        used |= reached

    def kept_line(index: int) -> bool:
        kind, name = owners[index]
        if kind == "specfun":
            return name in used
        if kind == "shell":
            start, end = name
            return any(
                start <= starts[other] < end and owners[other][0] != "shell"
                and kept_line(other)
                for other in inserted
            )
        return keeps(owners[index])

    lines: list[str] = []
    for index, line in enumerate(reference_lines):
        if index in inserted:
            if not kept_line(index):
                continue
            # A blank line separated what is no longer there.
            if not every and not line.strip() and (
                not lines or not lines[-1].strip() or lines[-1].rstrip().endswith("{")
            ):
                continue
        lines.append(line)
    return "\n".join(lines)


def call_closures(module: str) -> dict[str, set[tuple[str, str]]]:
    """Each function of the module mapped to the `(module, function)`s it
    calls, transitively."""
    with tempfile.TemporaryDirectory(prefix="corpus-v4-inventory-") as temporary:
        output = Path(temporary) / "inventory.json"
        result = subprocess.run(
            ["move-flow", "experiment", "inventory-package", "--package", str(BUILD / module),
             "--output", str(output), "--selection-policy", str(SELECTION_POLICY)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode != 0:
            raise SystemExit(f"{module}: package inventory failed:\n{result.stderr[-2000:]}")
        candidates = json.loads(output.read_text(encoding="utf-8"))["candidates"]
    closures = {}
    for candidate in candidates:
        if candidate["granularity"] != "function":
            continue
        if candidate["module"].rsplit("::", 1)[-1] != module:
            continue
        closures[candidate["function"]] = {
            tuple(callee.rsplit("::", 2)[-2:])
            for callee in candidate["transitive_called_function_dependencies"]
        }
    return closures


def rebase_anchor(case: dict, package_text: str, task_source: str) -> bool:
    """Point a mutant's anchor into the task tree; whether it changed."""
    anchor = case["anchor"]

    def matches(text: str, offset: int) -> bool:
        fragment = text[offset:offset + anchor["length"]]
        return (len(fragment) == anchor["length"]
                and hashlib.sha256(fragment.encode("utf-8")).hexdigest() == anchor["sha256"])

    if matches(task_source, anchor["offset"]):
        return False  # already rebased, or before every kept specification
    if not matches(package_text, anchor["offset"]):
        raise SystemExit(f"mutant {case['mutant_id']} matches neither the package nor the task tree")
    moved = _implementation_offset(package_text, task_source, anchor["offset"])
    if moved is None or not matches(task_source, moved):
        raise SystemExit(f"mutant {case['mutant_id']} does not survive rebasing")
    anchor["offset"] = moved
    return True


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true",
                        help="fail if a patch, a mutant anchor or the manifest would change")
    args = parser.parse_args()

    manifest_path = ROOT / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    closures: dict[str, dict[str, set[str]]] = {}
    changed: list[str] = []
    PATCHES.mkdir(exist_ok=True)

    for record in manifest["records"]:
        task_id = require_plain_name(record["task_id"], "task_id")
        module = module_name(record["module"])
        function = require_plain_name(record["function"], "function")
        files = reference_files(module)
        sources = {relative: ((PACKAGE / relative).read_text(encoding="utf-8"),
                              (BUILD / module / relative).read_text(encoding="utf-8"))
                   for relative in files}
        # A specification file specifies the functions of its sibling source.
        functions = {
            relative: {span.name for span in spans_of(
                (PACKAGE / relative.replace(".spec.move", ".move")).read_text(encoding="utf-8"))
                if span.kind == "function"}
            for relative in files
        }
        if module not in closures:
            closures[module] = call_closures(module)
            # Keeping every reference specification must give back the
            # reference: then every inserted line has an owner.
            for relative, (package_text, reference) in sources.items():
                if task_text(package_text, reference, None, functions[relative]) != reference:
                    raise SystemExit(f"{module}: {relative} has reference lines no item owns")
        if function not in closures[module]:
            raise SystemExit(f"{task_id}: the inventory does not list {module}::{function}")
        callees = closures[module][function] - {(module, function)}
        texts = {
            relative: task_text(package_text, reference,
                                {name for owner, name in callees if owner == file_module(relative)},
                                functions[relative])
            for relative, (package_text, reference) in sources.items()
        }
        kept = sorted(f"{owner}::{name}" for owner, name in callees
                      if any(file_module(relative) == owner for relative in files))
        target_file = next(relative for relative in files
                           if file_module(relative) == module and relative.endswith(".move")
                           and not relative.endswith(".spec.move"))

        committed = PATCHES / f"{task_id}.patch"
        with tempfile.TemporaryDirectory(prefix=f"corpus-v4-{task_id}-") as temporary:
            prepared = Path(temporary) / "package"
            copy_snapshot(PACKAGE, prepared)
            for relative, text in texts.items():
                (prepared / relative).write_text(text, encoding="utf-8")
            patch = Path(temporary) / f"{task_id}.patch"
            if all(texts[relative] == sources[relative][0] for relative in files):
                # Nothing the target calls in its module has a contract to give.
                patch.write_text("", encoding="utf-8")
            else:
                _write_git_patch(PACKAGE, prepared, patch)
                _verify_patch_reproduction(PACKAGE, prepared, patch, task_id)
            prepared_sha256 = tree_hash(prepared)
            if not committed.is_file() or committed.read_bytes() != patch.read_bytes():
                changed.append(f"patches/{task_id}.patch")
                if not args.verify:
                    shutil.copyfile(patch, committed)

        fields = {
            "preparation_patch": f"patches/{task_id}.patch",
            "preparation_patch_sha256": sha256_file(committed) if committed.is_file() else None,
            "prepared_sha256": prepared_sha256,
            "reference_contracts": kept,
        }
        if any(record.get(key) != value for key, value in fields.items()):
            changed.append(f"manifest record {task_id}")
            record.update(fields)

        for mutant_set in MUTANT_SETS:
            path = ROOT / mutant_set / task_id / "mutants.json"
            data = json.loads(path.read_text(encoding="utf-8"))
            if any(case["file"] != target_file for case in data["mutants"]):
                raise SystemExit(f"{task_id}: a mutant edits a file other than {target_file}")
            if any([rebase_anchor(case, sources[target_file][0], texts[target_file])
                    for case in data["mutants"]]):
                changed.append(f"{mutant_set}/{task_id}")
                if not args.verify:
                    write_json(path, data)
        print(f"{task_id}: {len(kept)} callee contract(s) {kept}", flush=True)

    if args.verify:
        if changed:
            raise SystemExit("would change:\n  " + "\n  ".join(changed))
        print("every task tree, patch, mutant anchor and manifest record is current")
        return
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"{len(changed)} change(s)")


if __name__ == "__main__":
    main()
