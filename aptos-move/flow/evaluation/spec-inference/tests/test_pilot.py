from __future__ import annotations

import hashlib
import json
import random
from collections import Counter
import tempfile
import unittest
from dataclasses import asdict
from pathlib import Path

from harness.artifacts import sha256_file
from harness.config import ExperimentConfig, RunSpec
from harness.mutants import NO_MUTANTS
from harness.pilot import (
    _balanced_arm_orders,
    _require_committed_corpus,
    _resolve_mutant_manifests,
    build_pilot,
    load_round_shape,
    plugins_from_file,
)
from harness.schedule import ARMS

# Toy tasks for scheduling tests. The scheduler only reads a sample's id,
# target, and required contract categories, so these need no Move sources
# beyond a package stub; the real task sources are the corpus packages.
TOY_TASKS: tuple[dict[str, object], ...] = (
    {"name": "calculator", "target": "0x42::calculator", "categories": ["abort", "state-transition", "frame"]},
    {"name": "double", "target": "0x42::double::double_n_times", "categories": ["normal-result", "abort", "loop-invariant"]},
    {"name": "find", "target": "0x42::collection::find", "categories": ["normal-result", "abort", "loop-invariant"]},
    {"name": "fold", "target": "0x42::runner::fold", "categories": ["normal-result", "abort", "loop-invariant"]},
    {"name": "ledger", "target": "0x42::ledger::credit_all", "categories": ["abort", "state-transition", "frame", "loop-invariant"]},
    {"name": "pow", "target": "0x42::pow::pow", "categories": ["normal-result", "abort", "loop-invariant"]},
    {"name": "split", "target": "0x42::storage::split_balance", "categories": ["normal-result", "abort", "state-transition", "frame"]},
    {"name": "sum", "target": "0x42::loops::sum_to_n", "categories": ["normal-result", "abort", "loop-invariant"]},
    {"name": "transfer", "target": "0x42::loops::transfer", "categories": ["abort", "state-transition", "frame"]},
    {"name": "vault", "target": "0x42::vault::withdraw", "categories": ["normal-result", "abort", "state-transition", "frame"]},
)


class PilotScheduleTest(unittest.TestCase):
    def test_a_flat_plugin_map_is_read_as_a_single_level(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            mapping = root / "plugin-map" / "plugins.json"
            mapping.parent.mkdir()
            mapping.write_text(
                json.dumps(
                    {
                        "agent_only": "plugins/agent-only",
                        "hybrid_guided": "/opt/plugins/hybrid-guided",
                    }
                ),
                encoding="utf-8",
            )

            plugins = plugins_from_file(mapping)

            self.assertEqual(
                root / "plugin-map/plugins/agent-only",
                plugins["acceptance"]["agent_only"],
            )
            self.assertEqual(
                Path("/opt/plugins/hybrid-guided"),
                plugins["acceptance"]["hybrid_guided"],
            )

    @staticmethod
    def _corpus(root: Path) -> Path:
        """A minimal corpus: one package and one record per toy task."""
        corpus = root / "toy-corpus"
        package = corpus / "package"
        (package / "sources").mkdir(parents=True)
        (package / "Move.toml").write_text(
            '[package]\nname = "ToyCorpus"\nversion = "1.0.0"\n', encoding="utf-8"
        )
        (package / "Prover.toml").write_text(
            "[backend]\nvc_timeout = 40\n", encoding="utf-8"
        )
        for task in TOY_TASKS:
            (package / "sources" / f"{task['name']}.move").write_text(
                f"module 0x42::{task['name']} {{}}\n", encoding="utf-8"
            )
        manifest = corpus / "manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "records": [
                        {
                            "task_id": task["name"],
                            "target": task["target"],
                            "required_contract_categories": task["categories"],
                        }
                        for task in TOY_TASKS
                    ],
                }
            ),
            encoding="utf-8",
        )
        return manifest

    @classmethod
    def _inputs(cls, root: Path) -> tuple[Path, dict[str, Path], Path, str]:
        corpus_manifest = cls._corpus(root)
        plugins = {}
        commit = "a" * 40
        for arm in ARMS:
            plugin = root / "plugins" / arm
            plugin.mkdir(parents=True)
            manifest = {
                "inference_tactic": arm,
                "evaluation_mode": True,
                "flow_source_commit": commit,
                "feedback_level": "acceptance",
            }
            (plugin / "move-flow-manifest.json").write_text(
                json.dumps(manifest), encoding="utf-8"
            )
            plugins[arm] = plugin

        config = ExperimentConfig(
            schema_version=1,
            source_commit=commit,
            claude_agent_sdk_version="0.2.139",
            claude_code_version="2.1.241",
            model="glm-5.3[1m]",
            provider_base_url="https://api.z.ai/api/anthropic",
            effort="max",
            feedback_level="acceptance",
            max_controller_turns=6,
            max_model_turns_per_controller_turn=60,
            max_wall_seconds=2700,
            max_output_tokens=100000,
            operational_timeout_seconds=10,
            eventual_timeout_seconds=40,
            infrastructure_retries=1,
            allowed_builtin_tools=[],
            denied_builtin_tools=[],
            compile_command=[],
            prove_command=[],
            inference_command=[],
            check_candidate_command=[],
        )
        config_path = root / "config.json"
        config_path.write_text(json.dumps(asdict(config)), encoding="utf-8")
        return corpus_manifest, {"acceptance": plugins}, config_path, commit

    def test_single_replicate_round_is_scheduled_and_readable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = self._inputs(root)
            output = root / "pilot"

            manifest = build_pilot(
                corpus_manifest,
                plugins,
                output,
                commit,
                config_path,
                replicates=1,
                round_id="baseline-001",
            )
            shape = load_round_shape(output)

            tasks = len(TOY_TASKS)
            self.assertEqual(tasks * 3, manifest["runs"])
            self.assertEqual(tasks * 3, len(sorted((output / "runs").glob("*.json"))))
            self.assertEqual(tasks * 3, shape.runs)
            self.assertEqual(tasks, shape.blocks)
            self.assertEqual(1, shape.replicates)

    def test_a_schedule_is_not_written_over_an_existing_round(self) -> None:
        # Rescheduling in place would replace the snapshot and manifests that
        # already-dispatched cells resolve against, while dispatch skips a cell
        # whose judge.json exists -- mixing two schedule versions in one round.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = self._inputs(root)
            output = root / "pilot"
            build_pilot(corpus_manifest, plugins, output, commit, config_path, replicates=1)

            with self.assertRaises(FileExistsError) as raised:
                build_pilot(
                    corpus_manifest, plugins, output, commit, config_path, replicates=1
                )
            self.assertIn("written once", str(raised.exception))

    def test_builds_loadable_fresh_run_manifests_for_every_cell(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = self._inputs(root)

            output = root / "pilot"
            manifest = build_pilot(
                corpus_manifest, plugins, output, commit, config_path
            )
            expected_blocks = len(TOY_TASKS) * 3
            expected_runs = expected_blocks * 3
            self.assertEqual(expected_blocks, manifest["blocks"])
            self.assertEqual(expected_runs, manifest["runs"])
            run_paths = sorted((output / "runs").glob("*.json"))
            self.assertEqual(expected_runs, len(run_paths))
            session_keys = set()
            for path in run_paths:
                spec = RunSpec.load(path).resolve_paths(path).spec
                self.assertEqual(
                    manifest["experiment_config_sha256"],
                    spec.experiment_config_sha256,
                )
                session_keys.add((spec.task_id, spec.replicate, spec.arm))
                self.assertEqual(
                    spec.plugin_manifest_sha256,
                    sha256_file((path.parent / spec.plugin_dir) / "move-flow-manifest.json"),
                )
            self.assertEqual(expected_runs, len(session_keys))


class TaskSubsetTest(unittest.TestCase):
    """A round may spend its budget on more replicates of fewer tasks."""

    def test_only_the_named_tasks_are_scheduled(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = PilotScheduleTest._inputs(root)

            manifest = build_pilot(
                corpus_manifest,
                plugins,
                root / "round",
                commit,
                config_path,
                replicates=2,
                round_id="subset-001",
                task_names=["pow", "double"],
            )

            self.assertEqual(2, manifest["task_count"])
            self.assertEqual(
                ["double", "pow"],
                sorted(task["task_id"] for task in manifest["tasks"]),
            )
            # two tasks, two replicates, three arms
            self.assertEqual(12, manifest["runs"])

    def test_an_unknown_task_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = PilotScheduleTest._inputs(root)

            with self.assertRaisesRegex(ValueError, "unknown corpus sample"):
                build_pilot(
                    corpus_manifest,
                    plugins,
                    root / "round",
                    commit,
                    config_path,
                    round_id="subset-002",
                    task_names=["pow", "nonesuch"],
                )


class ControlLevelTest(unittest.TestCase):
    """A level may be held as a cheaper control than the level under study."""

    @staticmethod
    def _layered(root, commit):
        levels = ("baseline", "acceptance")
        for level in levels:
            for arm in ARMS:
                plugin = root / "levels" / level / arm
                plugin.mkdir(parents=True)
                (plugin / "move-flow-manifest.json").write_text(
                    json.dumps(
                        {
                            "inference_tactic": arm,
                            "evaluation_mode": True,
                            "flow_source_commit": commit,
                            "feedback_level": level,
                        }
                    ),
                    encoding="utf-8",
                )
        return {level: {arm: root / "levels" / level / arm for arm in ARMS} for level in levels}

    def test_a_control_level_appears_only_in_early_replicates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, _, config_path, commit = PilotScheduleTest._inputs(root)

            manifest = build_pilot(
                corpus_manifest,
                self._layered(root, commit),
                root / "round",
                commit,
                config_path,
                replicates={"acceptance": 3, "baseline": 1},
                round_id="control-001",
                task_names=["pow", "double"],
            )
            shape = load_round_shape(root / "round")

            # 2 tasks x 3 arms x (3 acceptance + 1 baseline)
            self.assertEqual(24, manifest["runs"])
            self.assertEqual(24, shape.runs)
            self.assertEqual(3, manifest["replicates"])
            self.assertEqual({"acceptance": 3, "baseline": 1}, manifest["replicates_by_level"])

            by_replicate: dict[int, set[str]] = {}
            for path in sorted((root / "round" / "runs").glob("*.json")):
                spec = RunSpec.load(path)
                by_replicate.setdefault(spec.replicate, set()).add(spec.feedback_level)
            self.assertEqual({"acceptance", "baseline"}, by_replicate[1])
            self.assertEqual({"acceptance"}, by_replicate[2])
            self.assertEqual({"acceptance"}, by_replicate[3])

    def test_replicates_must_cover_every_scheduled_level(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, _, config_path, commit = PilotScheduleTest._inputs(root)

            with self.assertRaisesRegex(ValueError, "omit scheduled level"):
                build_pilot(
                    corpus_manifest,
                    self._layered(root, commit),
                    root / "round",
                    commit,
                    config_path,
                    replicates={"acceptance": 3},
                    round_id="control-002",
                    task_names=["pow"],
                )


class CorpusRoundTest(unittest.TestCase):
    """A round may be scheduled from a corpus manifest of shared-package targets."""

    @staticmethod
    def _corpus(root: Path) -> Path:
        package = root / "corpus" / "package"
        (package / "sources").mkdir(parents=True)
        (package / "Move.toml").write_text(
            '[package]\nname = "C"\nversion = "1.0.0"\n', encoding="utf-8"
        )
        (package / "sources" / "m.move").write_text(
            "module 0x42::m { public fun f(): u64 { 1 } }\n", encoding="utf-8"
        )
        manifest = root / "corpus" / "manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "corpus": "test",
                    "records": [
                        {
                            "task_id": "C-001",
                            "target": "0x42::m::f",
                            "required_contract_categories": ["normal-result"],
                            "provenance": "test",
                            "feature_strata": ["total"],
                        },
                        {
                            "task_id": "C-002",
                            "target": "0x42::m::g",
                            "required_contract_categories": ["abort"],
                            "provenance": "test",
                            "feature_strata": ["abort"],
                        },
                    ],
                }
            ),
            encoding="utf-8",
        )
        return manifest

    def test_every_sample_shares_one_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            manifest = self._corpus(root)

            built = build_pilot(
                manifest, plugins, root / "round", commit, config_path,
                replicates=1, round_id="corpus-001",
            )

            self.assertEqual(2, built["task_count"])
            self.assertEqual(6, built["runs"])  # 2 samples x 3 arms
            specs = [RunSpec.load(p) for p in sorted((root / "round" / "runs").glob("*.json"))]
            packages = {s.shared_package for s in specs}
            self.assertEqual(1, len(packages), "samples must share one package")
            self.assertEqual(
                {"0x42::m::f", "0x42::m::g"}, {s.target for s in specs}
            )

    def test_a_symlink_in_the_package_is_not_scheduled(self) -> None:
        # The digest walk does not descend a symlinked directory, and the
        # snapshot copy dereferences it, so a symlink could carry an arbitrary
        # tree past the digest check; the corpus contains none.
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "package"
            (package / "sources").mkdir(parents=True)
            (package / "sources" / "m.move").write_text("module 0x42::m {}\n", encoding="utf-8")
            _require_committed_corpus(package, None)
            (package / "sources" / "elsewhere").symlink_to(Path(temporary))
            with self.assertRaises(ValueError) as raised:
                _require_committed_corpus(package, None)
            self.assertIn("symlink", str(raised.exception))

    def test_a_package_that_is_not_the_committed_corpus_is_not_scheduled(self) -> None:
        # The manifest's per-file digests are a reproducibility claim; a round
        # must enforce it rather than snapshot whatever is on disk and hash the
        # copy, which would make an altered package look self-consistent.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            manifest = self._corpus(root)
            source = root / "corpus" / "package" / "sources" / "m.move"
            record = json.loads(manifest.read_text(encoding="utf-8"))
            record["generated_file_sha256"] = {
                "m.move": hashlib.sha256(source.read_bytes()).hexdigest()
            }
            manifest.write_text(json.dumps(record), encoding="utf-8")

            built = build_pilot(
                manifest, plugins, root / "round", commit, config_path,
                replicates=1, round_id="corpus-002",
            )
            self.assertEqual(2, built["task_count"])

            source.write_text(
                "module 0x42::m { public fun f(): u64 { 2 } }\n", encoding="utf-8"
            )
            with self.assertRaises(ValueError) as raised:
                build_pilot(
                    manifest, plugins, root / "round-altered", commit, config_path,
                    replicates=1, round_id="corpus-003",
                )
            self.assertIn("differs from the committed corpus", str(raised.exception))
            self.assertIn("m.move", str(raised.exception))

    def test_a_sample_the_screen_did_not_clear_is_not_scheduled(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            manifest = self._corpus(root)
            record = json.loads(manifest.read_text(encoding="utf-8"))
            record["records"][0]["screening_status"] = "blocked_signed_div"
            record["records"][1]["screening_status"] = "ready"
            manifest.write_text(json.dumps(record), encoding="utf-8")

            built = build_pilot(
                manifest, plugins, root / "round", commit, config_path,
                replicates=1, round_id="screened-001",
            )

            self.assertEqual(1, built["task_count"], "a blocked sample is not a task")

    def test_naming_a_blocked_sample_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            manifest = self._corpus(root)
            record = json.loads(manifest.read_text(encoding="utf-8"))
            blocked = record["records"][0]["task_id"]
            record["records"][0]["screening_status"] = "blocked_signed_div"
            manifest.write_text(json.dumps(record), encoding="utf-8")

            # Naming it explicitly must not be a way around the screen.
            with self.assertRaisesRegex(ValueError, "not cleared by screening"):
                build_pilot(
                    manifest, plugins, root / "round", commit, config_path,
                    replicates=1, round_id="screened-002", task_names=[blocked],
                )

    def test_an_unknown_sample_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            manifest = self._corpus(root)

            with self.assertRaisesRegex(ValueError, "unknown corpus sample"):
                build_pilot(
                    manifest, plugins, root / "round", commit, config_path,
                    replicates=1, round_id="corpus-002",
                    task_names=["C-001", "nope"],
                )


class RoundShapeTest(unittest.TestCase):
    @staticmethod
    def _write(root: Path, **overrides: object) -> Path:
        manifest = {
            "runs": 24,
            "blocks": 8,
            "replicates": 1,
            "task_count": 8,
            "feedback_levels": ["acceptance"],
        }
        manifest.update(overrides)
        (root / "pilot-manifest.json").write_text(
            json.dumps(manifest), encoding="utf-8"
        )
        return root

    def test_rejects_run_count_inconsistent_with_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            schedule = self._write(Path(temporary), runs=23)
            with self.assertRaisesRegex(ValueError, "23 runs"):
                load_round_shape(schedule)

    def test_rejects_block_count_inconsistent_with_replicates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            schedule = self._write(Path(temporary), replicates=2)
            with self.assertRaisesRegex(ValueError, "8 blocks"):
                load_round_shape(schedule)

    def test_rejects_missing_counts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            schedule = self._write(Path(temporary), runs="24")
            with self.assertRaisesRegex(ValueError, "positive `runs`"):
                load_round_shape(schedule)


if __name__ == "__main__":
    unittest.main()


class FeedbackFactorTest(unittest.TestCase):
    """A round may compare feedback levels inside its own blocks."""

    def test_every_block_holds_each_arm_at_each_level(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            levels = ("baseline", "acceptance")
            for level in levels:
                for arm in ARMS:
                    plugin = root / "levels" / level / arm
                    plugin.mkdir(parents=True)
                    (plugin / "move-flow-manifest.json").write_text(
                        json.dumps(
                            {
                                "inference_tactic": arm,
                                "evaluation_mode": True,
                                "flow_source_commit": commit,
                                "feedback_level": level,
                            }
                        ),
                        encoding="utf-8",
                    )
            layered = {
                level: {arm: root / "levels" / level / arm for arm in ARMS}
                for level in levels
            }
            del plugins

            manifest = build_pilot(
                corpus_manifest,
                layered,
                root / "round",
                commit,
                config_path,
                replicates=1,
                round_id="ablation-001",
            )
            shape = load_round_shape(root / "round")

            tasks = len(TOY_TASKS)
            self.assertEqual(tasks * 6, manifest["runs"])
            self.assertEqual(tasks, manifest["blocks"])
            self.assertEqual(("acceptance", "baseline"), shape.feedback_levels)

            cells: dict[int, set[tuple[str, str]]] = {}
            for path in sorted((root / "round" / "runs").glob("*.json")):
                spec = RunSpec.load(path)
                cells.setdefault(spec.block, set()).add((spec.arm, spec.feedback_level))
            self.assertEqual(tasks, len(cells))
            for block in cells.values():
                self.assertEqual(6, len(block))

    def test_a_plugin_declaring_the_wrong_level_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus_manifest, plugins, config_path, commit = PilotScheduleTest._inputs(root)

            with self.assertRaisesRegex(ValueError, "feedback level"):
                build_pilot(
                    corpus_manifest,
                    {"baseline": plugins["acceptance"]},
                    root / "round",
                    commit,
                    config_path,
                    replicates=1,
                    round_id="mismatch-001",
                )


class LegacyRoundTest(unittest.TestCase):
    """A round scheduled before the feedback factor stays readable."""

    def test_a_run_without_a_declared_level_loads(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "run.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 2,
                        "run_id": "legacy-001-task-r01-agent-only",
                        "task_id": "task",
                        "target": "0x42::m::f",
                        "arm": "agent_only",
                        "replicate": 1,
                        "shared_package": ".",
                        "task_patch": ".",
                        "plugin_dir": ".",
                        "plugin_manifest_sha256": "0" * 64,
                        "plugin_tree_sha256": "0" * 64,
                        "initial_tree_sha256": "0" * 64,
                        "mutant_manifest_sha256": "0" * 64,
                        "required_contract_categories": ["abort"],
                    }
                ),
                encoding="utf-8",
            )

            spec = RunSpec.load(path)

            self.assertIsNone(spec.feedback_level)

    def test_a_manifest_without_levels_reads_as_one_level(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "pilot-manifest.json").write_text(
                json.dumps(
                    {"runs": 30, "blocks": 10, "replicates": 1, "task_count": 10}
                ),
                encoding="utf-8",
            )

            shape = load_round_shape(root)

            self.assertEqual(30, shape.runs)
            self.assertEqual((), shape.feedback_levels)


class PerTaskProveBudget(unittest.TestCase):
    """A task may record a solver budget the round's single budget does not fit."""

    def test_a_task_without_a_budget_records_none(self) -> None:
        self.assertIsNone(
            RunSpec(
                schema_version=2,
                run_id="r",
                task_id="t",
                target="0x1::m::f",
                arm="agent_only",
                replicate=1,
                shared_package=".",
                task_patch=".",
                plugin_dir=".",
                plugin_manifest_sha256="0" * 64,
                plugin_tree_sha256="0" * 64,
                initial_tree_sha256="0" * 64,
                mutant_manifest_sha256="0" * 64,
                required_contract_categories=("abort",),
            ).prove_timeout_seconds
        )

    def test_a_recorded_budget_survives_a_round_trip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "run.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 2,
                        "run_id": "r",
                        "task_id": "t",
                        "target": "0x1::m::f",
                        "arm": "agent_only",
                        "replicate": 1,
                        "shared_package": ".",
                        "task_patch": ".",
                        "plugin_dir": ".",
                        "plugin_manifest_sha256": "0" * 64,
                        "plugin_tree_sha256": "0" * 64,
                        "initial_tree_sha256": "0" * 64,
                        "mutant_manifest_sha256": "0" * 64,
                        "required_contract_categories": ["abort"],
                        "prove_timeout_seconds": 120,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(120, RunSpec.load(path).prove_timeout_seconds)


class MutantManifestResolution(unittest.TestCase):
    """A round declares its scoring mode when it is scheduled."""

    def test_without_a_root_every_task_takes_the_sentinel(self) -> None:
        digests = _resolve_mutant_manifests([{"task_id": "a"}, {"task_id": "b"}], None)
        self.assertEqual(digests, {"a": NO_MUTANTS, "b": NO_MUTANTS})

    def test_a_task_without_a_manifest_fails_the_round(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _write_mutants(root / "a", ["m1"])
            with self.assertRaises(FileNotFoundError) as caught:
                _resolve_mutant_manifests([{"task_id": "a"}, {"task_id": "b"}], root)
            self.assertIn("b", str(caught.exception))

    def test_an_empty_manifest_does_not_count_as_a_mutant_set(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _write_mutants(root / "a", ["m1"])
            _write_mutants(root / "b", [])
            with self.assertRaises(FileNotFoundError):
                _resolve_mutant_manifests([{"task_id": "a"}, {"task_id": "b"}], root)

    def test_a_complete_root_binds_each_task_to_its_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _write_mutants(root / "a", ["m1"])
            _write_mutants(root / "b", ["m2"])
            digests = _resolve_mutant_manifests([{"task_id": "a"}, {"task_id": "b"}], root)
            self.assertEqual(digests["a"], sha256_file(root / "a" / "mutants.json"))
            self.assertNotEqual(digests["a"], digests["b"])


def _write_mutants(directory: Path, mutant_ids: list[str]) -> None:
    directory.mkdir(parents=True)
    (directory / "mutants.json").write_text(
        json.dumps({"mutants": [{"mutant_id": name} for name in mutant_ids]}),
        encoding="utf-8",
    )


class BalancedOrderTest(unittest.TestCase):
    def test_every_arm_order_is_used_equally_often(self) -> None:
        # Thirty blocks and six orders: each order five times, so no position
        # in a block correlates with an arm.
        orders = _balanced_arm_orders(random.Random(7), ARMS, 30)
        counts = Counter(orders)
        self.assertEqual(6, len(counts))
        self.assertEqual({5}, set(counts.values()))

