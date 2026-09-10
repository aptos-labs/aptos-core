"""Promote a deterministic reserve into an already prepared shared corpus."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any

from .identifiers import resolve_within
from .artifacts import load_object, sha256_file, write_json
from .prepare import _allowed_edit_paths, _required_categories
from .refresh import refresh_recipes
from .replacement import replace_task


def promote_replacement(
    corpus_root: Path,
    selection_path: Path,
    package_inventory_path: Path,
    task_id: str,
    reason: str,
) -> dict[str, Any]:
    """Replace one prepared task and regenerate every hash-bearing recipe.

    The selection manifest remains the authoritative reserve list. The prepared
    manifest intentionally contains only the 30 runnable records, so this
    bridge copies the promoted reserve into that smaller manifest and refreshes
    its dependency closure against the edited shared package.
    """
    manifest_path = corpus_root / "manifest.json"
    manifest = load_object(manifest_path)
    selection = replace_task(selection_path, task_id, reason)
    inventory = load_object(package_inventory_path)
    catalog = load_object(corpus_root / "framework" / "corpus-modules.json")

    replacement = next(
        record for record in selection["records"] if record.get("replaces") == task_id
    )
    current = next(
        candidate
        for candidate in inventory["candidates"]
        if candidate["package_module_target"]
        == replacement["package_module_target"]
    )
    old = next(record for record in manifest["records"] if record["task_id"] == task_id)

    record = dict(replacement)
    record["transitive_function_dependencies"] = current[
        "transitive_function_dependencies"
    ]
    record["called_function_dependencies"] = current[
        "called_function_dependencies"
    ]
    record["spec_function_dependencies"] = current[
        "spec_function_dependencies"
    ]
    record["transitive_called_function_dependencies"] = current[
        "transitive_called_function_dependencies"
    ]
    record["transitive_module_dependencies"] = current[
        "transitive_module_dependencies"
    ]
    source_map = catalog["source_path_map"]
    record["corpus_source_path"] = source_map[record["source_path"]]
    record["corpus_reference_paths"] = [
        source_map[path] for path in record["reference_paths"] if path in source_map
    ]
    record["allowed_edit_paths"] = _allowed_edit_paths(record)
    record["required_contract_categories"] = _required_categories(record)
    record.update(
        {
            "shared_package_path": old["shared_package_path"],
            "shared_package_sha256": "",
            "prepared_sha256": "",
            "package_relpath": ".",
            "preparation_patch": f"patches/{record['task_id']}.patch",
            "preparation_patch_sha256": "",
            "removed_reference_blocks": [],
            "sample_path": f"samples/{record['task_id']}",
            "sample_readme": f"samples/{record['task_id']}/README.md",
            "snapshot_status": "shared_package_recipe",
        }
    )

    index = manifest["records"].index(old)
    manifest["records"][index] = record
    preparation = manifest["preparation"]
    preparation_record = next(
        item for item in preparation["records"] if item["task_id"] == task_id
    )
    preparation["records"][preparation["records"].index(preparation_record)] = {
        "task_id": record["task_id"],
        "shared_package_sha256": "",
        "prepared_sha256": "",
        "preparation_patch_sha256": "",
        "removed_reference_blocks": [],
        "required_contract_categories": record["required_contract_categories"],
    }

    for key in (
        "replacement_history",
        "selected_counts",
        "selected_feature_counts",
        "unmet_feature_minima",
    ):
        if key in selection:
            manifest[key] = selection[key]
    manifest["corpus_status"] = "prepared_requires_screening"
    manifest.pop("compatibility_screen", None)
    manifest.pop("screening_ledger_sha256", None)

    write_json(selection_path, selection)
    preparation["provenance_input_sha256"] = sha256_file(selection_path)
    write_json(manifest_path, manifest)
    refreshed = refresh_recipes(corpus_root, package_inventory_path)

    old_patch = resolve_within(corpus_root, old["preparation_patch"], "preparation_patch")
    old_sample = resolve_within(corpus_root, old["sample_path"], "sample_path")
    old_patch.unlink(missing_ok=True)
    if old_sample.exists():
        shutil.rmtree(old_sample)
    return refreshed



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument("--package-inventory", type=Path, required=True)
    parser.add_argument("--task-id", required=True)
    parser.add_argument("--reason", required=True)
    args = parser.parse_args()
    result = promote_replacement(
        args.corpus.resolve(),
        args.selection.resolve(),
        args.package_inventory.resolve(),
        args.task_id,
        args.reason,
    )
    replacement = result["replacement_history"][-1]
    print(json.dumps(replacement, sort_keys=True))


if __name__ == "__main__":
    main()
