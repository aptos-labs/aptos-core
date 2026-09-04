#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Diffs a MonoMove test run against failing.toml and skipped.toml.

The parity CI job runs the normal suites with MONO_MOVE_ENV=1, which adds
ENABLE_MONO_MOVE to the default feature set. Every test absent from both files is
expected to pass. This script turns that expectation into a gate.

  --skip-filter          print a nextest filterset excluding skipped.toml
  --check RESULTS...     diff a finished run against both files, exit 1 on drift
  --update RESULTS...    rewrite failing.toml from a finished run

RESULTS is nextest libtest-json output, produced by

  NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json
"""

import argparse
import json
import re
import sys
from pathlib import Path

FAILING = Path(__file__).with_name("failing.toml")
SKIPPED = Path(__file__).with_name("skipped.toml")
INVOCATION = "python3 third_party/move/mono-move/aptos-tests-parity/check_parity.py"

# Nextest reports a test killed by `slow-timeout.terminate-after` as a failure
# carrying this reason.
TIMEOUT_REASON = "time limit exceeded"

FAILING_HEADER = """\
# Tests that run under MonoMove and fail. Anything absent from this file and from
# skipped.toml must pass. See README.md.
#
# Regenerate. Download results.json from the mono-move-tests-parity workflow, or
# produce it locally:
#
#   FILTER=$({invocation} --skip-filter)
#   MONO_MOVE_ENV=1 RUST_MIN_STACK=1073741824 NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 \\
#     cargo nextest run --profile ci --workspace --no-fail-fast \\
#     --message-format libtest-json -E "$FILTER" > results.json
#
#   {invocation} --update results.json
""".format(invocation=INVOCATION)

SKIPPED_HEADER = """\
# Tests excluded from MonoMove parity checks because MonoMove does not support
# these features. Currently also includes tests that may hang, because MonoMove
# does not enable metering. See README.md for more details.
"""

FIELDS = ("name", "note")
ASSIGNMENT = re.compile(r'^(\w+) = (".*")$')


class Entry:
    def __init__(self, name, status, note):
        self.name = name
        self.status = status
        self.note = note


def parse(text, filename, status):
    """Reads the `[[test]]` array, and nothing else.

    Not a TOML parser: `tomllib` needs 3.11 and the runners are not pinned.
    Anything outside the schema is an error, never a silent skip.
    """
    records = []
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue

        where = f"{filename}:{number}"
        if line == "[[test]]":
            records.append({})
            continue

        match = ASSIGNMENT.match(line)
        if not match:
            fail(f"{where}: expected `[[test]]` or `key = \"value\"`, got {line!r}")
        key, quoted = match.groups()
        if key not in FIELDS:
            fail(f"{where}: unknown key {key!r}, expected one of {', '.join(FIELDS)}")
        if not records:
            fail(f"{where}: {key} appears before any `[[test]]`")
        if key in records[-1]:
            fail(f"{where}: {key} is set twice in the same entry")
        try:
            records[-1][key] = json.loads(quoted)
        except json.JSONDecodeError:
            fail(f"{where}: {quoted} is not a valid quoted string")

    entries = {}
    for record in records:
        if "name" not in record:
            fail(f"{filename}: an entry is missing `name`")
        name = record["name"]
        if name in entries:
            fail(f"{filename}: {name} is listed twice")
        entries[name] = Entry(name, status, record.get("note"))
    return entries


def load():
    entries = {}
    for path, status in ((FAILING, "failing"), (SKIPPED, "skipped")):
        if not path.exists():
            continue
        for name, entry in parse(path.read_text(), path.name, status).items():
            if name in entries:
                fail(f"{name} is in both {FAILING.name} and {SKIPPED.name}")
            entries[name] = entry
    return entries


def write_failing(entries):
    out = [FAILING_HEADER]
    for entry in sorted(entries.values(), key=lambda e: e.name):
        if entry.status != "failing":
            continue
        out.append("\n[[test]]\n")
        out.append(f"name = {quote(entry.name)}\n")
        if entry.note:
            out.append(f"note = {quote(entry.note)}\n")
    FAILING.write_text("".join(out))


def quote(value):
    # A TOML basic string and a JSON string escape identically, so this round
    # trips through `json.loads` in the reader and stays valid TOML on disk.
    return json.dumps(value)


def parse_results(paths):
    """Reads nextest libtest-json into `{name: outcome}`.

    Nextest emits one terminal event per test even under retries, suffixed with
    `#<attempt>`. It already reflects the final outcome, so the suffix is dropped.
    """
    outcomes = {}
    for path in paths:
        for line in Path(path).read_text().splitlines():
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") != "test":
                continue

            outcome = event.get("event")
            if outcome not in ("ok", "failed", "ignored"):
                continue
            if outcome == "failed" and event.get("reason") == TIMEOUT_REASON:
                outcome = "timeout"
            outcomes[event["name"].rsplit("#", 1)[0]] = outcome

    if not outcomes:
        fail(
            "no test events found; the run needs NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 "
            "and --message-format libtest-json"
        )
    return outcomes


def split_name(name):
    """Splits `package::binary$test::path` into a nextest binary id and test name.

    Nextest labels a lib test with the bare package name. Cargo derives that
    binary name by replacing hyphens with underscores, which is how a lib test is
    recognised here. Every other binary keeps the `package::binary` form.
    """
    binary_id, _, test = name.partition("$")
    package, _, binary = binary_id.partition("::")
    if binary == package.replace("-", "_"):
        binary_id = package
    return binary_id, test


def skip_filter(entries):
    skipped = sorted(e.name for e in entries.values() if e.status == "skipped")
    if not skipped:
        return "all()"
    terms = []
    for name in skipped:
        binary_id, test = split_name(name)
        terms.append(f"(binary_id(={binary_id}) & test(={test}))")
    return f"not ({' + '.join(terms)})"


def check(entries, outcomes):
    broken = ("failed", "timeout")

    new_failures = sorted(
        name
        for name, outcome in outcomes.items()
        if outcome in broken and name not in entries
    )
    now_passing = sorted(
        name
        for name, entry in entries.items()
        if entry.status == "failing" and outcomes.get(name) == "ok"
    )
    # A skipped test is filtered out of the run, so seeing one means the filter
    # and skipped.toml disagree.
    ran_anyway = sorted(
        name
        for name, entry in entries.items()
        if entry.status == "skipped" and name in outcomes
    )

    if new_failures:
        print(f"{len(new_failures)} test(s) fail under MonoMove and are not listed:")
        for name in new_failures:
            suffix = " (timed out)" if outcomes[name] == "timeout" else ""
            print(f"  {name}{suffix}")
        print()

    if now_passing:
        print(f"{len(now_passing)} test(s) in {FAILING.name} now pass, drop them:")
        for name in now_passing:
            print(f"  {name}")
        print()

    if ran_anyway:
        print(f"{len(ran_anyway)} test(s) in {SKIPPED.name} ran anyway:")
        for name in ran_anyway:
            print(f"  {name}")
        print(f"Either the run dropped `--skip-filter`, or these belong in {FAILING.name}.")
        print()

    if new_failures or now_passing or ran_anyway:
        # `--update` fixes the first two. It leaves skipped.toml alone, so
        # `ran_anyway` needs a person either way.
        return False

    failing = sum(1 for e in entries.values() if e.status == "failing")
    skipped = sum(1 for e in entries.values() if e.status == "skipped")
    print(f"parity holds: {len(outcomes)} run, {failing} failing, {skipped} skipped")
    return True


def update(entries, outcomes):
    removed = [
        name
        for name, entry in sorted(entries.items())
        if entry.status == "failing" and outcomes.get(name) == "ok"
    ]
    for name in removed:
        del entries[name]

    added = []
    for name, outcome in sorted(outcomes.items()):
        if name in entries or outcome not in ("failed", "timeout"):
            continue
        entries[name] = Entry(name, "failing", None)
        added.append((name, outcome))

    write_failing(entries)

    print(f"{FAILING.name}: removed {len(removed)}, added {len(added)}")
    for name in removed:
        print(f"  - {name}")
    for name, outcome in added:
        suffix = f" (timed out; consider moving to {SKIPPED.name})" if outcome == "timeout" else ""
        print(f"  + {name}{suffix}")


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(2)


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--skip-filter",
        action="store_true",
        help="print a nextest filterset excluding skipped.toml",
    )
    mode.add_argument(
        "--check",
        nargs="+",
        metavar="RESULTS",
        help="diff a finished run against both files",
    )
    mode.add_argument(
        "--update",
        nargs="+",
        metavar="RESULTS",
        help="rewrite failing.toml from a finished run",
    )
    args = parser.parse_args()

    entries = load()

    if args.skip_filter:
        print(skip_filter(entries))
        return 0

    if args.check:
        if check(entries, parse_results(args.check)):
            return 0
        print(f"Update {FAILING.name} from this run with:")
        print(f"  {INVOCATION} --update {' '.join(args.check)}")
        return 1

    update(entries, parse_results(args.update))
    return 0


if __name__ == "__main__":
    sys.exit(main())
