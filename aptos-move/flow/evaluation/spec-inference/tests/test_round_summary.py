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

    def test_a_clean_round_reports_no_inconclusive_mutants(self) -> None:
        mutation = self._summary(
            [{"mutant_id": "m", "killed": True, "outcome": "killed"}]
        )
        self.assertEqual([], mutation["survived"])
        self.assertEqual({}, mutation["inconclusive"])


if __name__ == "__main__":
    unittest.main()
