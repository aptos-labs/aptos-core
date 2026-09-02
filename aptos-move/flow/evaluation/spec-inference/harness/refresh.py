"""Refresh all sample recipes after an intentional shared-package edit."""

from __future__ import annotations

import argparse
import json
import shutil
import tempfile
from pathlib import Path

from .artifacts import copy_snapshot, load_object, sha256_file, tree_hash, write_json
from .move_source import mask_comments_and_strings
from .prepare import (
    _atomic_text,
    _remove_target_references,
    _verify_patch_reproduction,
    _write_corpus_catalog_readme,
    _write_git_patch,
    _write_sample_catalog,
)
from .shared_package import _SPEC_MODULE, _module_id


def refresh_recipes(corpus_root: Path, package_inventory_path: Path | None = None) -> dict:
    manifest_path = corpus_root / "manifest.json"
    manifest = load_object(manifest_path)
    if package_inventory_path is not None:
        inventory = load_object(package_inventory_path)
        _refresh_dependency_closures(manifest, inventory)
        manifest["dependency_refresh"] = {
            "package_inventory": package_inventory_path.relative_to(corpus_root).as_posix(),
            "package_inventory_sha256": sha256_file(package_inventory_path),
        }
    shared = corpus_root / "framework"
    shared_metadata = _refresh_shared_catalog(shared)
    shared_hash = tree_hash(shared)
    preparation_by_task = {
        record["task_id"]: record
        for record in manifest["preparation"]["records"]
    }
    for record in manifest["records"]:
        task_id = record["task_id"]
        patch = corpus_root / record["preparation_patch"]
        with tempfile.TemporaryDirectory(
            prefix=f"move-inference-refresh-{task_id}-"
        ) as temporary:
            prepared = Path(temporary) / "package"
            copy_snapshot(shared, prepared)
            removed = _remove_target_references(prepared, record)
            descriptor = {
                "schema_version": 3,
                "task_id": task_id,
                "source_commit": manifest["source_commit"],
                "source_path": record["corpus_source_path"],
                "package_module_target": record["package_module_target"],
                "granularity": record["granularity"],
                "target_functions": record["target_functions"],
                "transitive_function_dependencies": record[
                    "transitive_function_dependencies"
                ],
                "called_function_dependencies": record[
                    "called_function_dependencies"
                ],
                "spec_function_dependencies": record[
                    "spec_function_dependencies"
                ],
                "transitive_called_function_dependencies": record[
                    "transitive_called_function_dependencies"
                ],
                "transitive_module_dependencies": record[
                    "transitive_module_dependencies"
                ],
            }
            _atomic_text(
                prepared / ".move-inference-task.json",
                json.dumps(descriptor, indent=2, sort_keys=True) + "\n",
            )
            _write_git_patch(shared, prepared, patch)
            _verify_patch_reproduction(shared, prepared, patch, task_id)
            prepared_hash = tree_hash(prepared)
        patch_hash = sha256_file(patch)
        record.update(
            {
                "shared_package_sha256": shared_hash,
                "prepared_sha256": prepared_hash,
                "preparation_patch_sha256": patch_hash,
                "removed_reference_blocks": removed,
            }
        )
        record.pop("compatibility_screen", None)
        preparation_by_task[task_id].update(
            {
                "shared_package_sha256": shared_hash,
                "prepared_sha256": prepared_hash,
                "preparation_patch_sha256": patch_hash,
                "removed_reference_blocks": removed,
            }
        )
        sample = corpus_root / record["sample_path"]
        if sample.exists():
            shutil.rmtree(sample)
        _write_sample_catalog(
            sample,
            shared,
            patch,
            record,
            manifest["source_commit"],
        )
    manifest["corpus_status"] = "prepared_requires_screening"
    manifest["preparation"]["shared_package_sha256"] = shared_hash
    manifest["preparation"]["shared_module_count"] = shared_metadata[
        "module_count"
    ]
    manifest["preparation"]["shared_move_file_count"] = shared_metadata[
        "move_file_count"
    ]
    manifest.pop("compatibility_screen", None)
    manifest.pop("screening_ledger_sha256", None)
    write_json(manifest_path, manifest)
    _write_corpus_catalog_readme(
        corpus_root,
        manifest["records"],
        shared_metadata,
        manifest["source_commit"],
    )
    return manifest


def _refresh_dependency_closures(manifest: dict, inventory: dict) -> None:
    """Refresh selected targets against the edited shared package's call graph."""
    functions = {
        candidate["package_module_target"]: candidate
        for candidate in inventory["candidates"]
        if candidate["granularity"] == "function"
    }
    for record in manifest["records"]:
        module = record["module"]
        targets = (
            [record["package_module_target"]]
            if record["granularity"] == "function"
            else [f"{module}::{function}" for function in record["target_functions"]]
        )
        missing = [target for target in targets if target not in functions]
        if missing:
            raise ValueError(
                f"edited package inventory is missing targets for {record['task_id']}: "
                + ", ".join(missing)
            )
        dependencies = set()
        called_boundaries = set()
        spec_dependencies = set()
        called_dependencies = set()
        modules = set()
        for target in targets:
            candidate = functions[target]
            dependencies.update(candidate["transitive_function_dependencies"])
            called_boundaries.update(candidate["called_function_dependencies"])
            spec_dependencies.update(candidate["spec_function_dependencies"])
            called_dependencies.update(
                candidate["transitive_called_function_dependencies"]
            )
            modules.update(candidate["transitive_module_dependencies"])
        record["transitive_function_dependencies"] = sorted(dependencies)
        record["called_function_dependencies"] = sorted(called_boundaries)
        record["spec_function_dependencies"] = sorted(spec_dependencies)
        record["transitive_called_function_dependencies"] = sorted(called_dependencies)
        record["transitive_module_dependencies"] = sorted(modules)


def _refresh_shared_catalog(shared: Path) -> dict:
    """Reconcile the module map with editable/generated companion specs."""
    catalog_path = shared / "corpus-modules.json"
    catalog = load_object(catalog_path)
    aliases = catalog["resolved_address_aliases"]
    modules = {record["module"]: record for record in catalog["modules"]}
    for record in modules.values():
        record["specifications"] = []

    move_files = sorted((shared / "sources").rglob("*.move"))
    for path in move_files:
        relative = path.relative_to(shared).as_posix()
        source = mask_comments_and_strings(path.read_text(encoding="utf-8"))
        for match in _SPEC_MODULE.finditer(source):
            module = _module_id(match.group(1), match.group(2), aliases)
            if module not in modules:
                raise ValueError(
                    f"companion specification `{relative}` targets unknown module `{module}`"
                )
            specifications = modules[module]["specifications"]
            if relative not in specifications:
                specifications.append(relative)

    for record in modules.values():
        record["specifications"].sort()
    original_paths = set(catalog["source_path_map"].values())
    catalog["schema_version"] = 2
    catalog["generated_specifications"] = sorted(
        path.relative_to(shared).as_posix()
        for path in move_files
        if path.name.endswith(".spec.move")
        and path.relative_to(shared).as_posix() not in original_paths
    )
    write_json(catalog_path, catalog)
    return {
        "module_count": len(modules),
        "move_file_count": len(move_files),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument(
        "--package-inventory",
        type=Path,
        required=True,
        help="inventory-package output for the edited shared package",
    )
    args = parser.parse_args()
    manifest = refresh_recipes(
        args.corpus.resolve(), args.package_inventory.resolve()
    )
    print(
        json.dumps(
            {
                "shared_package_sha256": manifest["preparation"][
                    "shared_package_sha256"
                ],
                "tasks": len(manifest["records"]),
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
