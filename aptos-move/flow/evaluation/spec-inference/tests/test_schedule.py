from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.artifacts import sha256_file, tree_hash
from harness.config import RunSpec
from harness.schedule import ARMS, build_schedule


class IterativeScheduleTest(unittest.TestCase):
    def test_screened_corpus_runs_without_reviews_or_mutants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            commit = "a" * 40
            records = []
            shared = root / "framework"
            shared.mkdir()
            (shared / "Move.toml").write_text(
                '[package]\nname = "Shared"\n', encoding="utf-8"
            )
            for index in range(30):
                patch = root / "patches" / f"task-{index:02d}.patch"
                patch.parent.mkdir(exist_ok=True)
                patch.write_text("", encoding="utf-8")
                records.append(
                    {
                        "task_id": f"task-{index:02d}",
                        "selection_status": "selected",
                        "package_module_target": f"0x1::m{index}::f",
                        "shared_package_path": "framework",
                        "shared_package_sha256": tree_hash(shared),
                        "preparation_patch": f"patches/task-{index:02d}.patch",
                        "preparation_patch_sha256": sha256_file(patch),
                        "prepared_sha256": tree_hash(shared),
                        "required_contract_categories": ["normal-result"],
                    }
                )
            corpus = {
                "schema_version": 1,
                "corpus_status": "screened",
                "source_commit": commit,
                "selection_seed": "b" * 64,
                "compatibility_screen": {
                    "passed": 30,
                    "failed": 0,
                    "requires_fix_or_rerun": 0,
                },
                "records": records,
            }
            corpus_path = root / "corpus.json"
            corpus_path.write_text(json.dumps(corpus), encoding="utf-8")

            base_tools = ["move_package_verify"]
            plugin_paths = {}
            for arm in ARMS:
                plugin = root / "plugins" / arm
                plugin.mkdir(parents=True)
                tools = base_tools + ([] if arm == "agent_only" else ["move_package_wp"])
                (plugin / "move-flow-manifest.json").write_text(
                    json.dumps(
                        {
                            "inference_tactic": arm,
                            "evaluation_mode": True,
                            "flow_source_commit": commit,
                            "rendered_inference_skill_sha256": "c" * 64,
                            "mcp_tool_list_sha256": "d" * 64,
                            "mcp_tools": tools,
                        }
                    ),
                    encoding="utf-8",
                )
                plugin_paths[arm] = str(plugin)
            plugins_path = root / "plugins.json"
            plugins_path.write_text(json.dumps(plugin_paths), encoding="utf-8")

            config = json.loads(
                (Path(__file__).parent.parent / "config" / "default.json").read_text(
                    encoding="utf-8"
                )
            )
            config["source_commit"] = commit
            config_path = root / "config.json"
            config_path.write_text(json.dumps(config), encoding="utf-8")

            first = build_schedule(
                corpus_path,
                plugins_path,
                root / "round-001",
                1,
                "round-001",
                config_path,
                None,
                ("initial skill version",),
            )
            second = build_schedule(
                corpus_path,
                plugins_path,
                root / "round-002",
                1,
                "round-002",
                config_path,
                "round-001",
                ("reword guided skill",),
            )

            self.assertEqual(90, len(first["runs"]))
            self.assertNotEqual(first["randomization_seed"], second["randomization_seed"])
            self.assertEqual("round-001", second["parent_round_id"])
            run_path = next((root / "round-001" / "runs").glob("*.json"))
            run = RunSpec.load(run_path)
            self.assertEqual("round-001", run.round_id)
            self.assertEqual("0" * 64, run.mutant_manifest_sha256)
            self.assertIsNotNone(run.experiment_config_sha256)

    def test_failed_machine_screen_is_not_runnable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus = root / "corpus.json"
            corpus.write_text(
                json.dumps(
                    {
                        "source_commit": "a" * 40,
                        "selection_seed": "b" * 64,
                        "compatibility_screen": {"passed": 29, "failed": 1},
                        "records": [
                            {"selection_status": "selected"} for _ in range(30)
                        ],
                    }
                ),
                encoding="utf-8",
            )
            plugins = root / "plugins.json"
            plugins.write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "machine compatibility screen"):
                build_schedule(corpus, plugins, root / "round", 1)


if __name__ == "__main__":
    unittest.main()
