"""Mark existing dependency contracts opaque in the editable shared package."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from .artifacts import write_json
from .move_source import closing_brace, function_spec_blocks, mask_comments_and_strings
from .prepare import _atomic_text


def make_dependency_contracts_opaque(
    package: Path, audit_path: Path, output_path: Path
) -> dict[str, Any]:
    audit = json.loads(audit_path.read_text(encoding="utf-8"))
    catalog = json.loads(
        (package / "corpus-modules.json").read_text(encoding="utf-8")
    )
    paths_by_module = {
        record["module"]: [
            *record.get("specifications", []),
            record["implementation"],
        ]
        for record in catalog["modules"]
    }
    changed = []
    deferred = []
    for contract in audit["contracts"]:
        if contract["status"] != "nonopaque_contract":
            continue
        function = contract["function"].rsplit("::", 1)[-1]
        locations = []
        own_paths = paths_by_module.get(contract["module"], [])
        condition_paths = set(contract["specification_paths"])
        candidate_paths = [path for path in own_paths if path in condition_paths]
        candidate_paths.extend(path for path in own_paths if path not in condition_paths)
        for relative in candidate_paths:
            path = package / relative
            if not path.is_file():
                continue
            source = path.read_text(encoding="utf-8")
            updated, count = _insert_opaque_pragma(source, function)
            if count:
                _atomic_text(path, updated)
                locations.append(
                    {"path": relative, "blocks": count, "kind": "made_opaque"}
                )
                break
            existing = len(
                function_spec_blocks(mask_comments_and_strings(source), function)
            )
            if existing:
                locations.append(
                    {
                        "path": relative,
                        "blocks": existing,
                        "kind": "already_opaque",
                    }
                )
                break
        if not locations:
            # Conditions introduced through a schema `apply` have a source
            # location but no direct textual function block in which to place
            # the pragma.  Add a direct opaque-only block to the same module;
            # the compiler combines it with the schema-applied conditions.
            module = contract["module"].rsplit("::", 1)[-1]
            for relative in candidate_paths:
                path = package / relative
                if not path.is_file():
                    continue
                source = path.read_text(encoding="utf-8")
                updated, count = _append_opaque_stub(source, module, function)
                if count:
                    _atomic_text(path, updated)
                    locations.append(
                        {"path": relative, "blocks": count, "kind": "opaque_stub"}
                    )
                    break
        if not locations:
            deferred.append(
                {
                    "function": contract["function"],
                    "reason": "no_textual_function_or_module_spec_block",
                }
            )
        else:
            changed.append({"function": contract["function"], "locations": locations})
    result = {
        "schema_version": 1,
        "package": str(package),
        "audit": str(audit_path),
        "changed_function_count": len(changed),
        "changed": changed,
        "deferred_function_count": len(deferred),
        "deferred": deferred,
    }
    write_json(output_path, result)
    return result


def _insert_opaque_pragma(text: str, function: str) -> tuple[str, int]:
    masked = mask_comments_and_strings(text)
    insertions = []
    for block in function_spec_blocks(masked, function):
        if re.search(r"\bpragma\s+opaque\b", masked[block.brace : block.end]):
            continue
        indent = block.indent + "    "
        prefix = masked[block.brace + 1 : block.end]
        declarations = re.match(r"(?:\s*use\b[^;]*;)*", prefix)
        insertion_offset = block.brace + 1 + (0 if declarations is None else declarations.end())
        insertions.append(
            (insertion_offset, f"\n{indent}pragma opaque = true;")
        )
    for offset, insertion in reversed(insertions):
        text = text[:offset] + insertion + text[offset:]
    return text, len(insertions)


def _append_opaque_stub(text: str, module: str, function: str) -> tuple[str, int]:
    """Append a direct opaque block to one textual module-spec block."""
    masked = mask_comments_and_strings(text)
    pattern = re.compile(
        rf"(?m)^[ \t]*(?:spec|module)[ \t]+[A-Za-z0-9_]+::{re.escape(module)}"
        rf"\b[ \t]*\{{"
    )
    matches = list(pattern.finditer(masked))
    if len(matches) != 1:
        return text, 0
    opening = masked.find("{", matches[0].start(), matches[0].end())
    closing = closing_brace(masked, opening)
    if closing is None:
        raise ValueError(f"unterminated module spec block for {module}")
    insertion = (
        f"\n\n    spec {function} {{\n"
        "        pragma opaque = true;\n"
        "    }"
    )
    offset = closing - 1
    return text[:offset] + insertion + text[offset:], 1


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = make_dependency_contracts_opaque(
        args.package.resolve(), args.audit.resolve(), args.output.resolve()
    )
    print(
        json.dumps(
            {
                "changed_function_count": result["changed_function_count"],
                "deferred_function_count": result["deferred_function_count"],
            }
        )
    )


if __name__ == "__main__":
    main()
