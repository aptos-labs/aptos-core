#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""Whether the authored proofs of Verex sources are still needed.

Every source that carries authored proofs (a `foo.proof.lean` beside a Move
file, a Rust `.spec.lean`, a VerexSL file with `verify … by` items) is verified
three times: as it is, with its proofs ignored (`--ignore-proofs`) at the
default budget, and with them ignored at RAISE times that budget. Per authored
proof the census reports the heartbeats of all three runs and a verdict:

- drop: the automatic verification succeeds within the default budget, and
  the proof costs more than KEEP_RATIO of it;
- keep, cheaper: it succeeds, but the proof costs at most KEEP_RATIO of it;
- keep, budget: it succeeds only within the raised budget;
- keep, needed: it does not succeed within the raised budget either.

Run from anywhere in the checkout after building `verex-move`, `verex-rust`,
`verex-e2e-tests`, the `move` exchange CLI and the `move-prover` binary
(ci profile): `python3 third_party/move/lean/scripts/proof-census.py
[--only <part of a path>] [--jobs N] [--json out.json] [--markdown out.md]`.
"""

import argparse
import concurrent.futures
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

LEAN_DIR = Path(__file__).resolve().parent.parent
REPO = LEAN_DIR.parent.parent.parent
PROVER_DIR = REPO / "third_party/move/move-prover"
KEEP_RATIO = 0.66
RAISE = 4
TIMEOUT = 45 * 60


def default_budget() -> int:
    """`verex.verifyHeartbeats`'s default, in `maxHeartbeats` units."""
    options = (LEAN_DIR / "verex-ir/VerexSL/Options.lean").read_text()
    match = re.search(r"verex\.verifyHeartbeats : Nat := \{\s*defValue := (\d+)", options)
    if not match:
        sys.exit("proof-census: cannot read the default budget from Options.lean")
    return int(match.group(1))


def latest_language_version() -> str:
    """The language version the Move Prover's test suite compiles with."""
    metadata = (REPO / "third_party/move/move-model/src/metadata.rs").read_text()
    match = re.search(r"LATEST_LANGUAGE_VERSION_VALUE: LanguageVersion = LanguageVersion::V(\d+)_(\d+)",
                      metadata)
    if not match:
        sys.exit("proof-census: cannot read the latest language version")
    return f"{match.group(1)}.{match.group(2)}"


@dataclass
class Source:
    """A source with authored proofs and how to verify it."""
    kind: str  # package | registry | file | rust | lean
    path: Path
    package: Path | None = None
    module: str = ""
    dev: bool = False
    spec: Path | None = None

    @property
    def label(self) -> str:
        return str(self.path.relative_to(REPO))


def git_files(*patterns: str) -> list[Path]:
    listing = subprocess.run(["git", "ls-files", *patterns], cwd=REPO, capture_output=True,
                             text=True, check=True).stdout.split()
    return [REPO / path for path in listing]


def package_of(path: Path) -> Path | None:
    for parent in path.parents:
        if (parent / "Move.toml").is_file():
            return parent
        if parent == REPO:
            return None
    return None


def discover() -> list[Source]:
    sources: list[Source] = []
    for proof in git_files("*.proof.lean"):
        move = proof.with_name(proof.name.removesuffix(".proof.lean") + ".move")
        if not move.is_file():
            continue
        if PROVER_DIR / "tests" in move.parents:
            sources.append(Source("registry", move))
            continue
        package = package_of(move)
        if package is None:
            sources.append(Source("file", move))
            continue
        declared = re.search(r"^\s*module\s+([\w:]+)", move.read_text(), re.MULTILINE)
        if not declared:
            continue
        manifest = (package / "Move.toml").read_text()
        sources.append(Source("package", move, package, declared.group(1),
                              dev="[dev-addresses]" in manifest))
    for spec in git_files("*.spec.lean"):
        rust = spec.with_name(spec.name.removesuffix(".spec.lean") + ".rs")
        if rust.is_file() and re.search(r"^\s*verify\b.*\bby\b", spec.read_text(), re.MULTILINE):
            sources.append(Source("rust", rust, spec=spec))
    for lean in git_files("third_party/move/lean/verex-e2e-tests/*.lean"):
        if lean.name.endswith((".proof.lean", ".spec.lean")):
            continue
        if re.search(r"^\s+verify\b.*\bby\b", lean.read_text(), re.MULTILINE):
            sources.append(Source("lean", lean))
    return sorted(sources, key=lambda source: source.label)


def lake_exe(package: str, exe: str) -> list[str]:
    root = LEAN_DIR / package
    return ["lake", "--dir", str(root), "env", str(root / ".lake/build/bin" / exe)]


def command(source: Source, ignore: bool, budget: int | None, log: Path,
            work: Path) -> tuple[list[str], Path, dict[str, str]]:
    """The command verifying `source`, its directory, and its environment."""
    # elan picks the toolchain by the current directory: ask for the pinned one.
    toolchain = (LEAN_DIR / "verex-move/lean-toolchain").read_text().strip()
    environment = dict(os.environ, APTOS_MOVE_CLI=str(REPO / "target/ci/move"),
                       ELAN_TOOLCHAIN=toolchain)
    options = [f"verex.stageLog={log}"]
    if budget is not None:
        options.append(f"verex.verifyHeartbeats={budget}")
    environment["VEREX_OPTIONS"] = ",".join(options)
    output = work / "rendering.lean"
    if source.kind == "package":
        args = lake_exe("verex-move", "verex-move") + [
            "verify", str(source.package), "--modules", source.module, "--output", str(output)]
        if source.dev:
            args.append("--dev")
        cwd = REPO
    elif source.kind == "file":
        args = lake_exe("verex-move", "verex-move") + ["verify", str(source.path), "--output",
                                                        str(output)]
        cwd = REPO
    elif source.kind == "registry":
        text = source.path.read_text()
        if "// use-aptos-stdlib" in text:
            dependencies = ["--dependency=../../../aptos-move/framework/move-stdlib/sources"]
        else:
            dependencies = ["--dependency=../move-stdlib/sources",
                            "--dependency=../move-stdlib/nursery/sources",
                            "--dependency=../extensions/move-table-extension/sources"]
        directives = [flag for line in text.splitlines() if line.startswith("// flag:")
                      for flag in shlex.split(line.removeprefix("// flag:"))]
        args = [str(REPO / "target/ci/move-prover"), *dependencies,
                "--named-addresses", "std=0x1", "extensions=0x2",
                f"--language-version={latest_language_version()}", "--lean", *directives,
                f"--output={work / 'out.bpl'}", str(source.path.relative_to(PROVER_DIR))]
        cwd = PROVER_DIR
    elif source.kind == "rust":
        args = lake_exe("verex-rust", "verex-rust") + [
            "verify", str(source.path), "--spec", str(source.spec), "--output", str(output)]
        cwd = REPO
    else:
        flags = [f"-Dweak.verex.stageLog={log}"]
        if budget is not None:
            flags.append(f"-Dweak.verex.verifyHeartbeats={budget}")
        if ignore:
            flags.append("-Dweak.verex.ignoreProofs=true")
        args = ["lake", "env", "lean", *flags, str(source.path)]
        return args, LEAN_DIR / "verex-e2e-tests", environment
    if ignore:
        args.append("--ignore-proofs")
    return args, cwd, environment


DONE = re.compile(r"^\d+ (.+): theorem done, (\d+) heartbeats, (\w+), (authored|automatic)$")
MODULE = re.compile(r"^\d+ module (.+): verification start$")


@dataclass
class Outcome:
    heartbeats: int
    outcome: str
    authored: bool


@dataclass
class Run:
    targets: dict[str, Outcome] = field(default_factory=dict)
    failure: str | None = None


def run(source: Source, ignore: bool, budget: int | None, keep: Path | None = None) -> Run:
    """Verify `source`; with `keep`, its rendering is copied there."""
    with tempfile.TemporaryDirectory(prefix="proof-census-") as directory:
        work = Path(directory)
        log = work / "stages.log"
        args, cwd, environment = command(source, ignore, budget, log, work)
        try:
            completed = subprocess.run(args, cwd=cwd, env=environment, capture_output=True,
                                       text=True, timeout=TIMEOUT)
        except subprocess.TimeoutExpired:
            return Run(failure="timeout")
        result = Run()
        module = ""
        for line in log.read_text().splitlines() if log.exists() else []:
            if started := MODULE.match(line):
                module = started.group(1)
            elif done := DONE.match(line):
                result.targets[f"{module}::{done.group(1)}" if module else done.group(1)] = Outcome(
                    int(done.group(2)), done.group(3), done.group(4) == "authored")
        if not result.targets and completed.returncode != 0:
            result.failure = (completed.stdout + completed.stderr).strip()[-400:]
        rendering = next((path for path in (work / "rendering.lean", work / "out.lean")
                          if path.exists()), None)
        if keep and rendering:
            keep.mkdir(parents=True, exist_ok=True)
            (keep / (source.label.replace("/", "__") + ".lean")).write_bytes(rendering.read_bytes())
        return result


def verdict(authored: Outcome, automatic: Outcome | None, raised: Outcome | None) -> str:
    if authored.outcome != "verified":
        return "authored proof fails"
    if automatic and automatic.outcome == "verified":
        if authored.heartbeats <= KEEP_RATIO * automatic.heartbeats:
            return "keep, cheaper"
        return "drop"
    if raised and raised.outcome == "verified":
        return "keep, budget"
    return "keep, needed"


def census(source: Source, raised: int, keep: Path | None) -> dict:
    normal = run(source, ignore=False, budget=None, keep=keep)
    automatic = run(source, ignore=True, budget=None)
    high = run(source, ignore=True, budget=raised)
    rows = []
    for target, authored in normal.targets.items():
        if not authored.authored:
            continue
        auto, more = automatic.targets.get(target), high.targets.get(target)
        rows.append({
            "target": target,
            "authored": vars(authored),
            "automatic": vars(auto) if auto else None,
            "raised": vars(more) if more else None,
            "verdict": verdict(authored, auto, more),
        })
    return {"source": source.label, "kind": source.kind, "rows": rows,
            "failures": {name: value.failure for name, value in
                         (("normal", normal), ("automatic", automatic), ("raised", high))
                         if value.failure}}


def heartbeats(outcome: dict | None) -> str:
    if not outcome:
        return "—"
    value = outcome["heartbeats"]
    text = f"{value / 1e9:.2f}G" if value >= 1e9 else f"{value / 1e6:.1f}M"
    return text if outcome["outcome"] == "verified" else f"{text} {outcome['outcome']}"


def markdown(results: list[dict], budget: int, raised: int) -> str:
    lines = [f"Default budget {budget * 1000 / 1e9:.1f}G heartbeats, raised {raised * 1000 / 1e9:.1f}G;"
             f" a proof is kept when it costs at most {KEEP_RATIO:.0%} of the automatic run.", "",
             "| Source | Target | Authored | Automatic | Raised | Verdict |",
             "|---|---|---|---|---|---|"]
    for result in results:
        for row in result["rows"]:
            lines.append(f"| `{result['source']}` | `{row['target']}` | {heartbeats(row['authored'])} | "
                         f"{heartbeats(row['automatic'])} | {heartbeats(row['raised'])} | {row['verdict']} |")
        for name, failure in result["failures"].items():
            lines.append(f"| `{result['source']}` | ({name} run) | | | | {failure.splitlines()[-1][:80]} |")
    counts: dict[str, int] = {}
    for result in results:
        for row in result["rows"]:
            counts[row["verdict"]] = counts.get(row["verdict"], 0) + 1
    lines += ["", ", ".join(f"{count} {name}" for name, count in sorted(counts.items()))]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--only", help="only sources whose path contains this")
    parser.add_argument("--jobs", type=int, default=4, help="sources verified at once")
    parser.add_argument("--json", type=Path, help="write the results here")
    parser.add_argument("--markdown", type=Path, help="write the table here")
    parser.add_argument("--list", action="store_true", help="list the sources and stop")
    parser.add_argument("--renderings", type=Path, help="keep the sources' renderings here")
    args = parser.parse_args()
    sources = [source for source in discover() if not args.only or args.only in source.label]
    if args.list:
        for source in sources:
            print(f"{source.kind:9} {source.label}")
        return
    budget = default_budget()
    raised = RAISE * budget
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(census, source, raised, args.renderings): source
                   for source in sources}
        results = []
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            print(f"proof-census: {result['source']}: {len(result['rows'])} authored proofs",
                  file=sys.stderr)
            results.append(result)
    results.sort(key=lambda result: result["source"])
    table = markdown(results, budget, raised)
    if args.json:
        args.json.write_text(json.dumps(results, indent=1) + "\n")
    if args.markdown:
        args.markdown.write_text(table)
    print(table)


if __name__ == "__main__":
    main()
