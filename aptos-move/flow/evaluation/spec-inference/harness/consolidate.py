"""Consolidate a completed corpus into one stable, inspectable artifact tree."""

from __future__ import annotations

import argparse
import copy
import os
import re
import shutil
from pathlib import Path
from typing import Any

from .artifacts import load_object, sha256_file, tree_hash, write_json
from .prepare import (
    _atomic_text,
    _root_name,
    _verify_patch_reproduction,
    _write_corpus_catalog_readme,
)


GENERATED_RECORD_FIELDS = {
    "allowed_edit_paths",
    "compatibility_screen",
    "dependency_closure",
    "package_relpath",
    "preparation_patch",
    "preparation_patch_sha256",
    "prepared_path",
    "prepared_sha256",
    "pristine_sha256",
    "removed_reference_blocks",
    "required_contract_categories",
    "resolved_address_aliases",
    "sample_path",
    "sample_readme",
    "snapshot_status",
}


def consolidate_corpus(
    source_manifest: Path,
    candidate_inventory: Path,
    screening_ledger: Path,
    output_root: Path,
) -> dict[str, Any]:
    if output_root.exists():
        raise FileExistsError(f"refusing to overwrite corpus tree: {output_root}")
    source = load_object(source_manifest)
    selected = [
        copy.deepcopy(record)
        for record in source["records"]
        if record["selection_status"] == "selected"
    ]
    if len(selected) != 30:
        raise ValueError(f"expected 30 selected records, got {len(selected)}")
    old_artifacts = (
        source_manifest.parent / source["preparation"]["artifacts_root"]
    ).resolve()
    if not (old_artifacts / "snapshots").is_dir():
        raise ValueError(f"prepared corpus is missing: {old_artifacts}")

    shutil.copytree(old_artifacts, output_root, symlinks=True, copy_function=os.link)
    metadata_dir = output_root / "metadata"
    patches_dir = output_root / "patches"
    results_dir = output_root / "screening" / "results"
    metadata_dir.mkdir()
    patches_dir.mkdir()
    results_dir.mkdir(parents=True)

    for record in selected:
        task_id = record["task_id"]
        snapshot = output_root / "snapshots" / task_id
        if tree_hash(snapshot) != record["prepared_sha256"]:
            raise ValueError(f"prepared source hash mismatch for {task_id}")
        source_patch = (
            source_manifest.parent / record["preparation_patch"]
        ).resolve()
        if sha256_file(source_patch) != record["preparation_patch_sha256"]:
            raise ValueError(f"preparation patch hash mismatch for {task_id}")
        destination_patch = patches_dir / f"{task_id}.patch"
        os.link(source_patch, destination_patch)

        screen = record["compatibility_screen"]
        source_result = (source_manifest.parent / screen["result_path"]).resolve()
        if sha256_file(source_result) != screen["result_sha256"]:
            raise ValueError(f"screening result hash mismatch for {task_id}")
        destination_result = results_dir / f"{task_id}.json"
        os.link(source_result, destination_result)

        record["prepared_path"] = f"snapshots/{task_id}"
        record["preparation_patch"] = f"patches/{task_id}.patch"
        record["sample_path"] = f"samples/{task_id}"
        record["sample_readme"] = f"samples/{task_id}/README.md"
        screen["result_path"] = f"screening/results/{task_id}.json"

        sample_patch = output_root / "samples" / task_id / "preparation.patch"
        sample_patch.unlink(missing_ok=True)
        sample_patch.symlink_to(f"../../patches/{task_id}.patch")

    manifest = {
        **{key: value for key, value in source.items() if key != "records"},
        "records": selected,
    }
    manifest.pop("freeze_status", None)
    manifest["corpus_status"] = "screened"
    manifest["candidate_inventory"] = "metadata/candidate-inventory.json"
    manifest["selection_metadata"] = "metadata/selection.json"
    manifest["screening_ledger"] = "screening/ledger.json"
    manifest["preparation"] = {
        **manifest["preparation"],
        "artifacts_root": ".",
        "package_store": "packages",
        "sample_catalog": "samples",
    }
    write_json(output_root / "manifest.json", manifest)

    inventory = load_object(candidate_inventory)
    if inventory.get("source_commit") != source.get("source_commit"):
        raise ValueError("candidate inventory and corpus source commit disagree")
    write_json(metadata_dir / "candidate-inventory.json", inventory)
    selection = {
        **{
            key: value
            for key, value in source.items()
            if key
            not in {
                "records",
                "preparation",
                "compatibility_screen",
                "corpus_status",
                "freeze_status",
            }
        },
        "records": [
            {
                key: value
                for key, value in record.items()
                if key not in GENERATED_RECORD_FIELDS
            }
            for record in source["records"]
        ],
    }
    write_json(metadata_dir / "selection.json", selection)

    ledger = load_object(screening_ledger)
    ledger["screen_manifests"] = [
        {
            "path": "../manifest.json",
            "sha256": sha256_file(output_root / "manifest.json"),
        }
    ]
    write_json(output_root / "screening" / "ledger.json", ledger)

    closures: dict[str, dict[str, Any]] = {}
    for record in selected:
        closures.setdefault(
            record["source_root"], {"packages": record["dependency_closure"]}
        )
    _write_corpus_catalog_readme(
        output_root,
        selected,
        closures,
        source["source_commit"],
    )
    validate_consolidated_corpus(output_root)
    return manifest


def validate_consolidated_corpus(root: Path) -> None:
    manifest = load_object(root / "manifest.json")
    records = manifest["records"]
    if len(records) != 30 or any(
        record["selection_status"] != "selected" for record in records
    ):
        raise ValueError("consolidated manifest is not the selected 30-task corpus")
    for record in records:
        task_id = record["task_id"]
        snapshot = root / record["prepared_path"]
        patch = root / record["preparation_patch"]
        result = root / record["compatibility_screen"]["result_path"]
        if tree_hash(snapshot) != record["prepared_sha256"]:
            raise ValueError(f"consolidated source hash mismatch for {task_id}")
        if sha256_file(patch) != record["preparation_patch_sha256"]:
            raise ValueError(f"consolidated patch hash mismatch for {task_id}")
        if sha256_file(result) != record["compatibility_screen"]["result_sha256"]:
            raise ValueError(f"consolidated screen hash mismatch for {task_id}")
        sample = root / record["sample_path"]
        if (sample / "source").resolve() != snapshot.resolve():
            raise ValueError(f"sample source link mismatch for {task_id}")
        if (sample / "preparation.patch").resolve() != patch.resolve():
            raise ValueError(f"sample patch link mismatch for {task_id}")
        if not (sample / "README.md").is_file():
            raise ValueError(f"sample README missing for {task_id}")
    for relative in (
        manifest["candidate_inventory"],
        manifest["selection_metadata"],
        manifest["screening_ledger"],
    ):
        if not (root / relative).is_file():
            raise ValueError(f"corpus metadata missing: {relative}")


def normalize_consolidated_patches(root: Path) -> None:
    """Remove construction-machine paths from patches and refresh their hashes."""
    manifest_path = root / "manifest.json"
    manifest = load_object(manifest_path)
    preparation_records = {
        record["task_id"]: record for record in manifest["preparation"]["records"]
    }
    for record in manifest["records"]:
        task_id = record["task_id"]
        patch = root / record["preparation_patch"]
        text = patch.read_text(encoding="utf-8")
        pattern = re.compile(
            rf"(?m)^diff --git a/.*/snapshots/{re.escape(task_id)}/([^ ]+) b/([^ ]+)$"
        )
        normalized, count = pattern.subn(r"diff --git a/\1 b/\2", text)
        if count == 0 and f"/snapshots/{task_id}/" in text:
            raise ValueError(
                f"could not normalize machine-specific patch header for {task_id}"
            )
        _atomic_text(patch, normalized)
        _verify_patch_reproduction(
            root / "pristine" / _root_name(record["source_root"]),
            root / record["prepared_path"],
            patch,
            task_id,
        )
        digest = sha256_file(patch)
        record["preparation_patch_sha256"] = digest
        preparation_records[task_id]["preparation_patch_sha256"] = digest
    write_json(manifest_path, manifest)
    ledger_path = root / manifest["screening_ledger"]
    ledger = load_object(ledger_path)
    ledger["screen_manifests"] = [
        {"path": "../manifest.json", "sha256": sha256_file(manifest_path)}
    ]
    write_json(ledger_path, ledger)
    validate_consolidated_corpus(root)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--candidate-inventory", type=Path)
    parser.add_argument("--screening-ledger", type=Path)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--normalize-patches", action="store_true")
    args = parser.parse_args()
    output_root = args.output_root.resolve()
    if args.normalize_patches:
        normalize_consolidated_patches(output_root)
        print(f"normalized patches at {output_root}")
        return
    if args.validate_only:
        validate_consolidated_corpus(output_root)
        print(f"validated corpus at {output_root}")
        return
    if not args.manifest or not args.candidate_inventory or not args.screening_ledger:
        parser.error(
            "--manifest, --candidate-inventory, and --screening-ledger are "
            "required when building a consolidated corpus"
        )
    result = consolidate_corpus(
        args.manifest.resolve(),
        args.candidate_inventory.resolve(),
        args.screening_ledger.resolve(),
        output_root,
    )
    print(f"consolidated {len(result['records'])} samples at {output_root}")


if __name__ == "__main__":
    main()
