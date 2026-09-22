from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.taxonomy import (
    PROPOSED_CATEGORIES,
    _policy_violations,
    build_report,
    render_markdown,
)


class PolicyViolationTest(unittest.TestCase):
    def test_violations_are_counted_from_both_judgements(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            runs = Path(temporary)
            run = runs / "run-a"
            run.mkdir()
            (run / "judge.json").write_text(
                json.dumps(
                    {
                        "final_judge": {
                            "verdict": {
                                "policy": {
                                    "violations": [{"code": "vacuous_ensures"}],
                                    "contract_coverage": {
                                        "violations": [{"code": "missing_contract_category"}]
                                    },
                                }
                            }
                        },
                        "eventual_judge": {
                            "verdict": {
                                "policy": {
                                    "violations": [{"code": "vacuous_ensures"}],
                                    "contract_coverage": {"violations": []},
                                }
                            }
                        },
                    }
                ),
                encoding="utf-8",
            )

            counts = _policy_violations(runs)

        self.assertEqual(2, counts["vacuous_ensures"])
        self.assertEqual(1, counts["missing_contract_category"])

    def test_a_round_without_violations_reports_none(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            self.assertEqual({}, dict(_policy_violations(Path(temporary))))


class ReportShapeTest(unittest.TestCase):
    def test_every_proposed_category_appears_with_a_verdict(self) -> None:
        report = {
            "runs": 2,
            "terminal_statuses": {"operational_success": 2},
            "failure_kinds": {"postcondition": 3},
            "policy_violations": {},
            "proposed_categories": [
                {
                    "category": name,
                    "observations": 3 if name == "incorrect normal-return behavior" else 0,
                    "evidence": ["postcondition"]
                    if name == "incorrect normal-return behavior"
                    else [],
                    "status": "observed"
                    if name == "incorrect normal-return behavior"
                    else "unobserved",
                    "note": "",
                }
                for name, _, _ in PROPOSED_CATEGORIES
            ],
            "unproposed_kinds": ["syntax_error"],
            "adopted": ["incorrect normal-return behavior", "syntax_error"],
        }

        text = render_markdown(report)

        for name, _, _ in PROPOSED_CATEGORIES:
            self.assertIn(name, text)
        self.assertIn("unobserved", text)
        self.assertIn("Kinds the design did not propose", text)
        self.assertIn("syntax_error", text)


if __name__ == "__main__":
    unittest.main()


class ReachabilityTest(unittest.TestCase):
    """A silent category means different things in different corpora."""

    @staticmethod
    def _round(root: Path, categories: list[str]) -> Path:
        runs = root / "runs"
        run = runs / "run-a"
        (run / "final").mkdir(parents=True)
        (run / "run.json").write_text(
            json.dumps(
                {
                    "run_id": "run-a",
                    "task_id": "task",
                    "arm": "agent_only",
                    "replicate": 1,
                    "required_contract_categories": categories,
                }
            ),
            encoding="utf-8",
        )
        (run / "judge.json").write_text(
            json.dumps({"terminal_status": "operational_success"}), encoding="utf-8"
        )
        for name in ("controller-events.jsonl", "claude-events.jsonl"):
            (run / name).write_text("", encoding="utf-8")
        return runs

    def _status(self, categories: list[str], category: str) -> str:
        with tempfile.TemporaryDirectory() as temporary:
            runs = self._round(Path(temporary), categories)
            report = build_report(runs)
        return next(
            item["status"]
            for item in report["proposed_categories"]
            if item["category"] == category
        )

    def test_a_corpus_without_frames_cannot_speak_to_frame_failures(self) -> None:
        self.assertEqual(
            "unreachable in this corpus",
            self._status(["normal-result", "abort"], "missing global frame"),
        )

    def test_a_corpus_with_frames_that_never_fail_is_evidence(self) -> None:
        self.assertEqual(
            "reachable but never triggered",
            self._status(["frame", "abort"], "missing global frame"),
        )

    def test_a_category_without_a_reachability_signal_says_so(self) -> None:
        self.assertEqual(
            "reachability unknown",
            self._status(["frame"], "insufficient callee contract"),
        )
