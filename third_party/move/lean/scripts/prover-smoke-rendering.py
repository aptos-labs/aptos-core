#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""Write the snapshot of a source fixture's rendering for prover-smoke.

Usage: prover-smoke-rendering.py <name> <rendering> <fixtures> <output>

Reads the VerexSL rendering a frontend wrote for the source fixture `<name>`
(`<rendering>`: the file the e2e driver leaves under `.verex/` for a Move
source, the file `scripts/prover-smoke-render-rust.lean` writes for a Rust source) and the
fixture's baseline `<fixtures>/<name>.exp`, and writes the snapshot `<output>`:
the rendering, unedited, with the baseline's diagnostics under
`#guard_rendering`.

A snapshot is elaborated on its own, without the driver around it, so a
fixture is refused where the driver adds what a lone elaboration cannot:

- a failing `no panic` or `no abort` diagnostic in the baseline: the driver
  quotes the original source, or words a macro's panic, through the table of
  the rendered source, which a snapshot does not have;
- a refused function: its report is the driver's, made after elaboration;
- a proof file beside the source: the driver defers the verification to it;
- a namespace before the last that is verified: only the last command is
  under the marker, and the ones before it must report nothing.

`scripts/prover-smoke-renderings.sh` drives it.
"""
import os
import re
import sys
import textwrap

HEAD = re.compile(r"^[^\s:]+:\d+:\d+: (error|warning|info): (.*)$")
COMMAND = re.compile(r"^verex (namespace|module) ", re.M)

MOVE_HOW = """The module below is the rendering of `VerexE2ETests/SourceVerify/{name}.move`
that the source-verification driver writes to `.verex/{name}.move.lean`, its
`verex sources at` command dropped; the diagnostics are those of `{name}.exp`.
Neither is edited here: `scripts/prover-smoke-renderings.sh --write` writes this file
again, and without `--write` tells whether it is what the frontend renders."""

RUST_HOW = """The namespaces below are the rendering of
`VerexE2ETests/SourceVerify/{name}.rs`, the items of its `{name}.spec.lean` and
the contracts its calls reach spliced in, which the verification of the source
renders and elaborates; its `verex sources at` command is dropped and the
diagnostics are those of `{name}.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and without
`--write` tells whether it is what the frontend renders. The crate's own
namespace, which the diagnostics are of, is the last command; the namespaces
before it are what it uses, and report nothing."""

GIVEN_INFO = "uses concrete verification: Given interpretation does not yet support"

GIVEN_NOTE = """

`{name}.exp` records {dropped} further message{plural} of the Given route
(`uses concrete verification`). The option that takes that route,
`verex.givenRoute`, is unset and nothing sets it, so no configuration produces
them and they are no part of the diagnostics below."""


def messages(fixtures, name):
    """The baseline as `#guard_msgs` writes the same messages: no file, no
    position; the messages of the Given route apart."""
    path = os.path.join(fixtures, name + ".exp")
    if not os.path.exists(path):
        return [], 0
    found = []
    for line in open(path).read().split("\n"):
        head = HEAD.match(line)
        if head:
            found.append("%s: %s" % (head.group(1), head.group(2)))
        elif found:
            found[-1] += "\n" + line
        elif line.strip():
            raise SystemExit("%s: line before any message: %r" % (name, line))
    found = [message.rstrip() for message in found]
    kept = [message for message in found if GIVEN_INFO not in message]
    return kept, len(found) - len(kept)


NO_PANIC = re.compile(r"`no (panic|abort)[^`]*` is not established")


def refusals(name, fixtures, kept, text, before):
    """Why the fixture cannot be a snapshot; empty when it can."""
    found = []
    if any(NO_PANIC.search(message) for message in kept):
        found.append("its baseline holds a failing `no panic` or `no abort` diagnostic")
    if "@[refused" in text or any("is refused, so it is not verified" in m for m in kept):
        found.append("it has a refused function")
    if os.path.exists(os.path.join(fixtures, name + ".proof.lean")):
        found.append("it has a proof file, to which the driver defers the verification")
    commands = [match.start() for match in COMMAND.finditer(before)] + [len(before)]
    for start, end in zip(commands, commands[1:]):
        if "pragma verify = false" not in before[start:end]:
            head = before[start:end].split("\n", 1)[0]
            found.append("`%s` before the last namespace is verified" % head.strip())
    return found


def rendering(name, path):
    """The rendered commands: the ones before the target, and the target."""
    text = open(path).read()
    text = text[:text.index("\nverex sources at ")]
    lines = text.split("\n")
    first = max(i for i, line in enumerate(lines) if line.startswith("import ")) + 1
    imports = [line for line in lines[:first] if line.startswith("import ")]
    if imports != ["import VerexSL"]:
        raise SystemExit("%s: the rendering imports %s" % (name, imports))
    body = "\n".join(lines[first:]).strip() + "\n"
    target = [match.start() for match in COMMAND.finditer(body)]
    if not target:
        raise SystemExit("%s: the rendering holds no namespace command" % name)
    return body[:target[-1]].strip(), body[target[-1]:].strip() + "\n"


def wrapped(text):
    """The text rewrapped at 78 columns."""
    return "\n\n".join(
        textwrap.fill(" ".join(block.split()), width=78, break_long_words=False,
                      break_on_hyphens=False)
        for block in text.split("\n\n"))


def snapshot(name, path, fixtures):
    kept, dropped = messages(fixtures, name)
    text = "\n---\n".join(kept).strip()
    doc = "/--\n%s\n-/\n" % text if text else ""
    how = MOVE_HOW if os.path.exists(os.path.join(fixtures, name + ".move")) else RUST_HOW
    how = how.format(name=name)
    if dropped:
        how += GIVEN_NOTE.format(name=name, dropped=dropped, plural="" if dropped == 1 else "s")
    before, target = rendering(name, path)
    refused = refusals(name, fixtures, kept, before + target, before)
    if refused:
        raise SystemExit("%s cannot be a snapshot: %s" % (name, "; ".join(refused)))
    return """-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `{name}` as a frontend renders it

{how}
-/

{before}{doc}#guard_rendering in
{target}""".format(name=name, how=wrapped(how), before=before + "\n\n" if before else "",
                   doc=doc, target=target)


if __name__ == "__main__":
    name, path, fixtures, output = sys.argv[1:]
    open(output, "w").write(snapshot(name, path, fixtures))
