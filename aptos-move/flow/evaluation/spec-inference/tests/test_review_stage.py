from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.artifacts import sha256_file, tree_hash
from harness.review_audit import audit_reviews
from harness.review_stage import stage_reviews


class ReviewStageTest(unittest.TestCase):
    def test_restores_upstream_references_and_marks_pending_work(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus = root / "corpus"
            corpus.mkdir()
            artifacts = root / "artifacts"
            patch = corpus / "prepare.patch"
            patch.write_text("", encoding="utf-8")

            records = []
            for index in range(30):
                upstream = index < 20
                source_name = "aptos-framework" if upstream else "aptos-experimental"
                source_root = f"aptos-move/framework/{source_name}"
                pristine = artifacts / "pristine" / source_name
                prepared = artifacts / "snapshots" / f"source-{index}"
                pristine.mkdir(parents=True, exist_ok=True)
                prepared.mkdir(parents=True)
                for package in (pristine, prepared):
                    (package / "Move.toml").write_text(
                        '[package]\nname = "Fixture"\n', encoding="utf-8"
                    )
                    (package / "sources").mkdir(exist_ok=True)
                    (package / "sources" / "m.move").write_text(
                        "module 0x1::m {}\n", encoding="utf-8"
                    )
                (pristine / "sources" / "m.spec.move").write_text(
                    "spec 0x1::m { spec f() { ensures true; } }\n",
                    encoding="utf-8",
                )
                (prepared / "sources" / "m.spec.move").write_text(
                    "                                                \n",
                    encoding="utf-8",
                )
                records.append(
                    {
                        "task_id": f"task-{index:02}",
                        "selection_status": "selected",
                        "source_root": source_root,
                        "package_module_target": "0x1::m::f",
                        "prepared_path": f"../artifacts/snapshots/source-{index}",
                        "prepared_sha256": tree_hash(prepared),
                        "pristine_sha256": tree_hash(pristine),
                        "preparation_patch": "prepare.patch",
                        "preparation_patch_sha256": sha256_file(patch),
                        "package_relpath": ".",
                        "allowed_edit_paths": ["sources/m.move"],
                        "required_contract_categories": ["normal-result"],
                        "reference_origin": "upstream" if upstream else "study-authored",
                        "removed_reference_blocks": (
                            [{"path": "sources/m.spec.move", "function": "f", "blocks": 1}]
                            if upstream
                            else []
                        ),
                    }
                )

            provenance = {
                "schema_version": 1,
                "source_commit": "a" * 40,
                "corpus_status": "screened",
                "compatibility_screen": {"passed": 30},
                "preparation": {"artifacts_root": "../artifacts"},
                "records": records,
            }
            provenance_path = corpus / "provenance.json"
            provenance_path.write_text(json.dumps(provenance), encoding="utf-8")

            result = stage_reviews(provenance_path, root / "reviews")

            self.assertEqual(30, result["tasks"])
            self.assertEqual(20, result["upstream_references_staged"])
            self.assertEqual(10, result["study_references_requiring_authorship"])
            self.assertFalse(result["scoring_ready"])
            restored = root / "reviews" / "references" / "task-00" / "sources" / "m.spec.move"
            self.assertIn("ensures true", restored.read_text(encoding="utf-8"))
            pending = root / "reviews" / "references" / "task-20" / "sources" / "m.spec.move"
            self.assertNotIn("ensures true", pending.read_text(encoding="utf-8"))
            recipe = json.loads(
                (root / "reviews" / "recipes" / "task-00.json").read_text(encoding="utf-8")
            )
            self.assertEqual([], recipe["reference_reviews"])
            self.assertEqual([], recipe["leakage_reviews"])

            audit = audit_reviews(provenance_path, root / "reviews")
            self.assertEqual(30, audit["tasks"])
            self.assertEqual(20, audit["references_authored"])
            self.assertEqual(0, audit["compatibility_passed"])
            self.assertEqual(0, audit["reference_reviews_complete"])
            self.assertEqual(0, audit["leakage_reviews_complete"])
            self.assertEqual(0, audit["mutant_sets_complete"])
            self.assertEqual(0, audit["scoring_ready_tasks"])
            self.assertFalse(audit["scoring_ready"])


if __name__ == "__main__":
    unittest.main()
