from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness.artifacts import sha256_file
from harness.mutants import require_unique_mutant_ids
from harness.score_round import (
    PendingScore,
    _disqualification_manifest,
    _score_pending,
    score_round,
)


class UnscorableRunTest(unittest.TestCase):
    """One cell that cannot be scored must not take the round with it.

    Scoring runs after the round, so the alternative to recording a failure is
    discarding sessions that already cost their full budget -- an apparatus
    failure reported as an absence of results.
    """

    def _run(self, error: BaseException) -> list[dict]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entries = [{"run_id": "bad"}, {"run_id": "good"}]
            pending = [
                PendingScore(entries[0], root / "bad", root / "base", "0x1::m::f", root / "m.json", 10),
                PendingScore(entries[1], root / "good", root / "base", "0x1::m::f", root / "m.json", 10),
            ]

            async def score(config, candidate, baseline, target, manifest, timeout):
                if "bad" in str(candidate):
                    raise error
                return {"mutation_adequacy": 1.0, "killed": 3, "essential_mutants": 3}

            with mock.patch("harness.score_round.score_mutants", side_effect=score), \
                 mock.patch("harness.score_round.write_json"):
                asyncio.run(_score_pending(mock.MagicMock(), pending, 1))
            return entries

    def test_an_unreadable_workspace_is_recorded_not_raised(self) -> None:
        for error in (
            ValueError("proof does not reproduce"),
            FileNotFoundError("workspace is gone"),
            OSError("input/output error"),
        ):
            with self.subTest(error=type(error).__name__):
                bad, good = self._run(error)
                self.assertEqual("not_scorable", bad["outcome"])
                self.assertIn(type(error).__name__, bad["detail"])
                # The point of the fix: the other cell still has its score.
                self.assertEqual("scored", good["outcome"])
                self.assertTrue(good["strict_success"])


class DisqualificationGateTest(unittest.TestCase):
    """A withheld set refutes a contract instead of teaching it.

    corpus-v1.1 does not show its refutation set during a run, so a contract gets
    no second attempt at the counterexamples in it. The set is applied here
    instead, and a mutation that survives voids the run rather than costing it
    a fraction of a mutation score.
    """

    def _run(self, gate: dict) -> dict:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry: dict = {"run_id": "r"}
            pending = [
                PendingScore(
                    entry, root / "final", root / "base", "0x1::m::f",
                    root / "scoring.json", 10, root / "gate.json",
                )
            ]

            async def score(config, candidate, baseline, target, manifest, timeout):
                if manifest.name == "gate.json":
                    return {"mutant_manifest_sha256": "g" * 64, **gate}
                return {
                    "mutation_adequacy": 1.0, "killed": 3, "essential_mutants": 3,
                    "inconclusive": [], "mutant_manifest_sha256": "s" * 64,
                }

            with mock.patch("harness.score_round.score_mutants", side_effect=score), \
                 mock.patch("harness.score_round.write_json"):
                asyncio.run(_score_pending(mock.MagicMock(), pending, 1))
            return entry

    @staticmethod
    def _results(*killed: bool) -> list[dict]:
        return [
            {"mutant_id": f"m{index}", "killed": value}
            for index, value in enumerate(killed)
        ]

    def test_a_surviving_mutation_disqualifies_the_run(self) -> None:
        entry = self._run({"results": self._results(True, False, True), "inconclusive": []})
        self.assertEqual("disqualified", entry["outcome"])
        self.assertEqual(["m1"], entry["disqualified_by"])
        self.assertFalse(entry["strict_success"])
        # A refuted contract has no mutation score to report.
        self.assertNotIn("mutation_adequacy", entry)

    def test_a_contract_that_kills_the_gate_is_scored(self) -> None:
        entry = self._run({"results": self._results(True, True, True), "inconclusive": []})
        self.assertEqual("scored", entry["outcome"])
        self.assertTrue(entry["strict_success"])
        self.assertEqual("g" * 64, entry["disqualification_manifest_sha256"])

    def test_an_inconclusive_gate_blocks_ordinary_scoring(self) -> None:
        # It is not a counterexample; it is a measurement that did not happen,
        # and recording it keeps a gate that measured nothing from reading as
        # one the contract passed.
        entry = self._run(
            {"results": self._results(True, False, True), "inconclusive": ["m1"]}
        )
        self.assertEqual("not_scorable", entry["outcome"])
        self.assertFalse(entry["strict_success"])
        self.assertEqual(["m1"], entry["disqualification_inconclusive"])

    def test_a_gate_must_be_bound_when_the_round_is_scheduled(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            gate = root / "gate/T/mutants.json"
            scored = root / "scored.json"
            gate.parent.mkdir(parents=True)
            gate.write_text(json.dumps({"mutants": []}), encoding="utf-8")
            scored.write_text(json.dumps({"mutants": []}), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "not scheduled"):
                _disqualification_manifest(
                    root / "gate", "T", "run-1", scored, root, [], None
                )

    def test_a_bound_gate_cannot_be_omitted_when_scoring(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            run = root / "runs" / "r1"
            mutants = root / "mutants" / "T"
            run.mkdir(parents=True)
            mutants.mkdir(parents=True)
            manifest = mutants / "mutants.json"
            manifest.write_text(json.dumps({"mutants": []}), encoding="utf-8")
            (run / "run.json").write_text(
                json.dumps(
                    {
                        "run_id": "r1",
                        "task_id": "T",
                        "target": "m::f",
                        "package_relpath": "pkg",
                        "mutant_manifest_sha256": sha256_file(manifest),
                        "disqualification_mutant_manifest_sha256": "d" * 64,
                        "result": {
                            "eventual_judge": {"state": "operational_success"}
                        },
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                ValueError, "--disqualification-mutants-root was not provided"
            ):
                asyncio.run(
                    score_round(
                        config=mock.MagicMock(),
                        round_dir=root,
                        mutants_root=root / "mutants",
                        timeout_seconds=1,
                    )
                )

    def test_a_gate_cannot_repeat_mutant_ids(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            gate = root / "gate/T/mutants.json"
            scored = root / "scored.json"
            gate.parent.mkdir(parents=True)
            gate.write_text(
                json.dumps(
                    {"mutants": [{"mutant_id": "same"}, {"mutant_id": "same"}]}
                ),
                encoding="utf-8",
            )
            scored.write_text(json.dumps({"mutants": []}), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "repeats mutant id"):
                _disqualification_manifest(
                    root / "gate",
                    "T",
                    "run-1",
                    scored,
                    root,
                    [],
                    sha256_file(gate),
                )

    def test_distinct_schema_mutant_ids_are_accepted(self) -> None:
        require_unique_mutant_ids(
            [{"mutant_id": "first"}, {"mutant_id": "second"}], "set"
        )


if __name__ == "__main__":
    unittest.main()
