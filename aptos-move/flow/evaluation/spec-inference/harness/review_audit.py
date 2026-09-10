"""Audit optional reference and mutant scoring coverage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from .artifacts import load_object, tree_hash, write_json


def audit_reviews(provenance_path: Path, review_dir: Path) -> dict[str, Any]:
    provenance = load_object(provenance_path)
    selected = [
        record
        for record in provenance["records"]
        if record["selection_status"] == "selected"
    ]
    records = []
    for record in selected:
        task_id = record["task_id"]
        recipe_path = review_dir / "recipes" / f"{task_id}.json"
        recipe = load_object(recipe_path)
        reference = (recipe_path.parent / recipe["reference_path"]).resolve()
        reference_hash = tree_hash(reference)
        reference_identity_ok = reference_hash == recipe["reference_sha256"]
        authored = (
            record["reference_origin"] == "upstream"
            or reference_hash != record["prepared_sha256"]
        )
        compatibility = load_object(
            (recipe_path.parent / recipe["compatibility_validation_result"]).resolve()
        )
        compatibility_passed = bool(
            compatibility.get("passed") is True
            and compatibility.get("target") == record["package_module_target"]
            and compatibility.get("package_sha256") == reference_hash
        )
        reference_reviewers = _approved_reviewers(recipe.get("reference_reviews", []))
        leakage_reviewers = _approved_reviewers(recipe.get("leakage_reviews", []))
        mutant_manifest = load_object(
            (recipe_path.parent / recipe["mutant_manifest"]).resolve()
        )
        approved_mutants = [
            mutant
            for mutant in mutant_manifest.get("mutants", [])
            if _mutant_approved(mutant)
        ]
        validation = load_object(
            (recipe_path.parent / recipe["mutant_validation_result"]).resolve()
        )
        mutant_validation_passed = bool(
            len(approved_mutants) >= 3
            and validation.get("package_sha256") == reference_hash
            and validation.get("essential_mutants") == len(approved_mutants)
            and validation.get("killed") == len(approved_mutants)
        )
        ready = bool(
            reference_identity_ok
            and authored
            and compatibility_passed
            and len(reference_reviewers) >= 2
            and len(leakage_reviewers) >= 2
            and mutant_validation_passed
        )
        records.append(
            {
                "task_id": task_id,
                "reference_origin": record["reference_origin"],
                "reference_identity_ok": reference_identity_ok,
                "reference_authored": authored,
                "compatibility_passed": compatibility_passed,
                "reference_approval_count": len(reference_reviewers),
                "leakage_approval_count": len(leakage_reviewers),
                "approved_essential_mutants": len(approved_mutants),
                "mutant_validation_passed": mutant_validation_passed,
                "scoring_ready": ready,
            }
        )
    return {
        "schema_version": 1,
        "source_commit": provenance["source_commit"],
        "tasks": len(records),
        "references_authored": sum(item["reference_authored"] for item in records),
        "compatibility_passed": sum(item["compatibility_passed"] for item in records),
        "reference_reviews_complete": sum(
            item["reference_approval_count"] >= 2 for item in records
        ),
        "leakage_reviews_complete": sum(
            item["leakage_approval_count"] >= 2 for item in records
        ),
        "mutant_sets_complete": sum(
            item["approved_essential_mutants"] >= 3 for item in records
        ),
        "mutant_validations_complete": sum(
            item["mutant_validation_passed"] for item in records
        ),
        "scoring_ready_tasks": sum(item["scoring_ready"] for item in records),
        "scoring_ready": len(records) == 30 and all(
            item["scoring_ready"] for item in records
        ),
        "records": records,
    }


def _approved_reviewers(reviews: list[dict[str, Any]]) -> set[str]:
    return {
        str(review["reviewer"])
        for review in reviews
        if review.get("approved") is True and review.get("reviewer")
    }


def _mutant_approved(case: dict[str, Any]) -> bool:
    reviewers = _approved_reviewers(case.get("reviews", []))
    return case.get("essential") is True and len(reviewers) >= 2



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--review-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_reviews(args.provenance.resolve(), args.review_dir.resolve())
    write_json(args.output.resolve(), result)
    print(
        json.dumps(
            {
                key: result[key]
                for key in (
                    "tasks",
                    "references_authored",
                    "compatibility_passed",
                    "reference_reviews_complete",
                    "leakage_reviews_complete",
                    "mutant_sets_complete",
                    "mutant_validations_complete",
                    "scoring_ready_tasks",
                    "scoring_ready",
                )
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
