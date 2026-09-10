from __future__ import annotations

import importlib.util
import itertools
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "round_summary", ROOT / "analysis" / "round_summary.py"
)
round_summary = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(round_summary)


class MutationReportingTest(unittest.TestCase):
    """`not killed` is three different things, and only one is a survivor.

    A survivor says the contract verified against code it should have
    rejected -- a claim about the specification. A timeout or an
    infrastructure failure says nothing at all, and reporting it as a survivor
    asserts a precision failure that was never observed.
    """

    def _summary(self, results: list[dict]) -> dict:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary)
            (artifact / "mutation-score.json").write_text(
                json.dumps(
                    {
                        "essential_mutants": len(results),
                        "killed": sum(1 for r in results if r["killed"]),
                        "mutation_adequacy": 0.5,
                        "results": results,
                    }
                ),
                encoding="utf-8",
            )
            (artifact / "run.json").write_text(
                json.dumps(
                    {
                        "run_id": "r",
                        "task_id": "t",
                        "arm": "agent_only",
                        "target": "0x1::m::f",
                        "result": {"terminal_status": "operational_success"},
                    }
                ),
                encoding="utf-8",
            )
            return round_summary.collect_run(artifact)["mutation"]

    def test_only_a_real_survivor_is_listed_as_one(self) -> None:
        mutation = self._summary(
            [
                {"mutant_id": "m-killed", "killed": True, "outcome": "killed"},
                {"mutant_id": "m-survived", "killed": False, "outcome": "survived"},
                {"mutant_id": "m-timeout", "killed": False, "outcome": "prover_timeout"},
                {
                    "mutant_id": "m-infra",
                    "killed": False,
                    "outcome": "infrastructure_failure",
                },
            ]
        )
        self.assertEqual(["m-survived"], mutation["survived"])
        self.assertEqual(
            {"m-timeout": "prover_timeout", "m-infra": "infrastructure_failure"},
            mutation["inconclusive"],
        )

    def test_extra_lives_are_counted(self) -> None:
        # The number the mechanism exists to justify: candidates the checker
        # accepted and refutation sent back. Each would otherwise have ended
        # its cell as a success.
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary)
            (artifact / "run.json").write_text(
                json.dumps({"run_id": "r", "task_id": "t", "arm": "agent_only",
                            "target": "0x1::m::f", "result": {}}), encoding="utf-8")
            (artifact / "controller-events.jsonl").write_text("\n".join(
                json.dumps(e) for e in (
                    {"event": "refutation", "controller_turn": 1, "killed": 3,
                     "total": 4, "survived": ["m-lost"]},
                    {"event": "refutation", "controller_turn": 2, "killed": 4,
                     "total": 4, "survived": []},
                )) + "\n", encoding="utf-8")
            summary = round_summary.collect_run(artifact)["refutation"]
        self.assertEqual(2, summary["runs"])
        self.assertEqual(1, summary["downgrades"])
        self.assertEqual(["3/4", "4/4"], summary["killed_by_turn"])
        self.assertTrue(summary["converged"])

    def test_a_round_without_refutation_reports_none(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary)
            (artifact / "run.json").write_text(
                json.dumps({"run_id": "r", "task_id": "t", "arm": "agent_only",
                            "target": "0x1::m::f", "result": {}}), encoding="utf-8")
            self.assertIsNone(round_summary.collect_run(artifact)["refutation"])

    def test_a_clean_round_reports_no_inconclusive_mutants(self) -> None:
        mutation = self._summary(
            [{"mutant_id": "m", "killed": True, "outcome": "killed"}]
        )
        self.assertEqual([], mutation["survived"])
        self.assertEqual({}, mutation["inconclusive"])


class RefutationConvergenceTest(unittest.TestCase):
    """Convergence means every mutant was rejected, not that none survived.

    `run_mutant_cases` reports outcomes that are neither `killed` nor
    `survived`. Reading convergence from an empty survivor list counts a final
    refutation that reached no verdict as a contract that rejected everything.
    """

    def _converged(self, last_event: dict) -> bool:
        """Through `collect_run`, not through a copy of its rule.

        This helper used to restate the predicate it was checking, so it agreed
        with the summary by construction and could not have caught the summary
        drifting from the controller -- which is the very thing that happened.
        """
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary)
            (artifact / "controller-events.jsonl").write_text(
                json.dumps({"event": "refutation", **last_event}) + "\n",
                encoding="utf-8",
            )
            (artifact / "run.json").write_text(
                json.dumps({
                    "run_id": "r", "task_id": "t", "arm": "agent_only",
                    "target": "0x1::m::f",
                    "result": {"terminal_status": "operational_success"},
                }),
                encoding="utf-8",
            )
            return round_summary.collect_run(artifact)["refutation"]["converged"]

    def test_all_killed_converges(self) -> None:
        self.assertTrue(self._converged({"killed": 3, "total": 3, "survived": [], "inconclusive": []}))

    def test_a_survivor_does_not_converge(self) -> None:
        self.assertFalse(self._converged({"killed": 2, "total": 3, "survived": ["m"], "inconclusive": []}))

    def test_an_inconclusive_mutant_does_not_converge(self) -> None:
        self.assertFalse(self._converged({"killed": 2, "total": 3, "survived": [], "inconclusive": ["m"]}))

    def test_a_pass_that_overran_the_budget_has_not_converged(self) -> None:
        # The controller returns `infrastructure_failure` for exactly this
        # event, so reporting it as converged credits a cell the round refused.
        self.assertFalse(self._converged({
            "killed": 3, "total": 3, "survived": [], "inconclusive": [],
            "overran_budget": True,
        }))

    def test_a_round_recorded_before_the_field_existed_still_reads(self) -> None:
        # An older event carries no `overran_budget`; absent is not overrun.
        self.assertTrue(self._converged({
            "killed": 3, "total": 3, "survived": [], "inconclusive": [],
        }))

    def test_the_summary_agrees_with_the_controller_on_every_case(self) -> None:
        """The invariant, checked by behaviour rather than by grepping source.

        This assertion used to look for the predicate's text inside
        `round_summary.py`. That passes as long as the words are present, which
        says nothing about whether the summary decides what the controller
        decided -- and the summary had in fact grown a third condition behind
        the controller's back. Comparing outcomes catches that; comparing
        source did not.
        """
        from harness.mutants import refutation_confirms

        for survived, inconclusive, overran in itertools.product(
            ([], ["m"]), ([], ["n"]), (False, True)
        ):
            event = {
                "killed": 1, "total": 2,
                "survived": survived, "inconclusive": inconclusive,
                "overran_budget": overran,
            }
            with self.subTest(survived=survived, inconclusive=inconclusive, overran=overran):
                self.assertEqual(
                    refutation_confirms(survived, inconclusive, overran),
                    self._converged(event),
                )

if __name__ == "__main__":
    unittest.main()
