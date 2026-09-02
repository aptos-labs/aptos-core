from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.mine import (
    ToolCall,
    _failure_kind,
    _out_of_workspace_searches,
    _repair_iterations,
    _reverted_edit_pairs,
    _session_totals,
    analyze_round,
)


def call(name: str, arguments: dict[str, object], failed: bool = False, result: str = "") -> ToolCall:
    return ToolCall(0, 0, name, name + str(id(arguments)), arguments, result, failed)


class FailureKindTest(unittest.TestCase):
    def test_source_snippets_do_not_decide_the_label(self) -> None:
        result = (
            "error: unexpected token\n"
            "   ┌─ sources/a.move:3:9\n"
            "3 │        modifies global<T>(addr);\n"
        )
        self.assertEqual("syntax_error", _failure_kind(result))

    def test_prover_wording_is_used_verbatim(self) -> None:
        self.assertEqual(
            "solver_timeout",
            _failure_kind("error: verification out of resources/timeout (global timeout set to 5s)"),
        )
        self.assertEqual(
            "postcondition", _failure_kind("error: post-condition does not hold")
        )
        self.assertEqual(
            "abort_not_covered",
            _failure_kind("error: abort not covered by any of the `aborts_if` clauses"),
        )

    def test_translator_and_infrastructure_failures_are_separated(self) -> None:
        self.assertEqual(
            "translator_defect",
            _failure_kind(
                "verification failed: [internal] boogie exited with compilation errors:"
            ),
        )
        self.assertEqual(
            "tool_infrastructure",
            _failure_kind('MCP server "move-flow" sent no response or progress for 1800s'),
        )


class TurnClassificationTest(unittest.TestCase):
    def test_only_searches_outside_the_workspace_are_counted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run = Path(temporary)
            workspace = str((run / "workspace").resolve())
            calls = [
                call("Read", {"file_path": workspace + "/sources/a.move"}),
                call("Grep", {"path": "/home/other/move-prover/doc"}),
                call("Glob", {"path": "/usr/share"}),
                call("Edit", {"file_path": "/home/other/x"}),
            ]

            self.assertEqual(2, _out_of_workspace_searches(calls, run))

    def test_an_edit_undone_by_its_inverse_is_a_self_test(self) -> None:
        calls = [
            call("Edit", {"file_path": "a.move", "old_string": "x", "new_string": "y"}),
            call("Edit", {"file_path": "a.move", "old_string": "y", "new_string": "x"}),
            call("Edit", {"file_path": "a.move", "old_string": "p", "new_string": "q"}),
        ]

        self.assertEqual(1, _reverted_edit_pairs(calls))

    def test_repairs_are_edits_made_while_the_last_answer_was_a_failure(self) -> None:
        calls = [
            call("mcp__move-flow__move_package_verify", {}, failed=True),
            call("Edit", {}),
            call("Edit", {}),
            call("mcp__move-flow__move_package_verify", {}),
            call("Edit", {}),
        ]

        self.assertEqual(2, _repair_iterations(calls))


class ArchiveTest(unittest.TestCase):
    ARCHIVE = Path("evaluation-artifacts/pilot-smoke-005/runs")

    def test_pilot_007_archive_reconciles_with_its_recorded_totals(self) -> None:
        if not self.ARCHIVE.is_dir():
            self.skipTest("pilot-007 archive is not present")
        report = analyze_round(self.ARCHIVE)
        self.assertEqual(9, report["runs"])
        by_run = {item["run_id"]: item for item in report["per_run"]}
        guided_double = by_run["pilot-007-pilot-double-r01-hybrid-guided"]
        # Reconciled against the values the controller recorded for this run.
        self.assertEqual(65, guided_double["model_turns"])
        self.assertEqual(62041, guided_double["output_tokens"])
        self.assertEqual(2884096, guided_double["cache_read_tokens"])
        self.assertNotIn("unclassified", report["failure_kinds"])
        for item in report["per_run"]:
            self.assertEqual("operational_success", item["terminal_status"])


if __name__ == "__main__":
    unittest.main()


class TimeoutKindTest(unittest.TestCase):
    """A killed process and an exhausted condition budget are different faults."""

    def test_a_condition_budget_is_a_solver_timeout(self) -> None:
        self.assertEqual(
            "solver_timeout",
            _failure_kind(
                "error: verification out of resources/timeout (global timeout set to 5s)"
            ),
        )

    def test_a_killed_process_is_reported_separately(self) -> None:
        self.assertEqual(
            "prover_process_timeout",
            _failure_kind("error: Boogie execution exceeded hard timeout of 70s"),
        )


class SessionTotalTest(unittest.TestCase):
    """Some telemetry fields are session totals, not per-turn values."""

    @staticmethod
    def _turn(api_ms: int, cost: float, out: int, cache: int) -> dict[str, object]:
        return {
            "event": "agent_result",
            "result": {
                "duration_api_ms": api_ms,
                "num_turns": 10,
                "usage": {
                    "input_tokens": 1,
                    "output_tokens": out,
                    "cache_read_input_tokens": cache,
                    "cache_creation_input_tokens": 0,
                },
                "model_usage": {"m": {"costUSD": cost}},
            },
        }

    def test_cumulative_fields_are_not_summed(self) -> None:
        controller = [
            self._turn(1_010_150, 2.264, 44_649, 1_715_904),
            self._turn(1_784_726, 4.314, 34_065, 2_017_024),
            self._turn(2_141_698, 5.835, 14_912, 2_118_528),
        ]

        totals, api_ms, turns, cost = _session_totals(controller)

        self.assertEqual(2_141_698, api_ms)
        self.assertAlmostEqual(5.835, cost)
        self.assertEqual(30, turns)
        self.assertEqual(93_626, totals["output_tokens"])
        self.assertEqual(5_851_456, totals["cache_read_input_tokens"])

    def test_a_retry_starts_a_fresh_session_whose_totals_add(self) -> None:
        controller = [
            self._turn(100, 1.0, 10, 20),
            self._turn(250, 2.5, 10, 20),
            {"event": "infrastructure_retry", "attempt": 2},
            self._turn(80, 0.5, 10, 20),
        ]

        _, api_ms, _, cost = _session_totals(controller)

        self.assertEqual(330, api_ms)
        self.assertAlmostEqual(3.0, cost)
