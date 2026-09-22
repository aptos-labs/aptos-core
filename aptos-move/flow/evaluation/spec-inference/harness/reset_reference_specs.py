"""Reconstruct pinned handwritten companion specs after dependency generation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from .artifacts import load_object, sha256_file, write_json
from .opaque_dependencies import make_dependency_contracts_opaque
from .prepare import _atomic_text
from .restore_reference_contracts import (
    _git_show,
    restore_reference_contracts,
)
from .shared_package import normalize_standalone_specs


def reset_reference_specs(
    package: Path,
    repo_root: Path,
    source_commit: str,
    audit_path: Path,
    opaque_output_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    """Restore mapped upstream spec files, then make dependency contracts opaque.

    Corpus-generated companion files are intentionally absent from
    `source_path_map` and therefore survive this operation.  Executable
    function bodies are never touched, preserving deterministic corpus
    preparation rewrites such as higher-order loop normalization.
    """
    catalog = load_object(package / "corpus-modules.json")
    restored = []
    for original, relative in sorted(catalog["source_path_map"].items()):
        if not original.endswith(".spec.move"):
            continue
        destination = package / relative
        source = _git_show(repo_root, source_commit, original)
        _atomic_text(destination, source)
        restored.append(
            {
                "source_path": original,
                "corpus_path": relative,
                "sha256": sha256_file(destination),
            }
        )

    opaque = make_dependency_contracts_opaque(
        package, audit_path, opaque_output_path
    )
    restoration_output_path = output_path.with_name(
        "restored-reference-contracts.after-reset.json"
    )
    reference_restoration = restore_reference_contracts(
        package,
        repo_root,
        source_commit,
        audit_path,
        restoration_output_path,
    )
    standalone_spec_qualifications = normalize_standalone_specs(package)
    catalog["standalone_spec_qualifications"] = standalone_spec_qualifications
    write_json(package / "corpus-modules.json", catalog)
    result = {
        "schema_version": 1,
        "package": str(package),
        "source_commit": source_commit,
        "audit": str(audit_path),
        "restored_spec_file_count": len(restored),
        "restored_spec_files": restored,
        "standalone_spec_qualifications": standalone_spec_qualifications,
        "opaque_report": str(opaque_output_path),
        "opaque_changed_function_count": opaque["changed_function_count"],
        "opaque_deferred_function_count": opaque["deferred_function_count"],
        "reference_restoration_report": str(restoration_output_path),
        "reference_restoration_unresolved_function_count": reference_restoration[
            "unresolved_function_count"
        ],
    }
    write_json(output_path, result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--opaque-output", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = reset_reference_specs(
        args.package.resolve(),
        args.repo_root.resolve(),
        args.source_commit,
        args.audit.resolve(),
        args.opaque_output.resolve(),
        args.output.resolve(),
    )
    print(
        json.dumps(
            {
                "restored_spec_file_count": result["restored_spec_file_count"],
                "opaque_changed_function_count": result[
                    "opaque_changed_function_count"
                ],
                "opaque_deferred_function_count": result[
                    "opaque_deferred_function_count"
                ],
                "reference_restoration_unresolved_function_count": result[
                    "reference_restoration_unresolved_function_count"
                ],
            },
            sort_keys=True,
        )
    )
    if result["opaque_deferred_function_count"]:
        raise SystemExit("one or more dependency contracts could not be made opaque")
    if result["reference_restoration_unresolved_function_count"]:
        raise SystemExit("one or more pinned dependency contracts could not be restored")


if __name__ == "__main__":
    main()
