from __future__ import annotations

import importlib.util
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
        refutations = [last_event]
        return (
            bool(refutations)
            and not refutations[-1].get("survived")
            and not refutations[-1].get("inconclusive")
        )

    def test_all_killed_converges(self) -> None:
        self.assertTrue(self._converged({"killed": 3, "total": 3, "survived": [], "inconclusive": []}))

    def test_a_survivor_does_not_converge(self) -> None:
        self.assertFalse(self._converged({"killed": 2, "total": 3, "survived": ["m"], "inconclusive": []}))

    def test_an_inconclusive_mutant_does_not_converge(self) -> None:
        self.assertFalse(self._converged({"killed": 2, "total": 3, "survived": [], "inconclusive": ["m"]}))

    def test_the_summary_reports_the_same_rule(self) -> None:
        source = (Path(__file__).resolve().parent.parent / "analysis" / "round_summary.py").read_text(encoding="utf-8")
        self.assertIn('not refutations[-1].get("inconclusive")', source)

if __name__ == "__main__":
    unittest.main()
