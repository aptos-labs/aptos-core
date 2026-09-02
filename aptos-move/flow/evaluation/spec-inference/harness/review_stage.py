"""Stage hidden reference and mutant material for the **V1 corpus**.

This is V1 tooling and is not used by V3, which does not need it: a V3
reference is a committed specification patch assembled by
`corpus-v3/build_references.py`, and its mutants are authored directly as
anchored manifests. Both are then validated by `harness.validate_mutants`.
The V1 flow is different because that corpus restores pinned-commit
framework specifications rather than authoring them, which is what this
stages.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
from pathlib import Path
from typing import Any

from .artifacts import copy_snapshot, load_object, sha256_file, tree_hash, write_json


def stage_reviews(
    provenance_path: Path,
    output_dir: Path,
) -> dict[str, Any]:
    provenance = load_object(provenance_path)
    selected = [
        record
        for record in provenance["records"]
        if record["selection_status"] == "selected"
    ]
    if len(selected) != 30:
        raise ValueError(f"expected 30 selected tasks, got {len(selected)}")
    if provenance.get("compatibility_screen", {}).get("passed") != 30:
        raise ValueError("review staging requires a 30/30 screened corpus")
    if output_dir.exists():
        raise FileExistsError(f"refusing to overwrite review staging directory: {output_dir}")

    corpus_root = (
        provenance_path.parent / provenance["preparation"]["artifacts_root"]
    ).resolve()
    references_dir = output_dir / "references"
    recipes_dir = output_dir / "recipes"
    mutants_dir = output_dir / "mutants"
    validations_dir = output_dir / "validations"
    for directory in (references_dir, recipes_dir, mutants_dir, validations_dir):
        directory.mkdir(parents=True, exist_ok=True)

    staged = []
    for record in selected:
        task_id = record["task_id"]
        prepared = (provenance_path.parent / record["prepared_path"]).resolve()
        pristine = corpus_root / "pristine" / Path(record["source_root"]).name
        if tree_hash(prepared) != record["prepared_sha256"]:
            raise ValueError(f"prepared snapshot hash mismatch for {task_id}")
        if tree_hash(pristine) != record["pristine_sha256"]:
            raise ValueError(f"pristine snapshot hash mismatch for {task_id}")

        reference = references_dir / task_id
        copy_snapshot(prepared, reference)
        restored_paths = _restore_upstream_references(record, pristine, reference)
        reference_hash = tree_hash(reference)
        reference_status = (
            "upstream_restored_pending_review"
            if record["reference_origin"] == "upstream"
            else "study_authorship_required"
        )

        mutant_path = mutants_dir / f"{task_id}.json"
        write_json(
            mutant_path,
            {
                "schema_version": 1,
                "task_id": task_id,
                "package_module_target": record["package_module_target"],
                "reference_sha256": reference_hash,
                "status": "authoring_and_independent_review_required",
                "mutants": [],
            },
        )
        mutant_validation = validations_dir / f"{task_id}.mutants.json"
        write_json(
            mutant_validation,
            {
                "schema_version": 1,
                "task_id": task_id,
                "package_sha256": reference_hash,
                "status": "pending",
                "essential_mutants": 0,
                "killed": 0,
            },
        )
        compatibility = validations_dir / f"{task_id}.compatibility.json"
        write_json(
            compatibility,
            {
                "schema_version": 4,
                "task_id": task_id,
                "target": record["package_module_target"],
                "package_sha256": reference_hash,
                "status": "pending",
                "passed": False,
            },
        )

        recipe_path = recipes_dir / f"{task_id}.json"
        recipe = {
            "schema_version": 1,
            "draft_status": reference_status,
            "task_id": task_id,
            "source_commit": provenance["source_commit"],
            "package_module_target": record["package_module_target"],
            "pristine_snapshot": _relative(pristine, recipe_path.parent),
            "pristine_sha256": record["pristine_sha256"],
            "prepared_snapshot": _relative(prepared, recipe_path.parent),
            "prepared_sha256": record["prepared_sha256"],
            "preparation_patch": _relative(
                (provenance_path.parent / record["preparation_patch"]).resolve(),
                recipe_path.parent,
            ),
            "preparation_patch_sha256": record["preparation_patch_sha256"],
            "package_relpath": record.get("package_relpath", "."),
            "allowed_edit_paths": record["allowed_edit_paths"],
            "required_contract_categories": record["required_contract_categories"],
            "reference_path": _relative(reference, recipe_path.parent),
            "reference_sha256": reference_hash,
            "reference_origin": record["reference_origin"],
            "restored_reference_paths": restored_paths,
            "reference_reviews": [],
            "leakage_reviews": [],
            "mutant_manifest": _relative(mutant_path, recipe_path.parent),
            "mutant_validation_result": _relative(
                mutant_validation, recipe_path.parent
            ),
            "compatibility_validation_result": _relative(
                compatibility, recipe_path.parent
            ),
        }
        write_json(recipe_path, recipe)
        staged.append(
            {
                "task_id": task_id,
                "reference_origin": record["reference_origin"],
                "reference_status": reference_status,
                "reference_sha256": reference_hash,
                "restored_reference_paths": restored_paths,
                "recipe": _relative(recipe_path, output_dir),
            }
        )

    summary = {
        "schema_version": 1,
        "source_commit": provenance["source_commit"],
        "input_manifest": str(provenance_path),
        "input_manifest_sha256": sha256_file(provenance_path),
        "tasks": len(staged),
        "upstream_references_staged": sum(
            item["reference_origin"] == "upstream" for item in staged
        ),
        "study_references_requiring_authorship": sum(
            item["reference_origin"] == "study-authored" for item in staged
        ),
        "reference_reviews_required": 2 * len(staged),
        "leakage_reviews_required": 2 * len(staged),
        "minimum_essential_mutants_required": 3 * len(staged),
        "scoring_ready": False,
        "records": staged,
    }
    write_json(output_dir / "review-status.json", summary)
    return summary


def _restore_upstream_references(
    record: dict[str, Any], pristine: Path, reference: Path
) -> list[str]:
    if record["reference_origin"] != "upstream":
        return []
    paths = sorted({block["path"] for block in record["removed_reference_blocks"]})
    if not paths:
        raise ValueError(f"upstream reference has no removed blocks: {record['task_id']}")
    for relative in paths:
        source = pristine / relative
        destination = reference / relative
        if not source.is_file() or not destination.is_file():
            raise ValueError(
                f"reference source is missing for {record['task_id']}: {relative}"
            )
        shutil.copyfile(source, destination)
    return paths


def _relative(path: Path, parent: Path) -> str:
    return os.path.relpath(path, parent)



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    result = stage_reviews(args.provenance.resolve(), args.output_dir.resolve())
    print(
        json.dumps(
            {
                "tasks": result["tasks"],
                "upstream_references_staged": result[
                    "upstream_references_staged"
                ],
                "study_references_requiring_authorship": result[
                    "study_references_requiring_authorship"
                ],
                "scoring_ready": result["scoring_ready"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
