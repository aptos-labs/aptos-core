#!/usr/bin/env python3
"""Compose the corpus per task, for reading rather than for running.

The corpus is stored as it must be for a public repository and a blind round:
the generated Move sources are gitignored, a reference specification is a
zero-context patch, and a mutant is an offset, a length and a digest. None of
that reads well. This script puts everything about one task side by side in an
untracked directory:

    inspect/
      README.md                     index of every task
      tasks/TASK/
        README.md                   target, why it is in the corpus, its mutants
        MODULE.move                 the module as the agent receives it
        MODULE.reference.move       the module with the reference specification
        reference.patch             the committed specification patch
        mutants/MUTANT.diff         the implementation change, as a unified diff

It needs the generated package, so `python3 corpus-v3/build.py` (which needs
access to the private Etna repository) must have run. Nothing here is written
inside the tracked tree: the output directory is gitignored, and the reference
packages it reads are assembled under `references/build/` exactly as
`build_references.py` leaves them.

    python3 corpus-v3/compose.py                  # into corpus-v3/inspect/
    python3 corpus-v3/compose.py --output DIR
    python3 corpus-v3/compose.py --task MM-min-013
"""

from __future__ import annotations

import argparse
import difflib
import json
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PACKAGE = ROOT / "package"
PATCHES = ROOT / "references"
MUTANTS = ROOT / "mutants"
DEFAULT_OUTPUT = ROOT / "inspect"

sys.path.insert(0, str(ROOT.parent))
sys.path.append(str(ROOT))

from build_references import assemble  # noqa: E402
from harness.mutants import apply_mutant  # noqa: E402


def module_file(module: str) -> Path:
    """Locate the generated source of a fully qualified module."""
    name = module.rsplit("::", 1)[-1]
    matches = sorted(PACKAGE.glob(f"sources/*/{name}.move"))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one source for `{module}`, found {matches}")
    return matches[0]


def mutant_diff(case: dict, relative: str, pristine: str) -> str:
    """Render one mutant as a unified diff of the file it rewrites."""
    with tempfile.TemporaryDirectory(prefix="move-inference-inspect-") as temporary:
        package = Path(temporary) / "package"
        target = package / relative
        target.parent.mkdir(parents=True)
        target.write_text(pristine, encoding="utf-8")
        apply_mutant(package, PACKAGE, case)
        mutated = target.read_text(encoding="utf-8")
    return "".join(
        difflib.unified_diff(
            pristine.splitlines(keepends=True),
            mutated.splitlines(keepends=True),
            fromfile=f"a/{relative}",
            tofile=f"b/{relative}",
            n=3,
        )
    )


def task_readme(record: dict, mutants: list[dict], reference: Path | None) -> str:
    lines = [
        f"# {record['task_id']}",
        "",
        f"- target: `{record['target']}`",
        f"- provenance: {record['provenance']}",
        f"- guessability: {record['guessability']}",
        f"- screening status: {record['screening_status']}",
        f"- required contract categories: {', '.join(record['required_contract_categories'])}",
        f"- feature strata: {', '.join(record['feature_strata'])}",
        "",
        "## Why it is in the corpus",
        "",
        record["why"],
        "",
        "## Reference specification",
        "",
        (
            f"`{reference.name}` holds the module with the hand-authored specification; "
            "`reference.patch` is the committed form."
            if reference
            else "No reference specification is committed for this module."
        ),
        "",
        "## Mutants",
        "",
    ]
    if not mutants:
        lines.append("No mutants are authored for this task.")
    else:
        lines += ["| mutant | obligation | essential | reference outcome |", "|---|---|---|---|"]
        for case in mutants:
            lines.append(
                f"| [`{case['mutant_id']}`](mutants/{case['mutant_id']}.diff) "
                f"| {case['obligation_category']} | {case.get('essential')} "
                f"| {case.get('validated', {}).get('outcome', 'unvalidated')} |"
            )
    return "\n".join(lines) + "\n"


def compose_task(record: dict, output: Path, assembled: dict[str, Path]) -> dict:
    task_id = record["task_id"]
    module = record["module"]
    source = module_file(module)
    relative = source.relative_to(PACKAGE).as_posix()
    directory = output / "tasks" / task_id
    directory.mkdir(parents=True)
    shutil.copyfile(source, directory / source.name)

    name = module.rsplit("::", 1)[-1]
    reference = None
    patch = PATCHES / f"{name}.patch"
    if patch.is_file():
        if name not in assembled:
            assembled[name] = assemble(name)
        reference = directory / f"{name}.reference.move"
        shutil.copyfile(assembled[name] / relative, reference)
        shutil.copyfile(patch, directory / "reference.patch")

    mutants: list[dict] = []
    manifest = MUTANTS / task_id / "mutants.json"
    if manifest.is_file():
        mutants = json.loads(manifest.read_text(encoding="utf-8"))["mutants"]
        pristine = source.read_text(encoding="utf-8")
        (directory / "mutants").mkdir()
        for case in mutants:
            if case["file"] != relative:
                raise SystemExit(
                    f"mutant {case['mutant_id']} rewrites {case['file']}, "
                    f"but the target module lives in {relative}"
                )
            (directory / "mutants" / f"{case['mutant_id']}.diff").write_text(
                mutant_diff(case, relative, pristine), encoding="utf-8"
            )
    (directory / "README.md").write_text(
        task_readme(record, mutants, reference), encoding="utf-8"
    )
    return {"task_id": task_id, "record": record, "mutants": len(mutants), "reference": reference is not None}


def index(entries: list[dict], manifest: dict) -> str:
    lines = [
        "# Corpus V3, composed for inspection",
        "",
        f"Corpus `{manifest['corpus']}`, {len(entries)} tasks. Generated by "
        "`corpus-v3/compose.py`; this directory is untracked and can be deleted.",
        "",
        "| task | target | guess | status | reference | mutants |",
        "|---|---|---|---|---|---|",
    ]
    for entry in entries:
        record = entry["record"]
        lines.append(
            f"| [`{record['task_id']}`](tasks/{record['task_id']}/README.md) "
            f"| `{record['target']}` | {record['guessability']} "
            f"| {record['screening_status']} | {'yes' if entry['reference'] else 'no'} "
            f"| {entry['mutants']} |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--task", action="append", help="compose only this task (repeatable)")
    args = parser.parse_args()

    if not (PACKAGE / "Move.toml").is_file() or not (PACKAGE / "sources").is_dir():
        raise SystemExit(
            f"the corpus package is not built: {PACKAGE}\n"
            "run `python3 corpus-v3/build.py` first -- it needs the private Etna sources"
        )
    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    records = manifest["records"]
    if args.task:
        known = {record["task_id"] for record in records}
        unknown = sorted(set(args.task) - known)
        if unknown:
            raise SystemExit(f"unknown task(s): {', '.join(unknown)}")
        records = [record for record in records if record["task_id"] in args.task]

    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    assembled: dict[str, Path] = {}
    entries = [compose_task(record, output, assembled) for record in records]
    (output / "README.md").write_text(index(entries, manifest), encoding="utf-8")
    print(f"composed {len(entries)} task(s) into {output}")


if __name__ == "__main__":
    main()
