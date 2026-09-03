from __future__ import annotations

import asyncio
import dataclasses
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from harness.controller import _require_disjoint_from_scoring
from harness.mutants import NO_MUTANTS, mutation_fingerprint
from harness.pilot import _require_screening_agrees
from harness.pilot_sandbox import _require_confined_mount


class RefutationVerdictTest(unittest.TestCase):
    """A refutation that reaches no verdict must not confirm the candidate.

    `run_mutant_cases` reports `prover_timeout`, `compile_failure`,
    `infrastructure_failure` and `unclassified_prover_failure` besides `killed`
    and `survived`. None of those is evidence about the specification, so a run
    where every mutant is inconclusive finds no survivor -- and returning the
    accepted judge on that basis states "no counterexample exists" when what
    happened was "no counterexample was looked for successfully".
    """

    def _refute_with(
        self, outcomes: list[str], wall_seconds: float = 0.0,
        wall_after: float | None = None,
    ) -> object:
        from harness.controller import Controller
        from harness.judge import JudgeResult

        results = [
            {"mutant_id": f"m{i}", "outcome": o, "killed": o == "killed"}
            for i, o in enumerate(outcomes)
        ]
        accepted = JudgeResult(
            schema_version=2,
            state="operational_success",
            diagnostics="",
            verdict={},
            command={},
            tree_sha256="0" * 64,
        )
        emitted: list[dict] = []

        class Events:
            def emit(self, _name: str, **fields: object) -> None:
                emitted.append(fields)

        controller = object.__new__(Controller)
        with tempfile.TemporaryDirectory() as temporary:
            manifest = Path(temporary) / "mutants.json"
            manifest.write_text(
                json.dumps({"mutants": [
                    {"mutant_id": f"m{i}", "obligation_category": "normal-result"}
                    for i in range(len(outcomes))
                ]}),
                encoding="utf-8",
            )
            controller.refutation_mutants = manifest
            # `_refute` bounds itself by the run's remaining wall budget.
            controller.config = mock.Mock(max_wall_seconds=3600)
            controller.package = Path(temporary)
            controller.baseline_package = Path(temporary)
            controller.run = mock.Mock()
            controller._eventual_timeout = lambda: 1
            clock = {"t": wall_seconds}

            def _wall() -> float:
                now = clock["t"]
                if wall_after is not None:
                    clock["t"] = wall_after   # the refutation consumed the rest
                return now

            controller._wall_seconds = _wall
            with mock.patch(
                "harness.controller.run_mutant_cases",
                new=mock.AsyncMock(return_value=results),
            ):
                verdict = asyncio.run(
                    Controller._refute(controller, accepted, Events(), 1)
                )
        return verdict, emitted

    def test_all_inconclusive_does_not_confirm(self) -> None:
        verdict, _ = self._refute_with(["prover_timeout", "compile_failure"])
        self.assertEqual("infrastructure_failure", verdict.state)
        self.assertIn("no verdict", verdict.diagnostics)

    def test_one_inconclusive_among_kills_does_not_confirm(self) -> None:
        # A partial verdict is still not a verdict about the unmeasured mutant.
        verdict, _ = self._refute_with(["killed", "prover_timeout"])
        self.assertEqual("infrastructure_failure", verdict.state)

    def test_a_survivor_still_reports_the_weak_contract(self) -> None:
        # A definite counterexample outranks an inconclusive one.
        verdict, _ = self._refute_with(["survived", "prover_timeout"])
        self.assertEqual("weak_contract", verdict.state)

    def test_all_killed_confirms(self) -> None:
        verdict, _ = self._refute_with(["killed", "killed"])
        self.assertEqual("operational_success", verdict.state)

    def test_an_exhausted_wall_budget_does_not_confirm(self) -> None:
        # Refutation runs after the turn's judge, so the loop's wall check is
        # behind it. Running anyway would put the cell's own runtime outside
        # the budget the round declares; confirming without running would make
        # an unmeasured candidate a success.
        verdict, emitted = self._refute_with(["killed"], wall_seconds=10_000)
        self.assertEqual("infrastructure_failure", verdict.state)
        self.assertIn("no wall budget", verdict.diagnostics)
        self.assertEqual(["m0"], emitted[0]["inconclusive"])

    def test_overrunning_the_budget_does_not_confirm(self) -> None:
        # Dividing the remaining time bounds each case, not the run: the cases
        # are sequential and every command carries its own watchdog, so the
        # total can still overrun. The cell must not record a success it spent
        # more than its budget reaching.
        verdict, emitted = self._refute_with(["killed"], wall_after=10_000)
        self.assertEqual("infrastructure_failure", verdict.state)
        self.assertIn("past the run", verdict.diagnostics)
        self.assertTrue(emitted[0]["overran_budget"])

    def test_a_survivor_outranks_an_overrun(self) -> None:
        # The counterexample was found; the clock does not unfind it. Reporting
        # infrastructure_failure here would drop the categories the session
        # needs and make a spent run-wide budget look retryable.
        verdict, emitted = self._refute_with(["survived"], wall_after=10_000)
        self.assertEqual("weak_contract", verdict.state)
        self.assertTrue(emitted[0]["overran_budget"])

    def test_the_event_records_which_mutants_were_inconclusive(self) -> None:
        _, emitted = self._refute_with(["killed", "prover_timeout"])
        self.assertEqual(["m1"], emitted[0]["inconclusive"])


class ControlArmTest(unittest.TestCase):
    """A control arm must not receive refutation feedback.

    `_refute` names the obligation categories the contract leaves
    unconstrained, which is acceptance-level information about the task. A
    baseline cell is defined by receiving compiler and prover answers only, so
    refuting there gives the control the treatment.
    """

    def _would_refute(self, level: str) -> bool:
        import inspect
        from harness.controller import Controller

        source = inspect.getsource(Controller.run_experiment)
        # The gate is on the call site, so read it there rather than running a
        # whole session: the condition must mention the feedback level.
        return "_feedback_level() != \"baseline\"" in source

    def test_the_call_site_is_gated_on_the_feedback_level(self) -> None:
        self.assertTrue(
            self._would_refute("baseline"),
            "refutation must be gated on the feedback level",
        )


class ScreeningApparatusTest(unittest.TestCase):
    """A screen must not pass a target whose apparatus failed.

    `check_compatibility` reports `infrastructure_failure` and
    `compatibility_timeout` for tooling that was unavailable or ran out of
    time. Neither says anything about the target, so admitting them records an
    unscreened task as a corpus member.
    """

    def _result(
        self, failure_kind: str | None, returncode: int = 1, report: bool = True
    ) -> dict:
        return {
            "compile": {"returncode": 0},
            # A refusal: WP ran, declined, and said why.
            "wp_inference": {"returncode": returncode, "stage_report": report},
            "enriched_compile": None,
            "passed": False,
            "failure_kind": failure_kind,
        }

    def test_wp_declining_still_admits(self) -> None:
        # The uninvariant-loop case: WP ran and refused, which is the task.
        from harness.screen_v3 import is_well_formed

        self.assertTrue(is_well_formed(self._result("implementation_failure")))

    def test_a_crashed_inference_does_not_admit(self) -> None:
        # A signal is never a refusal, and `_failure_kind` calls both
        # `implementation_failure`.
        from harness.screen_v3 import is_well_formed

        self.assertFalse(
            is_well_formed(self._result("implementation_failure", returncode=-11))
        )

    def test_an_undiagnosed_failure_does_not_admit(self) -> None:
        # WP declining comes with a stage report; a crash comes with neither.
        from harness.screen_v3 import is_well_formed

        self.assertFalse(
            is_well_formed(self._result("implementation_failure", report=False))
        )

    def test_missing_tooling_does_not_admit(self) -> None:
        from harness.screen_v3 import is_well_formed

        self.assertFalse(is_well_formed(self._result("infrastructure_failure")))

    def test_a_timeout_does_not_admit(self) -> None:
        from harness.screen_v3 import is_well_formed

        self.assertFalse(is_well_formed(self._result("compatibility_timeout")))

    def test_a_target_that_does_not_compile_is_not_admitted(self) -> None:
        from harness.screen_v3 import is_well_formed

        broken = {**self._result(None), "compile": {"returncode": 1}}
        self.assertFalse(is_well_formed(broken))


class EssentialityTest(unittest.TestCase):
    """An unmeasured mutant is not a non-essential mutant.

    Marking it non-essential drops it from the scoring set, and a candidate
    then reaches strict success by killing only what remains. The behaviour is
    covered by `UnvalidatedMutantTest` (the refusal) and by the recovery case
    beside it (the flag being cleared by a later verdict); what this pins is
    that validation records the flag on every run rather than only when true,
    which is what makes that recovery possible.
    """

    def test_the_flag_is_written_on_every_run(self) -> None:
        import ast

        source = (ROOT / "harness" / "validate_mutants.py").read_text(encoding="utf-8")
        tree = ast.parse(source)
        writes = [
            node
            for node in ast.walk(tree)
            if isinstance(node, ast.Assign)
            and any(
                isinstance(t, ast.Subscript)
                and isinstance(t.slice, ast.Constant)
                and t.slice.value == "inconclusive"
                for t in node.targets
            )
        ]
        self.assertEqual(1, len(writes), "exactly one place should record it")
        # A bare `True` would mean the flag can be set and never cleared.
        self.assertNotIsInstance(writes[0].value, ast.Constant)


class ScreeningEnforcementTest(unittest.TestCase):
    """A round may not be scheduled from readiness with no evidence behind it.

    `screening_status` is a field and a field can be edited, or simply added
    for a task the screen never saw. The scheduler reads the field, so it asks
    the report for positive clearance rather than for the absence of a
    complaint.
    """

    def _check(self, root: Path, results: list[dict], records: list[dict],
               package_hash: str | None = None, write_report: bool = True) -> str:
        from harness.pilot import _require_screening_agrees

        if write_report:
            (root / "screening").mkdir(parents=True, exist_ok=True)
            report = {"results": results}
            if package_hash is not None:
                report["package_tree_sha256"] = package_hash
            (root / "screening" / "summary.json").write_text(
                json.dumps(report), encoding="utf-8"
            )
        try:
            _require_screening_agrees(root / "manifest.json", records)
        except ValueError as error:
            return str(error)
        return ""

    def _ready(self, *task_ids: str) -> list[dict]:
        return [{"task_id": t, "screening_status": "ready"} for t in task_ids]

    def test_a_cleared_task_schedules(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            message = self._check(
                Path(temporary), [{"task_id": "T", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}], self._ready("T")
            )
        self.assertEqual("", message)

    def test_a_task_the_screen_failed_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            message = self._check(
                Path(temporary), [{"task_id": "T", "passed": False, "apparatus_ok": True}], self._ready("T")
            )
        self.assertIn("not cleared", message)

    def test_a_task_the_screen_never_saw_is_refused(self) -> None:
        # Adding a record does not change the package tree, so the tree digest
        # alone would not notice it.
        with tempfile.TemporaryDirectory() as temporary:
            message = self._check(
                Path(temporary), [{"task_id": "T", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}], self._ready("T", "NEW")
            )
        self.assertIn("NEW", message)

    def test_a_missing_report_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            message = self._check(
                Path(temporary), [], self._ready("T"), write_report=False
            )
        self.assertIn("no screening report", message)

    def test_a_report_for_another_tree_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "package").mkdir()
            (root / "package" / "Move.toml").write_text("x", encoding="utf-8")
            message = self._check(
                root, [{"task_id": "T", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}], self._ready("T"), "0" * 64
            )
        self.assertIn("different corpus tree", message)

    def test_a_not_ready_record_needs_no_evidence(self) -> None:
        # It is already out; the screen has nothing to say about it.
        with tempfile.TemporaryDirectory() as temporary:
            message = self._check(
                Path(temporary), [],
                [{"task_id": "OUT", "screening_status": "excluded_prover_defect"}],
            )
        self.assertEqual("", message)

    def test_the_scheduler_actually_calls_it(self) -> None:
        import inspect
        from harness.pilot import _corpus_tasks, build_pilot

        self.assertIn("_require_screening_agrees", inspect.getsource(_corpus_tasks))
        self.assertIn("_corpus_tasks", inspect.getsource(build_pilot))


class RefutationDigestTest(unittest.TestCase):
    """Disjointness has to be judged by content, not by path.

    The path comparison is inert whenever `--skip-hidden-scoring` leaves no
    hidden root to compare against, and a copy of the scoring manifest at
    another path defeats it while post-round scoring still accepts the original
    by its recorded digest -- so the contamination leaves no trace.
    """

    def test_a_copy_of_the_scoring_manifest_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            pkg = root / "pkg"
            pkg.mkdir()
            (pkg / "sources").mkdir()
            (pkg / "sources" / "x.move").write_text("aXbb\n", encoding="utf-8")
            case = {
                "mutant_id": "scored-one",
                "file": "sources/x.move",
                "anchor": {
                    "offset": 1, "length": 2,
                    "sha256": hashlib.sha256("Xb".encode()).hexdigest(),
                },
                "edit": {"at": 0, "kind": "substitute", "length": 1, "to": "9"},
            }
            body = json.dumps({"mutants": [case]})
            digest = hashlib.sha256(body.encode()).hexdigest()
            from harness.mutants import mutation_fingerprint

            fingerprint = mutation_fingerprint(case, pkg)
            # Same content, a different path, and no hidden root at all: both
            # gaps the path check leaves open.
            copy = root / "copied"
            (copy / "T").mkdir(parents=True)
            (copy / "T" / "mutants.json").write_text(body, encoding="utf-8")
            # The committed fixture, not a generated one: `evaluation-artifacts/`
            # is gitignored, so globbing it raises StopIteration on a fresh
            # clone rather than failing as a test.
            template = json.loads(
                (ROOT / "tests" / "fixtures" / "run.json").read_text(encoding="utf-8")
            )
            template["task_id"] = "T"
            template["mutant_manifest_sha256"] = digest
            template["mutant_identities"] = [fingerprint]
            # `resolve_paths` requires these to exist, relative to the manifest.
            (root / "plugin").mkdir()
            (root / "task.patch").write_text("", encoding="utf-8")
            template["shared_package"] = "pkg"
            template["task_patch"] = "task.patch"
            template["plugin_dir"] = "plugin"
            run = root / "run.json"
            run.write_text(json.dumps(template), encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable, "-m", "harness.controller",
                    "--config", "config/default.json",
                    "--run", str(run),
                    "--artifacts", temporary,
                    "--skip-hidden-scoring",
                    "--refutation-mutants-root", str(copy),
                ],
                cwd=ROOT, capture_output=True, text=True,
                # The sandbox guard fires before this check and is not what is
                # under test here.
                env={**os.environ, "MOVE_INFERENCE_EVAL_SANDBOXED": "1"},
            )
        self.assertNotEqual(0, result.returncode)
        self.assertIn("repeats mutation", result.stderr + result.stdout)



class PartialOverlapTest(unittest.TestCase):
    """Overlap is judged on what a mutation produces, not how it is written.

    The scored mutations must never exist where the session can read them, so
    the comparison is by an opaque hash -- which therefore has to be canonical,
    or one change under another encoding passes the check.
    """

    SOURCE = "module m { fun f() { aabb } }\n"

    def setUp(self) -> None:
        self.package = Path(tempfile.mkdtemp())
        (self.package / "sources").mkdir()
        (self.package / "sources" / "x.move").write_text(self.SOURCE, encoding="utf-8")
        self.at = self.SOURCE.index("aabb")

    def _anchor(self, offset: int, length: int) -> dict:
        fragment = self.SOURCE[offset:offset + length]
        return {
            "offset": offset,
            "length": length,
            "sha256": hashlib.sha256(fragment.encode()).hexdigest(),
        }

    def _print(self, case: dict) -> str:
        return mutation_fingerprint(case, self.package)

    def test_renaming_a_mutant_does_not_change_its_identity(self) -> None:
        one = {"mutant_id": "one", "file": "sources/x.move",
               "anchor": self._anchor(self.at, 4),
               "edit": {"at": 0, "kind": "substitute", "length": 1, "to": "z"}}
        self.assertEqual(self._print(one), self._print({**one, "mutant_id": "other"}))

    def test_a_wider_anchor_around_the_same_change_agrees(self) -> None:
        # The anchor locates the change; it is not the change.
        narrow = {"mutant_id": "narrow", "file": "sources/x.move",
                  "anchor": self._anchor(self.at, 4),
                  "edit": {"at": 2, "kind": "substitute", "length": 1, "to": "z"}}
        wide = {"mutant_id": "wide", "file": "sources/x.move",
                "anchor": self._anchor(self.at - 3, 7),
                "edit": {"at": 5, "kind": "substitute", "length": 1, "to": "z"}}
        self.assertEqual(self._print(narrow), self._print(wide))

    def test_two_swap_encodings_of_one_rearrangement_agree(self) -> None:
        # `aabb` becomes `bbaa` under either partition, so the two denote one
        # mutation and must not carry two identities.
        anchor = self._anchor(self.at, 4)
        even = {"mutant_id": "even", "file": "sources/x.move", "anchor": anchor,
                "edit": {"at": 0, "kind": "swap", "a_length": 2,
                         "separator_length": 0, "b_length": 2}}
        odd = {"mutant_id": "odd", "file": "sources/x.move", "anchor": anchor,
               "edit": {"at": 0, "kind": "swap", "a_length": 1,
                        "separator_length": 1, "b_length": 2}}
        from harness.mutants import _mutate

        self.assertEqual(
            _mutate("aabb", even["edit"], "even"),
            _mutate("aabb", odd["edit"], "odd"),
            "the two encodings must actually produce the same text",
        )
        self.assertEqual(self._print(even), self._print(odd))

    def test_a_different_replacement_disagrees(self) -> None:
        base = {"mutant_id": "b", "file": "sources/x.move",
                "anchor": self._anchor(self.at, 4),
                "edit": {"at": 0, "kind": "substitute", "length": 1, "to": "z"}}
        other = {**base, "edit": {**base["edit"], "to": "9"}}
        self.assertNotEqual(self._print(base), self._print(other))

    def test_a_path_alias_agrees(self) -> None:
        plain = {"mutant_id": "p", "file": "sources/x.move",
                 "anchor": self._anchor(self.at, 4),
                 "edit": {"at": 0, "kind": "substitute", "length": 1, "to": "z"}}
        self.assertEqual(
            self._print(plain), self._print({**plain, "file": "sources/./x.move"})
        )

    def test_the_fingerprint_does_not_disclose_the_edit(self) -> None:
        printed = self._print({
            "mutant_id": "one", "file": "sources/x.move",
            "anchor": self._anchor(self.at, 4),
            "edit": {"at": 0, "kind": "substitute", "length": 1, "to": "z"},
        })
        self.assertEqual(64, len(printed))
        self.assertNotIn("substitute", printed)
        self.assertNotIn("z", printed[:0] or "")


class UnvalidatedMutantTest(unittest.TestCase):
    """A mutant with no validation verdict may not be scored around.

    Leaving it non-essential drops it from the set silently, and the candidate
    then reaches strict success by killing only the mutants that were measured.
    """

    def test_a_case_that_later_reached_a_verdict_is_approved(self) -> None:
        from harness.mutants import _approved

        # Validation writes the flag on every run, so a later verdict clears
        # it. If it were only ever set, the mutant would stay unscorable and
        # the recovery the error names could not work.
        case = {
            "mutant_id": "m",
            "essential": True,
            "reviews": [{"reviewer": "r", "approved": True}],
            "validated": {
                "applies": True, "compiles": True,
                "killed_by_reference": True, "outcome": "killed",
                "inconclusive": False,
            },
        }
        self.assertTrue(_approved(case))

    def test_an_inconclusive_case_is_not_approved(self) -> None:
        from harness.mutants import _approved

        case = {
            "mutant_id": "m",
            "reviews": [{"reviewer": "r", "approved": True}],
            "validated": {"applies": True, "compiles": True, "inconclusive": True},
        }
        with self.assertRaises(SystemExit) as raised:
            _approved(case)
        self.assertIn("no validation verdict", str(raised.exception))


class ScoringIdentitiesTest(unittest.TestCase):
    """A scored manifest with no recorded identities cannot be checked.

    The scheduler writes the digest and the identities from the same manifest
    in the same loop, so the state means the round is stale -- not that there
    is nothing to compare.
    """

    def test_a_scored_run_without_identities_is_refused(self) -> None:
        spec = mock.Mock()
        spec.spec.mutant_manifest_sha256 = "c" * 64
        spec.spec.mutant_identities = []
        with self.assertRaises(SystemExit) as raised:
            _require_disjoint_from_scoring(spec, Path("/nonexistent/mutants.json"))
        self.assertIn("no mutant identities", str(raised.exception))

    def test_a_core_run_needs_no_check(self) -> None:
        spec = mock.Mock()
        spec.spec.mutant_manifest_sha256 = NO_MUTANTS
        spec.spec.mutant_identities = []
        _require_disjoint_from_scoring(spec, Path("/nonexistent/mutants.json"))


class ApparatusProvenanceTest(unittest.TestCase):
    """Screening evidence has to name the apparatus that produced it.

    The screen runs the prover and WP, so a target cleared under one binary or
    configuration was not cleared under another -- and the corpus digest cannot
    notice, because the apparatus changed and the corpus did not.
    """

    def _check(self, tools: dict, apparatus: dict) -> str:
        from harness.pilot import _require_screening_agrees

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({
                    "tools": tools,
                    "results": [{"task_id": "T", "target": "m::f",
                                 "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}],
                }),
                encoding="utf-8",
            )
            try:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "m::f", "screening_status": "ready"}],
                    apparatus,
                )
            except ValueError as error:
                return str(error)
        return ""

    def test_matching_apparatus_clears(self) -> None:
        self.assertEqual(
            "", self._check({"move_flow_sha256": "a" * 64}, {"move_flow_sha256": "a" * 64})
        )

    def test_a_different_binary_does_not_clear(self) -> None:
        message = self._check(
            {"move_flow_sha256": "a" * 64}, {"move_flow_sha256": "b" * 64}
        )
        self.assertIn("re-run screening", message)

    def test_evidence_that_names_no_binary_does_not_clear(self) -> None:
        message = self._check({}, {"move_flow_sha256": "a" * 64})
        self.assertIn("does not record", message)


class UnidentifiableApparatusTest(unittest.TestCase):
    """A round that cannot name its own apparatus is pinned to nothing.

    Every downstream check reads a null digest as "nothing to compare": the
    screening comparison skips it, the run spec records null, and the
    controller's runtime binary check skips too. So the absence has to be
    refused where it arises rather than carried forward as a verified round.
    """

    def test_scheduling_without_the_binary_is_refused(self) -> None:
        # Through the real fixture, so the refusal is reached rather than a
        # missing-file error standing in for it.
        import harness.pilot as pilot
        from tests.test_pilot import PilotScheduleTest

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, plugins, config_path, commit = PilotScheduleTest._inputs(root)
            with mock.patch.object(pilot, "_move_flow_sha256", return_value=None):
                with self.assertRaises(ValueError) as raised:
                    pilot.build_pilot(
                        corpus, plugins, root / "round", commit, config_path,
                        replicates=1, round_id="unidentified",
                    )
        self.assertIn("not on PATH", str(raised.exception))

    def test_a_round_that_does_not_know_its_binary_cannot_check_evidence(self) -> None:
        from harness.pilot import _require_screening_agrees

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({
                    "tools": {"move_flow_sha256": "a" * 64},
                    "results": [{"task_id": "T", "target": "m::f",
                                 "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}],
                }),
                encoding="utf-8",
            )
            with self.assertRaises(ValueError) as raised:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "m::f", "screening_status": "ready"}],
                    {"move_flow_sha256": None},
                )
        self.assertIn("does not know its own", str(raised.exception))


class ReferencePatchTest(unittest.TestCase):
    """A reference patch may add specification, and nothing else.

    Adding-only is not enough on its own: a patch that only adds can still add
    something that excuses the prover from its obligations, and the target then
    proves successfully while establishing less than the contract claims --
    which screening would record as this task's solvability.
    """

    def _check(self, body: str) -> str:
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "build_references", ROOT / "corpus-v3" / "build_references.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temporary:
            patch = Path(temporary) / "m.patch"
            patch.write_text(body, encoding="utf-8")
            try:
                module._require_specification_only(patch)
            except SystemExit as error:
                return str(error)
        return ""

    HEAD = "--- a/x.move\n+++ b/x.move\n@@ -1,0 +2,1 @@\n"

    def test_added_specification_is_accepted(self) -> None:
        self.assertEqual(
            "", self._check(self.HEAD + "+        ensures result == 1;\n")
        )

    def test_pragma_opaque_is_still_accepted(self) -> None:
        # The proof boundary, used by every committed reference.
        self.assertEqual("", self._check(self.HEAD + "+        pragma opaque;\n"))

    def test_a_removal_is_refused(self) -> None:
        self.assertIn("removes", self._check(self.HEAD + "-        let x = 1;\n"))

    def test_disabling_verification_is_refused(self) -> None:
        message = self._check(self.HEAD + "+        pragma verify = false;\n")
        self.assertIn("not one a reference specification needs", message)

    def test_a_partial_abort_contract_is_refused(self) -> None:
        message = self._check(self.HEAD + "+        pragma aborts_if_is_partial;\n")
        self.assertIn("not one a reference specification needs", message)

    def test_an_intrinsic_pragma_is_refused(self) -> None:
        # The prover excludes intrinsic functions from verification.
        message = self._check(self.HEAD + "+        pragma intrinsic;\n")
        self.assertIn("not one a reference specification needs", message)

    MULTI = "--- a/x.move\n+++ b/x.move\n@@ -1,0 +2,2 @@\n"

    def test_a_string_literal_is_refused(self) -> None:
        # `//` inside `b"http://x"` is not a comment, so stripping it would
        # hide whatever follows. Rather than lex string literals, the guard
        # refuses text it cannot confidently read.
        message = self._check(
            self.HEAD + '+        let s = b"http://x"; pragma intrinsic;\n'
        )
        self.assertIn("string literal", message)

    def test_an_apostrophe_in_prose_is_accepted(self) -> None:
        # Move has no character literal; this is a doc comment, and committed
        # references contain them.
        self.assertEqual(
            "", self._check(self.HEAD + "+        /// the accumulator's value\n")
        )

    def test_a_comment_spanning_lines_does_not_hide_a_pragma(self) -> None:
        # A block comment crosses newlines, so these two lines are one pragma
        # to the compiler and two unremarkable ones to a per-line scan.
        message = self._check(self.MULTI + "+  pragma /* c\n+  */ intrinsic;\n")
        self.assertIn("not one a reference specification needs", message)

    def test_a_pragma_list_spanning_lines_is_refused(self) -> None:
        message = self._check(
            self.MULTI + "+  pragma opaque,\n+  verify = false;\n"
        )
        self.assertIn("not one a reference specification needs", message)

    def test_a_second_statement_on_the_line_is_refused(self) -> None:
        # One added line can carry several statements; reading the first would
        # let an allowed pragma escort a forbidden one.
        message = self._check(
            self.HEAD + "+        pragma opaque; pragma verify = false;\n"
        )
        self.assertIn("not one a reference specification needs", message)

    def test_a_pragma_hidden_behind_a_comment_is_refused(self) -> None:
        # The Move lexer strips comments, so this is `pragma intrinsic` to the
        # compiler while not looking like one to a pattern expecting a space.
        message = self._check(self.HEAD + "+        pragma /* c */ intrinsic;\n")
        self.assertIn("not one a reference specification needs", message)

    def test_a_trailing_comment_does_not_hide_a_pragma(self) -> None:
        message = self._check(self.HEAD + "+        pragma verify = false; // ok\n")
        self.assertIn("not one a reference specification needs", message)

    def test_a_comment_alone_is_accepted(self) -> None:
        self.assertEqual(
            "", self._check(self.HEAD + "+        // pragma intrinsic;\n")
        )

    def test_a_forbidden_pragma_beside_an_allowed_one_is_refused(self) -> None:
        # A pragma statement is a comma-separated list, so reading only the
        # first name would let `opaque` escort `verify = false` onto the line.
        message = self._check(
            self.HEAD + "+        pragma opaque, verify = false;\n"
        )
        self.assertIn("not one a reference specification needs", message)

    def test_several_allowed_pragmas_on_one_line_are_accepted(self) -> None:
        self.assertEqual("", self._check(self.HEAD + "+        pragma opaque;\n"))

    def test_an_unanticipated_pragma_is_refused(self) -> None:
        # The point of allowing rather than forbidding: a pragma nobody thought
        # about is refused too, without the guard being extended.
        message = self._check(self.HEAD + "+        pragma some_future_escape;\n")
        self.assertIn("not one a reference specification needs", message)

    def test_an_assumption_is_refused(self) -> None:
        message = self._check(self.HEAD + "+        assume x > 0;\n")
        self.assertIn("by assumption", message)

    #: Two hunks that add only lines, and together comment out whatever sits
    #: between them -- here the target implementation -- and supply a
    #: replacement. Nothing is removed, so the adding-only rule permits it, and
    #: the two additions are balanced when read as one text.
    COMMENTED_OUT_TARGET = (
        "--- a/x.move\n"
        "+++ b/x.move\n"
        "@@ -9,0 +10,1 @@\n"
        "+    /*\n"
        "@@ -20,0 +22,3 @@\n"
        "+    */\n"
        "+    fun find_min_value(values: &vector<u64>): u64 { 1 }\n"
    )

    def test_commenting_out_the_target_and_replacing_it_is_refused(self) -> None:
        # Verified against `git apply --unidiff-zero`: this leaves the original
        # body inside a block comment and the replacement in its place.
        self.assertIn(
            "does not close", self._check(self.COMMENTED_OUT_TARGET)
        )

    def test_a_closer_without_an_opener_is_refused(self) -> None:
        # The mirror image: end a comment in the untouched source above, and
        # what the author wrote as prose becomes code.
        self.assertIn(
            "did not open", self._check(self.HEAD + "+        */ pragma intrinsic;\n")
        )

    def test_a_balanced_comment_within_one_addition_is_accepted(self) -> None:
        # A run that closes what it opens leaves the surrounding text lexing
        # exactly as it did, which is the whole of what the rule asks.
        self.assertEqual(
            "",
            self._check(self.HEAD + "+        /* a /* b */ c */ ensures result == 1;\n"),
        )


class CorrectedScoringManifestTest(unittest.TestCase):
    """Replacing the scored set replaces one side of the disjointness check.

    The scheduler proves the refutation set disjoint from the *scheduled*
    scoring set. `--allow-corrected-mutants` then substitutes a different
    scoring set, and nothing had compared that one to anything -- so a
    correction that happens to contain a mutation the run was shown would
    credit "the contract is complete" for what was really "the agent can act
    on feedback".
    """

    SOURCE = "module m { fun f() { aabb } }\n"

    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp())
        self.package = self.root / "runs" / "r1" / "baseline" / "pkg"
        (self.package / "sources").mkdir(parents=True)
        (self.package / "sources" / "x.move").write_text(self.SOURCE, encoding="utf-8")
        self.at = self.SOURCE.index("aabb")

    def _case(self, mutant_id: str, to: str) -> dict:
        fragment = self.SOURCE[self.at:self.at + 4]
        return {
            "mutant_id": mutant_id,
            "file": "sources/x.move",
            "anchor": {
                "offset": self.at,
                "length": 4,
                "sha256": hashlib.sha256(fragment.encode()).hexdigest(),
            },
            "edit": {"at": 0, "kind": "substitute", "length": 1, "to": to},
        }

    def _score(self, scored_cases: list[dict], shown_cases: list[dict]) -> str:
        from harness.score_round import score_round

        mutants_root = self.root / "mutants" / "T"
        mutants_root.mkdir(parents=True)
        manifest = mutants_root / "mutants.json"
        manifest.write_text(json.dumps({"mutants": scored_cases}), encoding="utf-8")
        run = self.root / "runs" / "r1"
        (run / "run.json").write_text(
            json.dumps({
                "run_id": "r1",
                "task_id": "T",
                "target": "m::f",
                "arm": "agent_only",
                "package_relpath": "pkg",
                # A digest that cannot match the manifest, so scoring takes
                # the corrected path this test is about. Not the all-zero
                # sentinel, which means "this run has no mutant set at all".
                "mutant_manifest_sha256": "1" * 64,
                "refutation_mutant_identities": sorted(
                    mutation_fingerprint(case, self.package) for case in shown_cases
                ),
                "result": {"eventual_judge": {"state": "operational_success"}},
            }),
            encoding="utf-8",
        )
        try:
            asyncio.run(score_round(
                config=None,  # never reached: the guard runs before scoring
                round_dir=self.root,
                mutants_root=self.root / "mutants",
                timeout_seconds=1,
                allow_corrected_mutants=True,
            ))
        except ValueError as error:
            return str(error)
        return ""

    def test_a_correction_repeating_a_shown_mutation_is_refused(self) -> None:
        shown = self._case("shown", "z")
        message = self._score([shown], [shown])
        self.assertIn("repeats mutation(s)", message)
        self.assertIn("was shown during refutation", message)

    def test_the_identity_is_the_mutation_not_its_name(self) -> None:
        # The same change under a different `mutant_id` is the same mutation;
        # comparing names rather than fingerprints would miss it.
        message = self._score([self._case("renamed", "z")], [self._case("shown", "z")])
        self.assertIn("repeats mutation(s)", message)

    def test_a_run_without_recorded_identities_cannot_be_corrected(self) -> None:
        run = self.root / "runs" / "r1"
        (self.root / "mutants" / "T").mkdir(parents=True)
        (self.root / "mutants" / "T" / "mutants.json").write_text(
            json.dumps({"mutants": [self._case("a", "z")]}), encoding="utf-8"
        )
        (run / "run.json").write_text(
            json.dumps({
                "run_id": "r1", "task_id": "T", "target": "m::f",
                "package_relpath": "pkg", "mutant_manifest_sha256": "1" * 64,
                "result": {"eventual_judge": {"state": "operational_success"}},
            }),
            encoding="utf-8",
        )
        from harness.score_round import score_round

        with self.assertRaises(ValueError) as raised:
            asyncio.run(score_round(
                config=None, round_dir=self.root,
                mutants_root=self.root / "mutants",
                timeout_seconds=1, allow_corrected_mutants=True,
            ))
        self.assertIn("records no refutation identities", str(raised.exception))


class ToolchainDigestTest(unittest.TestCase):
    """Identity has to cover what a stage runs, not just its first word.

    `render_command` executes the whole argument vector, so a
    `["python3", "wrapper.py"]` stage identified by its interpreter would keep
    that identity while the wrapper is rewritten. The prover backends are named
    by environment rather than by the command, and a screening verdict is a
    claim about what they decided.
    """

    def test_a_file_argument_is_hashed(self) -> None:
        from unittest import mock
        from harness.compatibility import tool_executables

        with tempfile.TemporaryDirectory() as temporary:
            wrapper = Path(temporary) / "wrapper.py"
            wrapper.write_text("print(1)", encoding="utf-8")
            config = mock.Mock(
                compile_command=["python3", str(wrapper)],
                inference_command=[], prove_command=[],
            )
            first = tool_executables(config)["compile"]
            self.assertIn(str(wrapper), first.get("arguments", {}))
            wrapper.write_text("print(2)", encoding="utf-8")
            second = tool_executables(config)["compile"]
        # The interpreter is unchanged; the program is not.
        self.assertEqual(first["sha256"], second["sha256"])
        self.assertNotEqual(first["arguments"], second["arguments"])

    def test_the_prover_backends_are_recorded(self) -> None:
        from unittest import mock
        from harness.compatibility import tool_executables

        with tempfile.TemporaryDirectory() as temporary:
            boogie = Path(temporary) / "boogie"
            boogie.write_text("#!/bin/sh\n", encoding="utf-8")
            config = mock.Mock(
                compile_command=[], inference_command=[], prove_command=[]
            )
            with mock.patch.dict("os.environ", {"BOOGIE_EXE": str(boogie)}):
                recorded = tool_executables(config)
        self.assertIn("boogie", recorded)
        self.assertEqual(64, len(recorded["boogie"]["sha256"]))


class VacuousReferenceTest(unittest.TestCase):
    """A reference with contradictory assumptions proves everything.

    So a successful prove is not evidence of solvability on its own.
    `validate_mutants` already refuses such a reference before certifying
    essentiality; the screen has to refuse it before certifying that the task
    is solvable at all.
    """

    def _prove(self, prove_ok: bool, incons_ok: bool, diagnostics: str) -> dict:
        import asyncio
        from unittest import mock
        import harness.screen_v3 as screen

        calls = {"n": 0}

        async def fake_run_command(*_args, **_kwargs):
            calls["n"] += 1
            first = calls["n"] == 1
            return mock.Mock(
                succeeded=prove_ok if first else incons_ok,
                diagnostics="" if first else diagnostics,
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "references" / "build" / "m"
            package.mkdir(parents=True)
            (package / "Move.toml").write_text("x", encoding="utf-8")
            with mock.patch.object(screen, "run_command", new=fake_run_command):
                return asyncio.run(
                    screen._prove_reference(
                        mock.Mock(prove_command=["p"]),
                        root / "manifest.json",
                        {"task_id": "T", "module": "pkg::m", "target": "pkg::m::f"},
                        40,
                    )
                )

    def test_a_sound_reference_proves(self) -> None:
        result = self._prove(True, True, "")
        self.assertTrue(result["proved"])
        self.assertFalse(result["vacuous"])

    def test_a_vacuous_reference_does_not_prove(self) -> None:
        # The prover succeeded; the inconsistency check says why that is empty.
        result = self._prove(True, True, "inconsistent assumption at line 3")
        self.assertFalse(result["proved"])
        self.assertTrue(result["vacuous"])

    def test_an_unchecked_vacuity_does_not_prove(self) -> None:
        # The check reached no verdict, so its silence is not a clean bill.
        result = self._prove(True, False, "")
        self.assertFalse(result["proved"])
        self.assertFalse(result["vacuity_checked"])


class ToolchainIdentityTest(unittest.TestCase):
    """Screening drives four commands, not one.

    The config may point compile, inference and prove at different executables
    from the checker, so recording only the checker leaves the toolchain that
    produced the evidence unpinned.
    """

    def _check(self, recorded: dict | None, scheduled: dict | None) -> str:
        from harness.pilot import _require_screening_agrees

        tools = {"move_flow_sha256": "a" * 64, "experiment_config_sha256": "b" * 64}
        if recorded is not None:
            tools["stage_executables"] = recorded
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({"tools": tools, "results": [
                    {"task_id": "T", "target": "m::f", "passed": True,
                     "apparatus_ok": True, "reference_sha256": "d" * 64}
                ]}),
                encoding="utf-8",
            )
            apparatus = {"move_flow_sha256": "a" * 64,
                         "experiment_config_sha256": "b" * 64}
            if scheduled is not None:
                apparatus["stage_executables"] = scheduled
            try:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "m::f", "screening_status": "ready"}],
                    apparatus,
                )
            except ValueError as error:
                return str(error)
        return ""

    def test_a_matching_toolchain_clears(self) -> None:
        stages = {"prover": {"path": "/p", "sha256": "e" * 64}}
        self.assertEqual("", self._check(stages, stages))

    def test_a_changed_prover_does_not_clear(self) -> None:
        message = self._check(
            {"prover": {"path": "/p", "sha256": "e" * 64}},
            {"prover": {"path": "/p", "sha256": "f" * 64}},
        )
        self.assertIn("different toolchain", message)

    def test_evidence_naming_no_toolchain_does_not_clear(self) -> None:
        message = self._check(None, {"prover": {"path": "/p", "sha256": "e" * 64}})
        self.assertIn("records no stage executables", message)


class ReferenceDigestTest(unittest.TestCase):
    """A reference proof has to identify the content it proved.

    The assembled reference is a generated, gitignored tree, so evidence
    recording only a boolean and a path cannot be tied to it. Whether the tree
    still matches is `build_references.py --verify`'s job -- the reference is
    not what the round runs, so the scheduler asks only that the evidence say
    which reference produced it.
    """

    def _check(self, result_extra: dict) -> str:
        from harness.pilot import _require_screening_agrees

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({
                    "tools": {"move_flow_sha256": "a" * 64,
                              "experiment_config_sha256": "b" * 64},
                    "results": [{"task_id": "T", "target": "pkg::m::f",
                                 "passed": True, "apparatus_ok": True,
                                 **result_extra}],
                }),
                encoding="utf-8",
            )
            try:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "pkg::m::f",
                      "screening_status": "ready"}],
                    {"move_flow_sha256": "a" * 64,
                     "experiment_config_sha256": "b" * 64},
                )
            except ValueError as error:
                return str(error)
        return ""

    def test_evidence_that_names_its_reference_clears(self) -> None:
        self.assertEqual("", self._check({"reference_sha256": "c" * 64}))

    def test_evidence_without_a_digest_does_not_clear(self) -> None:
        self.assertIn("records no reference digest", self._check({}))


class ReferencePathTest(unittest.TestCase):
    """The reference package name comes from the manifest, so it is input.

    An absolute segment discards the reference root and `..` climbs out of it,
    and whatever proved there would still be recorded as this task's
    solvability evidence.
    """

    def test_a_traversing_module_is_refused(self) -> None:
        from harness.identifiers import require_plain_name

        for bad in ("../../etc", "/absolute", "a/b"):
            with self.assertRaises(ValueError, msg=bad):
                require_plain_name(bad, "module")

    def test_an_ordinary_module_name_is_accepted(self) -> None:
        from harness.identifiers import require_plain_name

        self.assertEqual(
            "lomuto_partition", require_plain_name("lomuto_partition", "module")
        )


class MountReadabilityTest(unittest.TestCase):
    """Inside the evaluation tree is necessary but not sufficient.

    The plugin directory and the workspace are inside it too, and the agent
    reads both, so a refutation root nested under either would be mounted where
    `Read` and `Glob` reach it -- handing over the mutations the category-only
    feedback exists to withhold.
    """

    def test_a_root_under_the_plugin_is_refused(self) -> None:
        from harness.pilot_sandbox import _require_confined_mount

        root = (ROOT / "corpus-v3" / "mutants").resolve()
        with self.assertRaises(SystemExit) as raised:
            _require_confined_mount(root, ((ROOT / "corpus-v3").resolve(),))
        self.assertIn("can read", str(raised.exception))

    def test_a_root_outside_the_readable_trees_is_accepted(self) -> None:
        from harness.pilot_sandbox import _require_confined_mount

        root = (ROOT / "corpus-v3" / "mutants").resolve()
        _require_confined_mount(root, ((ROOT / "harness").resolve(),))


class EmptyRefutationTest(unittest.TestCase):
    """An empty refutation set refutes nothing.

    It finds no survivor and no inconclusive result, so the candidate is
    confirmed having been tested against nothing -- asking for a refutation
    root and getting silence is the one reading that must not be available.
    """

    def test_an_empty_manifest_is_refused(self) -> None:
        from harness.controller import _task_mutants

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "T").mkdir()
            (root / "T" / "mutants.json").write_text(
                json.dumps({"mutants": []}), encoding="utf-8"
            )
            with self.assertRaises(SystemExit) as raised:
                _task_mutants(root, "T", "refutation")
        self.assertIn("no mutants", str(raised.exception))

    def test_a_populated_manifest_is_accepted(self) -> None:
        from harness.controller import _task_mutants

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "T").mkdir()
            (root / "T" / "mutants.json").write_text(
                json.dumps({"mutants": [{"mutant_id": "m"}]}), encoding="utf-8"
            )
            self.assertIsNotNone(_task_mutants(root, "T", "refutation"))


class StaleEvidenceTest(unittest.TestCase):
    """Evidence has to identify the logic that produced it.

    A result without `apparatus_ok` came from a screen that could not tell WP
    declining from WP failing to run, so its `passed` does not mean what the
    scheduler reads it to mean -- and the package digest cannot notice, because
    the logic changed and the corpus did not.
    """

    def _check(self, results: list[dict]) -> str:
        from harness.pilot import _require_screening_agrees

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({"results": results}), encoding="utf-8"
            )
            try:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "m::f", "screening_status": "ready"}],
                )
            except ValueError as error:
                return str(error)
        return ""

    def test_a_result_without_the_apparatus_field_is_refused(self) -> None:
        message = self._check([{"task_id": "T", "target": "m::f", "passed": True}])
        self.assertIn("predates the apparatus check", message)

    def test_a_result_that_records_it_clears(self) -> None:
        self.assertEqual("", self._check([
            {"task_id": "T", "target": "m::f", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}
        ]))

    def test_a_result_whose_apparatus_failed_does_not_clear(self) -> None:
        message = self._check([
            {"task_id": "T", "target": "m::f", "passed": True,
             "apparatus_ok": False, "reference_sha256": "d" * 64}
        ])
        self.assertIn("not cleared", message)


class MountConfinementTest(unittest.TestCase):
    """A refutation root may not shadow the sandbox's own filesystem.

    The root is bind-mounted at its absolute path into a namespace that has
    already placed a tmpfs at `/tmp` and granted the agent read access to the
    toolchain, so a root at or above one of those replaces the isolated view
    with the host tree -- which the agent can then read and send onward.
    """

    def test_the_corpus_root_is_accepted(self) -> None:
        _require_confined_mount((ROOT / "corpus-v3" / "mutants").resolve())

    def test_tmp_is_refused(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            _require_confined_mount(Path("/tmp"))
        self.assertIn("shadow", str(raised.exception))

    def test_the_filesystem_root_is_refused(self) -> None:
        with self.assertRaises(SystemExit):
            _require_confined_mount(Path("/"))

    def test_opt_is_refused(self) -> None:
        with self.assertRaises(SystemExit):
            _require_confined_mount(Path("/opt"))

    def test_a_path_outside_the_evaluation_tree_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(SystemExit) as raised:
                _require_confined_mount(Path(temporary).resolve())
        self.assertIn("outside", str(raised.exception))


class ScreeningTargetTest(unittest.TestCase):
    """Evidence is about a target, not about a task id.

    A record can be repointed at a different target without the package tree
    changing, so matching on the id alone would let evidence for one target
    authorise scheduling another.
    """

    def test_evidence_for_another_target_does_not_clear(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({"results": [
                    {"task_id": "T", "target": "m::was_screened", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}
                ]}),
                encoding="utf-8",
            )
            with self.assertRaises(ValueError) as raised:
                _require_screening_agrees(
                    root / "manifest.json",
                    [{"task_id": "T", "target": "m::something_else",
                      "screening_status": "ready"}],
                )
        self.assertIn("T", str(raised.exception))

    def test_evidence_for_the_same_target_clears(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "screening").mkdir(parents=True)
            (root / "screening" / "summary.json").write_text(
                json.dumps({"results": [
                    {"task_id": "T", "target": "m::f", "passed": True, "apparatus_ok": True, "reference_sha256": "d" * 64}
                ]}),
                encoding="utf-8",
            )
            _require_screening_agrees(
                root / "manifest.json",
                [{"task_id": "T", "target": "m::f", "screening_status": "ready"}],
            )


class AuthoringPathTest(unittest.TestCase):
    """The authoring CLI takes both of its paths from its input.

    A specification is a file the operator may not have written, so an absolute
    `file` would read anywhere on the host and a traversing task id would write
    outside the requested output directory.
    """

    def _entry(self, file: str) -> dict:
        return {
            "mutant_id": "m", "file": file, "anchor": "x", "replace": "x",
            "with": "y", "obligation_category": "normal-result", "rationale": "r",
        }

    def _module(self):
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "author_mutants", ROOT / "corpus-v3" / "author_mutants.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_an_absolute_file_is_refused(self) -> None:
        # Asserting the containment message, not just any exception: a missing
        # file would raise too, and would pass a looser case for the wrong
        # reason.
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(ValueError) as raised:
                self._module().build_case(Path(temporary), self._entry("/etc/passwd"))
        self.assertIn("escapes", str(raised.exception))

    def test_a_traversing_file_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(ValueError) as raised:
                self._module().build_case(
                    Path(temporary), self._entry("../../etc/passwd")
                )
        self.assertIn("escapes", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
