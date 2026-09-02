"""Restore pinned handwritten dependency contracts after bulk WP inference."""

from __future__ import annotations

import argparse
import json
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Any

from .artifacts import load_object, write_json
from .move_source import function_spec_blocks, mask_comments_and_strings
from .opaque_dependencies import _insert_opaque_pragma
from .prepare import _atomic_text


_CONSERVATIVE_DEPENDENCY_OVERRIDES = {
    (
        "sources/AptosFramework/datastructures/big_ordered_map.move",
        "add_at",
    ),
    (
        "sources/AptosFramework/datastructures/big_ordered_map.move",
        "remove_at_with_iter_hint",
    ),
}
_CONSERVATIVE_MARKER = "Corpus dependency abstraction for recursive structural helper."


def restore_reference_contracts(
    package: Path,
    repo_root: Path,
    source_commit: str,
    audit_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    audit = load_object(audit_path)
    catalog = load_object(package / "corpus-modules.json")
    original_by_corpus_path = {
        corpus_path: original_path
        for original_path, corpus_path in catalog["source_path_map"].items()
    }
    paths_by_module = {
        record["module"]: {
            record["implementation"],
            *record.get("specifications", []),
        }
        for record in catalog["modules"]
    }
    functions_by_path: dict[str, set[str]] = defaultdict(set)
    for contract in audit["contracts"]:
        if not contract["condition_count"]:
            continue
        function = contract["function"].rsplit("::", 1)[-1]
        own_paths = paths_by_module.get(contract["module"], set())
        for relative in contract["specification_paths"]:
            if relative in own_paths and relative in original_by_corpus_path:
                functions_by_path[relative].add(function)

    restored = []
    non_textual = []
    missing = []
    for relative, functions in sorted(functions_by_path.items()):
        current_path = package / relative
        original_path = original_by_corpus_path[relative]
        original = _git_show(repo_root, source_commit, original_path)
        current = current_path.read_text(encoding="utf-8")
        replacements = []
        for function in sorted(functions):
            original_spans = _function_spec_spans(original, function)
            current_spans = _function_spec_spans(current, function)
            if not original_spans and current_spans:
                # Some conditions are attached by `apply` statements rather
                # than a textual `spec <function> { ... }` block.  There is
                # no handwritten block to restore in that case.  Preserve (or
                # reconstruct) the small direct opaque stub supplied by the
                # dependency pass, discarding any WP clauses appended to it.
                start, end = current_spans[0]
                indent = current[start:]
                indent = indent[: len(indent) - len(indent.lstrip(" \t"))]
                stub = (
                    f"{indent}spec {function} {{\n"
                    f"{indent}    pragma opaque = true;\n"
                    f"{indent}}}"
                )
                replacements.append((start, end, stub, function))
                for duplicate_start, duplicate_end in current_spans[1:]:
                    replacements.append(
                        (duplicate_start, duplicate_end, "", function)
                    )
                non_textual.append({"function": function, "path": relative})
                continue
            if not original_spans and not current_spans:
                non_textual.append({"function": function, "path": relative})
                continue
            if len(current_spans) < len(original_spans):
                missing.append(
                    {
                        "function": function,
                        "path": relative,
                        "original_blocks": len(original_spans),
                        "current_blocks": len(current_spans),
                    }
                )
                continue
            for original_span, current_span in zip(original_spans, current_spans):
                original_block = original[slice(*original_span)]
                opaque_block, _ = _insert_opaque_pragma(original_block, function)
                start, end = current_span
                replacements.append((start, end, opaque_block, function))
            for start, end in current_spans[len(original_spans) :]:
                replacements.append((start, end, "", function))
        # Replacements were collected in function-name order, which is not
        # source order.  Applying them in that order changes offsets for later
        # edits and can splice blocks together.  Always edit from the end of
        # the file toward the beginning.
        for start, end, replacement, function in sorted(
            replacements, key=lambda item: item[0], reverse=True
        ):
            current = current[:start] + replacement + current[end:]
            restored.append({"function": function, "path": relative})
        if replacements:
            _atomic_text(current_path, current)

    conservative_overrides = _apply_conservative_dependency_overrides(package)

    result = {
        "schema_version": 1,
        "package": str(package),
        "source_commit": source_commit,
        "audit": str(audit_path),
        "restored_function_count": len(restored),
        "restored": restored,
        "non_textual_function_count": len(non_textual),
        "non_textual": non_textual,
        "unresolved_function_count": len(missing),
        "unresolved": missing,
        "conservative_dependency_override_count": len(conservative_overrides),
        "conservative_dependency_overrides": conservative_overrides,
    }
    write_json(output_path, result)
    return result


def _apply_conservative_dependency_overrides(package: Path) -> list[dict[str, str]]:
    """Give recursive helpers a sound, deliberately unconstrained boundary.

    These helpers sit below the public intrinsic-map abstraction. WP does not
    terminate on them even at function granularity, while external proofs must
    not gain a false postcondition or a false no-abort guarantee.  A partial
    abort specification plus `ensures true` permits every behavior.
    """
    changed = []
    for relative, function in sorted(_CONSERVATIVE_DEPENDENCY_OVERRIDES):
        path = package / relative
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        spans = _function_spec_spans(text, function)
        if len(spans) != 1:
            raise ValueError(
                f"expected one spec block for conservative override "
                f"{relative}:{function}, got {len(spans)}"
            )
        start, end = spans[0]
        block = text[start:end]
        if _CONSERVATIVE_MARKER in block:
            continue
        indent = text[start:]
        indent = indent[: len(indent) - len(indent.lstrip(" \t"))]
        insertion = (
            f"\n{indent}    // {_CONSERVATIVE_MARKER}\n"
            f"{indent}    pragma aborts_if_is_partial = true;\n"
            f"{indent}    ensures true;\n{indent}"
        )
        text = text[: end - 1] + insertion + text[end - 1 :]
        _atomic_text(path, text)
        changed.append({"path": relative, "function": function})
    return changed


def _function_spec_spans(source: str, function: str) -> list[tuple[int, int]]:
    blocks = function_spec_blocks(mask_comments_and_strings(source), function)
    return [(block.start, block.end) for block in blocks]


def _git_show(repo_root: Path, commit: str, path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{commit}:{path}"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if result.returncode != 0:
        raise ValueError(f"cannot read {path} at {commit}: {result.stderr.strip()}")
    return result.stdout



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = restore_reference_contracts(
        args.package.resolve(),
        args.repo_root.resolve(),
        args.source_commit,
        args.audit.resolve(),
        args.output.resolve(),
    )
    print(
        json.dumps(
            {
                "restored_function_count": result["restored_function_count"],
                "unresolved_function_count": result["unresolved_function_count"],
            },
            sort_keys=True,
        )
    )
    if result["unresolved"]:
        raise SystemExit("one or more pinned contract blocks could not be restored")


if __name__ == "__main__":
    main()
