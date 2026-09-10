import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness.artifacts import tree_hash
from harness.compatibility import COMPATIBILITY_SCHEMA_VERSION, admission
from harness.screen import (
    _load_ledger,
    _resume_result,
    _screen_from_ledger,
    reference_targets,
)


class ScreeningLedgerTests(unittest.TestCase):
    def test_reused_compatibility_evidence_still_proves_the_reference(self) -> None:
        record = {
            "package_module_target": "0x1::m::f",
            "granularity": "function",
        }
        ledger = {"passed": True, "reason": None}
        proof = {"proved": False, "vacuity_checked": True}
        with mock.patch(
            "harness.screen.prove_reference", mock.AsyncMock(return_value=proof)
        ) as prove:
            result, apparatus_ok = asyncio.run(
                _screen_from_ledger(
                    mock.MagicMock(), Path("/shared"), record, 40, ledger
                )
            )

        prove.assert_awaited_once()
        self.assertFalse(result["passed"])
        self.assertEqual("reference_unproved", result["reason"])
        self.assertTrue(apparatus_ok)

    def test_ignores_entries_from_a_different_tool_build(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "ledger.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "source_commit": "a" * 40,
                        "entries": [
                            {
                                "task_id": "task",
                                "threshold_seconds": 40,
                                "tool_executables": {
                                    "compile": {
                                        "path": "/old/move-flow",
                                        "sha256": "old",
                                    }
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            entries = _load_ledger(
                path,
                "a" * 40,
                40,
                {
                    "compile": {
                        "path": "/new/move-flow",
                        "sha256": "new",
                    }
                },
            )

        self.assertEqual(entries, {})


class ResumeTests(unittest.TestCase):
    def test_a_result_of_the_current_schema_is_resumed(self) -> None:
        # The resumed result's identity is checked against the schema the
        # compatibility check writes now, not a number frozen in this module.
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "package"
            (package / "sources").mkdir(parents=True)
            (package / "sources" / "m.move").write_text("module 0x42::m {}\n", encoding="utf-8")
            record = {
                "prepared_sha256": tree_hash(package),
                "package_module_target": "0x42::m::f",
            }
            path = Path(temporary) / "result.json"
            result = {
                "schema_version": COMPATIBILITY_SCHEMA_VERSION,
                "package_sha256": record["prepared_sha256"],
                "target": "0x42::m::f",
                "threshold_seconds": 40,
                "passed": True,
            }
            path.write_text(json.dumps(result), encoding="utf-8")
            tools = {"prover": {"path": "/bin/move-flow", "sha256": "a" * 64}}
            result["tool_executables"] = tools
            path.write_text(json.dumps(result), encoding="utf-8")
            self.assertEqual(result, _resume_result(path, package, record, 40, tools))

            # A result the current toolchain did not produce is not resumable:
            # resuming would publish this build's name over another's verdicts.
            rebuilt = {"prover": {"path": "/bin/move-flow", "sha256": "b" * 64}}
            with self.assertRaises(ValueError) as raised:
                _resume_result(path, package, record, 40, rebuilt)
            self.assertIn("different toolchain", str(raised.exception))

            result["schema_version"] = COMPATIBILITY_SCHEMA_VERSION - 1
            path.write_text(json.dumps(result), encoding="utf-8")
            with self.assertRaises(ValueError):
                _resume_result(path, package, record, 40, tools)


if __name__ == "__main__":
    unittest.main()


class AdmissionTests(unittest.TestCase):
    """A target is a member when it is well-formed and its reference proves;
    WP falling short is recorded, not disqualifying."""

    @staticmethod
    def _result(passed: bool, failure_kind: str | None, proved: bool, **stages):
        base = {
            "compile": {"returncode": 0, "timed_out": False, "infrastructure_error": None},
            "wp_inference": {"returncode": 0, "timed_out": False, "infrastructure_error": None},
            "enriched_compile": {"returncode": 0, "timed_out": False, "infrastructure_error": None},
            "prover": {"returncode": 0, "timed_out": False, "infrastructure_error": None},
        }
        base.update(stages)
        return {
            **base,
            "passed": passed,
            "failure_kind": failure_kind,
            "reference_proof": {"proved": proved},
        }

    def test_wp_solving_it_unaided_is_not_hard(self) -> None:
        verdict = admission(self._result(True, None, True))
        self.assertEqual((verdict["passed"], verdict["wp_hard"], verdict["reason"]), (True, False, None))

    def test_an_untrusted_inference_with_a_proving_reference_is_admitted_as_hard(self) -> None:
        verdict = admission(self._result(False, "untrusted_inferred_contract", True))
        self.assertTrue(verdict["passed"])
        self.assertTrue(verdict["wp_hard"])
        self.assertEqual(verdict["wp_failure_kind"], "untrusted_inferred_contract")

    def test_a_declined_inference_with_a_proving_reference_is_admitted(self) -> None:
        # WP refused (uninvariant loop): nothing to recompile or prove.
        verdict = admission(
            self._result(
                False,
                "implementation_failure",
                True,
                wp_inference={
                    "returncode": 1,
                    "timed_out": False,
                    "infrastructure_error": None,
                    "stage_report": {"diagnostics": ["error: WP inferred `vacuous`"]},
                },
                enriched_compile=None,
                prover=None,
            )
        )
        self.assertTrue(verdict["passed"])
        self.assertTrue(verdict["well_formed"])

    def test_a_reference_that_does_not_prove_excludes(self) -> None:
        verdict = admission(self._result(False, "untrusted_inferred_contract", False))
        self.assertFalse(verdict["passed"])
        self.assertEqual(verdict["reason"], "reference_unproved")

    def test_a_target_that_does_not_compile_is_not_well_formed(self) -> None:
        verdict = admission(
            self._result(
                False,
                "implementation_failure",
                True,
                compile={"returncode": 1, "timed_out": False, "infrastructure_error": None},
                wp_inference=None,
                enriched_compile=None,
                prover=None,
            )
        )
        self.assertFalse(verdict["passed"])
        self.assertEqual(verdict["reason"], "implementation_failure")


class ReferenceTargetTests(unittest.TestCase):
    def test_a_module_task_proves_the_same_module_the_round_runs(self) -> None:
        record = {
            "package_module_target": "0x1::aptos_coin",
            "granularity": "module",
            "target_functions": ["initialize", "find_delegation"],
        }
        self.assertEqual(
            reference_targets(record),
            ["0x1::aptos_coin"],
        )

    def test_a_function_task_proves_itself(self) -> None:
        record = {
            "package_module_target": "0x1::stake::append",
            "granularity": "function",
            "target_functions": ["append"],
        }
        self.assertEqual(reference_targets(record), ["0x1::stake::append"])
