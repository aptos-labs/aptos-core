#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""Isolate one verification target of a VerexSL module file.

Every other function of the target's module gets
`pragma verify = false`, the modules after it are dropped, and the given
options are set after `import VerexSL`, so one target can be timed or
debugged on its own. A rendering carries no authored proofs: `--proofs`
names the module's proof file, whose theorems, `open` items, and proof of
the target become items of the module:

  scripts/isolate-target.py build/verex-verify.lean ordered_map test_verify_remove_or_none \\
      --proofs sources/datastructures/ordered_map.proof.lean -o one.lean \\
      --option verex.certifyDebug=true

Run the result from the package that builds `VerexSL`, for example
`(cd verex-ir && lake env lean ../one.lean)`.
"""

import argparse
import re
import sys


def proof_items(lines, target):
    """The items of a proof file after `verex proofs where`, indented as module
    items, without the proofs of functions other than `target`."""
    identifier = r"(?:«[^»]+»|\w+)"
    try:
        start = next(index for index, line in enumerate(lines)
                     if line.strip() == "verex proofs where") + 1
    except StopIteration:
        raise ValueError("the proof file has no `verex proofs where`")
    items = []
    skipping = False
    for line in lines[start:]:
        proof = re.match(rf"verify!? ({identifier})(?=\s|$)", line)
        if proof:
            skipping = proof.group(1) != target
        elif line.strip() and not line.startswith(" "):
            skipping = False
        if not skipping:
            items.append("  " + line if line.strip() else "")
    return items


def isolate(lines, module, target, options, proofs=()):
    module = module.split("::")[-1]
    out = []
    in_module = False
    seen_module = False
    current = None
    functions = []
    specifications = set()
    skipping_proof = False
    # Where the module's items end: before the first top-level command after it.
    end = None
    identifier = r"(?:«[^»]+»|\w+)"
    for line in lines:
        if line.startswith("verex module "):
            if seen_module:
                break
            in_module = line.split()[2].split("::")[-1] == module
            seen_module = in_module
            current = None
        elif in_module and line.strip() and not line.startswith(" "):
            in_module = False
            end = len(out)
        if line.startswith("set_option verex.stageLog"):
            continue
        if in_module:
            proof = re.match(rf"  verify ({identifier})(?=\s|$)", line)
            if proof:
                skipping_proof = proof.group(1) != target
            elif line.strip() and not line.startswith("    "):
                skipping_proof = False
            if skipping_proof:
                continue
            function = re.match(
                rf"  (?:(?:public(?:\([^)]*\))?|entry|inline|native)\s+)*fun ({identifier})", line)
            if function:
                functions.append(function.group(1))
                current = None
        header = re.match(rf"  spec ({identifier}) where\s*$", line)
        if header:
            current = header.group(1)
            if in_module:
                specifications.add(current)
        if (in_module and current and current != target
                and re.fullmatch(r"pragma verify(?:\s*=.*)?", line.strip())):
            line = "    pragma verify = false"
        out.append(line)
        if in_module and header and current != target:
            out.append("    pragma verify = false")
    if not seen_module:
        raise ValueError(f"module {module!r} was not found")
    if target not in functions:
        raise ValueError(f"function {target!r} was not found in module {module!r}")
    items = list(proofs)
    # Functions with assertions but no explicit spec are verification targets too.
    for function in functions:
        if function != target and function not in specifications:
            items.extend([f"  spec {function} where", "    pragma verify = false", ""])
    if end is None:
        end = len(out)
    if items and items[-1].strip():
        items.append("")
    out[end:end] = items
    text = "\n".join(out) + "\n"
    settings = "".join(f"set_option {name} {value}\n" for name, value in options)
    return text.replace("import VerexSL\n", "import VerexSL\n" + settings, 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("source")
    parser.add_argument("module", help="the module's last path segment, e.g. ordered_map")
    parser.add_argument("target", help="the function whose specification stays verified")
    parser.add_argument("-o", "--output", required=True)
    parser.add_argument("--option", action="append", default=[],
                        help="name=value, set after `import VerexSL`")
    parser.add_argument("--proofs", help="the module's proof file")
    arguments = parser.parse_args()
    options = []
    for option in arguments.option:
        name, _, value = option.partition("=")
        if not value:
            sys.exit(f"--option {option}: expected name=value")
        options.append((name, value))
    with open(arguments.source) as source:
        lines = source.read().split("\n")
    proofs = []
    if arguments.proofs:
        with open(arguments.proofs) as proof_file:
            proofs = proof_items(proof_file.read().split("\n"), arguments.target)
    with open(arguments.output, "w") as output:
        output.write(isolate(lines, arguments.module, arguments.target, options, proofs))


if __name__ == "__main__":
    main()
