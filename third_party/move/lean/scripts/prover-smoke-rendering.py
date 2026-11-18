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

A fixture named in `PROOF_CONSUMERS` is the one exception to the third: its
snapshot holds the proof file's items as items of the module, so that the
proofs an authored fixture owes are elaborated by the libraries alone. The
refusal stands for every other fixture with a proof file.

`scripts/prover-smoke-renderings.sh` drives it.
"""
import os
import re
import sys
import textwrap

HEAD = re.compile(r"^[^\s:]+:\d+:\d+: (error|warning|info): (.*)$")
COMMAND = re.compile(r"^verex (namespace|module) ", re.M)
# Any directive by which a rendering names a proof file, whichever file.
PROOF_FILE = re.compile(r"^\s*pragma\s+proof_file\b")

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

PROOF_CONSUMER_HOW = """The module below is the rendering of
`VerexE2ETests/SourceVerify/{name}.move` that the source-verification driver
writes to `.verex/{name}.move.lean`, with the items of
`VerexE2ETests/SourceVerify/{name}.proof.lean` as items of the module and the
`pragma proof_file` that names that file dropped: the driver defers this
fixture's verification to the proof file, which a lone elaboration does not
read, so the snapshot carries the proofs themselves and proves what the
fixture owes. Its `verex sources at` command is dropped and the diagnostics
are those of `{name}.exp`, which holds the proof file's messages as well.
Nothing here is edited by hand: `scripts/prover-smoke-renderings.sh --write`
writes this file again, and without `--write` tells whether it is what the
frontend renders and what the proof file says."""

# The fixtures whose snapshot holds their proof file's items. A Move source
# whose verification the driver defers to an adjacent `.proof.lean` is
# refused as a snapshot; one named here is rendered as a single module with
# the proofs in it, so that an authored proof is gated by the libraries
# alone. A cell without an authored block must verify automatically, so no
# `DEFERRED` diagnostic needs rewriting. A named fixture gives every authored
# cell a block; cells left without one are those expected to verify on their
# own (such as `split_reference_end`'s `drain_automatic`).
PROOF_CONSUMERS = ("map_key_default", "map_value_default", "map_size_update",
                   "split_reference_end", "pure_spec_map_fact", "map_write_block")

# The import a proof file of this mode carries.
PROOF_IMPORT = "import VerexMove.Proofs"

# How a cell with no block of its own reports its failure: the driver names
# the proof file, a module elaborated on its own says "in the module".
DEFERRED = "provide a proof in `"

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
    with open(path) as file:
        baseline = file.read()
    for line in baseline.split("\n"):
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
    proofs = os.path.exists(os.path.join(fixtures, name + ".proof.lean"))
    if name in PROOF_CONSUMERS:
        if any(DEFERRED in message for message in kept):
            found.append("a cell of it fails the automatic verification, whose message names "
                         "the proof file the driver defers to and a lone elaboration words "
                         "differently: give that cell a block in the proof file")
    elif proofs:
        found.append("it has a proof file, to which the driver defers the verification")
    commands = [match.start() for match in COMMAND.finditer(before)] + [len(before)]
    for start, end in zip(commands, commands[1:]):
        if "pragma verify = false" not in before[start:end]:
            head = before[start:end].split("\n", 1)[0]
            found.append("`%s` before the last namespace is verified" % head.strip())
    return found


def proof_directive(name, fixtures):
    """The directive by which a rendering of this fixture names its proof
    file: the path the driver writes, the fixture directory's last two parts
    and the file."""
    parts = os.path.normpath(fixtures).split(os.sep)[-2:]
    return 'pragma proof_file = "%s"' % "/".join(parts + [name + ".proof.lean"])


def proof_items(name, fixtures):
    """The proof file's items after `verex proofs where`, indented as items
    of the module, as `scripts/isolate-target.py` indents them for one
    target.

    The mode carries the items and nothing else, so it supports one shape of
    proof file — comments, `import VerexMove.Proofs`, one `verex proofs for`
    of this fixture's source, one `verex proofs where` — and refuses every
    other command before the heading instead of dropping it, which would
    leave the snapshot saying something the fixture does not."""
    path = os.path.join(fixtures, name + ".proof.lean")
    if not os.path.exists(path):
        raise SystemExit(
            "%s cannot be a snapshot: it is a proof consumer with no proof file" % name)
    refuse = lambda why: SystemExit("%s cannot be a snapshot: %s" % (name, why))
    with open(path) as file:
        lines = file.read().split("\n")
    headings = [index for index, line in enumerate(lines)
                if line.strip() == "verex proofs where"]
    if len(headings) != 1:
        raise refuse("its proof file has %d `verex proofs where` headings, not one"
                     % len(headings))
    imports, sources = 0, []
    for line in lines[:headings[0]]:
        text = line.strip()
        if not text or text.startswith("--"):
            continue
        if text == PROOF_IMPORT:
            imports += 1
            continue
        source = re.fullmatch(r'verex proofs for "([^"]*)"', text)
        if source:
            sources.append(source.group(1))
            continue
        raise refuse("its proof file's `%s` is no part of this mode, which carries the "
                     "items alone" % text)
    if imports != 1:
        raise refuse("its proof file has %d `%s`, not one" % (imports, PROOF_IMPORT))
    if sources != [name + ".move"]:
        raise refuse("its proof file is of %s, not of `%s.move`"
                     % (sources or "no source", name))
    items = ["  " + line if line.strip() else "" for line in lines[headings[0] + 1:]]
    body = "\n".join(items).strip("\n")
    if not any(item.strip().startswith(("verify ", "verify!")) for item in items):
        raise refuse("its proof file proves nothing: no `verify` item after the heading")
    return body


def rendering(name, path, proofs=None, directive=None):
    """The rendered commands: the ones before the target, and the target.
    `proofs` are the proof file's items, which become the target's last
    items in place of `directive`, the one `pragma proof_file` by which the
    rendering names their file. The target must name that file and no other:
    every `pragma proof_file` of it is collected, the list must be
    `[directive]`, and that one line is what the items replace."""
    with open(path) as file:
        text = file.read()
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
    before, last = body[:target[-1]].strip(), body[target[-1]:].strip() + "\n"
    if proofs is not None:
        lines = last.split("\n")
        named = [line.strip() for line in lines if PROOF_FILE.match(line)]
        if named != [directive]:
            raise SystemExit("%s cannot be a snapshot: its rendering does not name its proof "
                             "file once by `%s`; it names %s"
                             % (name, directive, ", ".join("`%s`" % one for one in named)
                                or "no proof file"))
        kept = [line for line in lines if not PROOF_FILE.match(line)]
        last = "\n".join(kept).strip() + "\n\n" + proofs + "\n"
    return before, last


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
    consumer = name in PROOF_CONSUMERS
    if consumer:
        how = PROOF_CONSUMER_HOW
    elif os.path.exists(os.path.join(fixtures, name + ".move")):
        how = MOVE_HOW
    else:
        how = RUST_HOW
    how = how.format(name=name)
    if dropped:
        how += GIVEN_NOTE.format(name=name, dropped=dropped, plural="" if dropped == 1 else "s")
    before, target = rendering(name, path, proof_items(name, fixtures) if consumer else None,
                               proof_directive(name, fixtures))
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
    with open(output, "w") as file:
        file.write(snapshot(name, path, fixtures))
