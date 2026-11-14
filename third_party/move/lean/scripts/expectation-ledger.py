#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""The expectation ledger: recorded test expectations against a reference tree.

Every fixture of the Lean tree and of the Move Prover's lean gate is compared
with the same fixture at a reference revision (by default `81ad86eb3ee`, the
last tree before the certified carriers). For each the ledger reports whether
the source still is what it was, whether its baseline is absent, unchanged or
changed, how many error lines the baseline held before and holds now, and at
which of the named intermediate revisions the current content first appears.

A clean fixture records no baseline at all, so a baseline that appeared means
the fixture started failing and one that vanished means it became clean.

Run from anywhere in the checkout; the script only reads the working tree and
`git show` / `git ls-tree`, and builds nothing:

    python3 third_party/move/lean/scripts/expectation-ledger.py --format md
    python3 third_party/move/lean/scripts/expectation-ledger.py --format json
    python3 third_party/move/lean/scripts/expectation-ledger.py \
        --check third_party/move/lean/doc/status/expectation-ledger.json
"""

import argparse
import hashlib
import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

REFERENCE = "81ad86eb3ee"

# The revisions that can carry a change, oldest first. The ledger reports the
# first one at which an entry already looks as it looks now.
ATTRIBUTION = [
    ("9d015f41b27", "certified carriers"),
    ("d5b620edea0", "Rust verification"),
    ("78dcaf9df45", "current tip"),
]

LEAN = "third_party/move/lean"
PROVER = "third_party/move/move-prover/tests/sources"

# A scope is one tree of fixtures. `sources` are the extensions a fixture's
# source may have, `baseline` the suffix its recorded expectation has.
#
# `clean_visible` says how a fixture without a baseline is found. In a fixture
# tree (the e2e suites, the exporter's units, the Prover's sources) every
# source in a directory that holds at least one baseline is a fixture, so a
# clean one is visible. In the other trees the sources are assertion-style
# unit tests, and only those with a baseline are expectations.
SCOPES = [
    {
        "package": "verex-ir",
        "root": f"{LEAN}/verex-ir/VerexSL/Tests",
        "sources": (".lean",),
        "baseline": ".exp",
        "clean_visible": False,
    },
    {
        "package": "verex-move",
        "root": f"{LEAN}/verex-move/Tests",
        "sources": (".lean",),
        "baseline": ".exp",
        "clean_visible": False,
    },
    {
        "package": "verex-e2e-tests",
        "root": f"{LEAN}/verex-e2e-tests/VerexE2ETests",
        "sources": (".lean", ".move", ".rs"),
        "baseline": ".exp",
        "clean_visible": True,
    },
    {
        "package": "verex-rust-exporter",
        "root": f"{LEAN}/verex-rust/rust-exporter/tests",
        "sources": (".rs",),
        "baseline": ".exp.json",
        "clean_visible": True,
    },
    {
        "package": "move-prover-lean",
        "root": PROVER,
        "sources": (".move",),
        "baseline": ".lean_exp",
        "clean_visible": True,
    },
]

# The Prover's harness records no baseline for these two trees.
PROVER_WITHOUT_BASELINE = ("/diem-framework/", "/move-stdlib/")


def run(args):
    """Run a git command in the checkout and return its stdout."""
    return subprocess.run(args, cwd=str(REPO), check=True,
                          stdout=subprocess.PIPE).stdout


REPO = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], check=True,
                           stdout=subprocess.PIPE, text=True).stdout.strip())


def blob_hash(data):
    """The hash git would give this content, so working tree and tree agree."""
    header = b"blob %d\0" % len(data)
    return hashlib.sha1(header + data).hexdigest()


def tree_of(revision, roots):
    """The blob hash of every file under `roots` at `revision`."""
    out = run(["git", "ls-tree", "-r", revision, "--"] + list(roots))
    blobs = {}
    for line in out.decode().splitlines():
        meta, path = line.split("\t", 1)
        blobs[path] = meta.split()[2]
    return blobs


def worktree_of(roots):
    """The blob hash of every tracked or untracked file under `roots`."""
    listed = run(["git", "ls-files", "--cached", "--others",
                  "--exclude-standard", "--"] + list(roots))
    blobs = {}
    for path in listed.decode().splitlines():
        full = REPO / path
        if full.is_file():
            blobs[path] = blob_hash(full.read_bytes())
    return blobs


def contents_of(revision, paths):
    """The text of each path at `revision`, missing ones left out."""
    texts = {}
    for path in paths:
        done = subprocess.run(["git", "show", f"{revision}:{path}"],
                              cwd=str(REPO), stdout=subprocess.PIPE,
                              stderr=subprocess.DEVNULL)
        if done.returncode == 0:
            texts[path] = done.stdout.decode("utf-8", "replace")
    return texts


def error_lines(text):
    """Lines holding `error`, the count scripts/exp-error-delta.sh compares."""
    return sum(1 for line in text.splitlines() if "error" in line)


def fixture_name(scope, path):
    """The fixture a source or baseline file belongs to, as a path stem.

    A Rust fixture's authored specification (`foo.spec.lean`) belongs to the
    fixture of `foo.rs`, so both sources answer the same name.
    """
    for suffix in (scope["baseline"],) + scope["sources"]:
        if path.endswith(suffix):
            stem = path[: -len(suffix)]
            return stem[: -len(".spec")] if stem.endswith(".spec") else stem
    return None


def scope_of(path):
    """The scope a path belongs to, or None."""
    for scope in SCOPES:
        if not path.startswith(scope["root"] + "/"):
            continue
        if scope["package"] == "move-prover-lean" and any(
                mark in path for mark in PROVER_WITHOUT_BASELINE):
            return None
        return scope
    return None


def index(blobs):
    """Group blobs by scope and fixture: {(package, fixture): {path: hash}}."""
    grouped = defaultdict(dict)
    for path, sha in sorted(blobs.items()):
        scope = scope_of(path)
        if scope is None:
            continue
        name = fixture_name(scope, path)
        if name is None:
            continue
        grouped[(scope["package"], name)][path] = sha
    return grouped


def baseline_path(package, fixture):
    for scope in SCOPES:
        if scope["package"] == package:
            return fixture + scope["baseline"]
    raise KeyError(package)


def source_fingerprint(files, baseline):
    """One hash for a fixture's sources, or None when it has none."""
    sources = {path: sha for path, sha in files.items() if path != baseline}
    if not sources:
        return None
    joined = "".join(f"{path}:{sha}\n" for path, sha in sorted(sources.items()))
    return blob_hash(joined.encode())


def state_of(files, baseline):
    """A fixture's whole recorded state: its sources and its baseline."""
    return (source_fingerprint(files, baseline), files.get(baseline))


def classify(source_reference, source_now, baseline_reference, baseline_now):
    """The class of one entry, from four hashes that may each be None."""
    if source_reference is not None and source_now is None:
        return "fixture-removed"
    if source_reference is None and source_now is not None:
        return "fixture-new"
    if baseline_reference is None and baseline_now is not None:
        return "baseline-added"
    if baseline_reference is not None and baseline_now is None:
        return "baseline-removed"
    if baseline_reference != baseline_now:
        return "baseline-changed"
    if source_reference != source_now:
        return "source-changed"
    return "same"


def source_status(source_reference, source_now):
    if source_reference is None and source_now is None:
        return "none"
    if source_reference is None:
        return "new"
    if source_now is None:
        return "removed"
    return "identical" if source_reference == source_now else "changed"


def introduced_at(state_now, states):
    """The first named revision whose state equals the current one."""
    for revision, _ in ATTRIBUTION:
        if states[revision] == state_now:
            return revision
    return "worktree"


def auto_reason(entry):
    """The only reason that can be read off mechanically: the Rust work."""
    rust_revision = ATTRIBUTION[1][0]
    if entry["introduced_at"] != rust_revision:
        return ""
    paths = [entry["baseline"]] + entry["sources"]
    if any("verex-rust" in path or path.endswith(".rs") or
           path.endswith(".spec.lean") for path in paths):
        return "rust"
    return ""


def is_fixture(scope, files, baseline):
    """Whether a group of files is an entry of the ledger at all."""
    if scope["clean_visible"]:
        return True
    return baseline in files


def build_entries():
    roots = sorted({scope["root"] for scope in SCOPES})
    now = index(worktree_of(roots))
    reference = index(tree_of(REFERENCE, roots))
    history = {revision: index(tree_of(revision, roots))
               for revision, _ in ATTRIBUTION}

    directories_with_baseline = set()
    for grouped in [now, reference] + list(history.values()):
        for (package, fixture), files in grouped.items():
            if baseline_path(package, fixture) in files:
                directories_with_baseline.add((package, fixture.rsplit("/", 1)[0]))

    entries = []
    for key in sorted(set(now) | set(reference)):
        package, fixture = key
        scope = next(s for s in SCOPES if s["package"] == package)
        baseline = baseline_path(package, fixture)
        files_now = now.get(key, {})
        files_reference = reference.get(key, {})
        if not is_fixture(scope, {**files_reference, **files_now}, baseline):
            continue
        if scope["clean_visible"] and (
                package, fixture.rsplit("/", 1)[0]) not in directories_with_baseline:
            continue
        state_now = state_of(files_now, baseline)
        state_reference = state_of(files_reference, baseline)
        klass = classify(state_reference[0], state_now[0],
                         state_reference[1], state_now[1])
        entry = {
            "package": package,
            "fixture": fixture,
            "sources": sorted(path for path in {**files_reference, **files_now}
                              if path != baseline),
            "baseline": baseline,
            "class": klass,
            "source_status": source_status(state_reference[0], state_now[0]),
            "baseline_at_reference": "present" if state_reference[1] else "absent",
            "baseline_now": "present" if state_now[1] else "absent",
            "errors_at_reference": None,
            "errors_now": None,
            "introduced_at": "same" if klass == "same" else introduced_at(
                state_now, {revision: state_of(history[revision].get(key, {}), baseline)
                            for revision, _ in ATTRIBUTION}),
            "reason": "",
        }
        entries.append(entry)

    add_error_counts(entries)
    for entry in entries:
        entry["reason"] = auto_reason(entry)
    return entries


def add_error_counts(entries):
    """Count the error lines of every baseline, at the reference and now."""
    wanted = [entry["baseline"] for entry in entries
              if entry["baseline_at_reference"] == "present"]
    texts = contents_of(REFERENCE, wanted)
    for entry in entries:
        if entry["baseline_at_reference"] == "present":
            entry["errors_at_reference"] = error_lines(texts.get(entry["baseline"], ""))
        if entry["baseline_now"] == "present":
            path = REPO / entry["baseline"]
            entry["errors_now"] = error_lines(
                path.read_text(encoding="utf-8", errors="replace"))


CLASS_ORDER = ["baseline-changed", "baseline-added", "baseline-removed",
               "fixture-new", "fixture-removed", "source-changed", "same"]


def summary(entries):
    by_class = Counter(entry["class"] for entry in entries)
    by_revision = Counter(entry["introduced_at"] for entry in entries
                          if entry["class"] != "same")
    by_package = Counter(entry["package"] for entry in entries)
    return by_class, by_revision, by_package


def render_markdown(entries):
    by_class, by_revision, by_package = summary(entries)
    out = []
    out.append(f"Reference: `{REFERENCE}`. Entries: {len(entries)}.")
    out.append("")
    out.append("| class | entries |")
    out.append("| --- | --- |")
    for klass in CLASS_ORDER:
        if by_class[klass]:
            out.append(f"| {klass} | {by_class[klass]} |")
    out.append("")
    out.append("| package | entries |")
    out.append("| --- | --- |")
    for package in sorted(by_package):
        out.append(f"| {package} | {by_package[package]} |")
    out.append("")
    out.append("| first seen at | changed entries |")
    out.append("| --- | --- |")
    for revision, label in ATTRIBUTION:
        if by_revision[revision]:
            out.append(f"| `{revision}` ({label}) | {by_revision[revision]} |")
    if by_revision["worktree"]:
        out.append(f"| worktree | {by_revision['worktree']} |")
    out.append("")

    for klass in CLASS_ORDER:
        chosen = [entry for entry in entries if entry["class"] == klass]
        if not chosen or klass == "same":
            continue
        out.append(f"## {klass} ({len(chosen)})")
        out.append("")
        for package in sorted({entry["package"] for entry in chosen}):
            rows = [entry for entry in chosen if entry["package"] == package]
            out.append(f"### {package} ({len(rows)})")
            out.append("")
            out.append("| fixture | source | baseline ref | baseline now |"
                       " errors | first seen | reason |")
            out.append("| --- | --- | --- | --- | --- | --- | --- |")
            for entry in rows:
                out.append("| {} | {} | {} | {} | {} | {} | {} |".format(
                    entry["fixture"].rsplit("/", 1)[-1],
                    entry["source_status"],
                    entry["baseline_at_reference"],
                    entry["baseline_now"],
                    errors_cell(entry),
                    entry["introduced_at"],
                    entry["reason"]))
            out.append("")
    count = by_class["same"]
    out.append(f"## same ({count})")
    out.append("")
    out.append("Not listed: these fixtures carry the reference's expectation.")
    out.append("")
    return "\n".join(out)


def errors_cell(entry):
    before = entry["errors_at_reference"]
    after = entry["errors_now"]
    if before is None and after is None:
        return "-"
    return f"{'-' if before is None else before} → {'-' if after is None else after}"


def check(entries, ledger_path):
    """Fail when an entry differs from the reference without a stated reason."""
    recorded = json.loads(Path(ledger_path).read_text())
    reasons = {(entry["package"], entry["fixture"]): entry.get("reason", "")
               for entry in recorded["entries"]}
    unexplained = []
    for entry in entries:
        if entry["class"] == "same":
            continue
        key = (entry["package"], entry["fixture"])
        if not reasons.get(key, "").strip():
            unexplained.append(entry)
    for entry in unexplained:
        print(f"{entry['class']}: {entry['baseline']}: no reason in {ledger_path}")
    if unexplained:
        print(f"{len(unexplained)} entries differ from {REFERENCE} without a reason.")
        return 1
    print(f"Every entry differing from {REFERENCE} has a reason.")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", choices=["md", "json"], default="md")
    parser.add_argument("--check", metavar="LEDGER",
                        help="fail when an entry differs without a reason")
    parser.add_argument("--output", metavar="FILE", help="write there, not to stdout")
    args = parser.parse_args()

    entries = build_entries()
    if args.check:
        return check(entries, args.check)

    if args.format == "json":
        text = json.dumps({"reference": REFERENCE,
                           "attribution": [{"revision": r, "what": w}
                                           for r, w in ATTRIBUTION],
                           "entries": entries}, indent=2) + "\n"
    else:
        text = render_markdown(entries) + "\n"
    if args.output:
        Path(args.output).write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
