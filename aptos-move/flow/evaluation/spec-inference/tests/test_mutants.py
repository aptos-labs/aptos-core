from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from harness.judge import CommandResult
from harness.mutants import apply_mutant, classify_prover_outcome

SOURCE = """module 0x42::m {
    fun f(v: &vector<u64>, n: u64): u64 {
        let i = 0;
        while (i < n) {
            i += 1;
        };
        i
    }
}
"""


def _case(fragment: str, edit: dict, source: str = SOURCE) -> dict:
    """A mutant anchored at `fragment`, which is never stored in the case."""
    return {
        "mutant_id": "T-001-example",
        "obligation_category": "abort",
        "file": "sources/m.move",
        "anchor": {
            "offset": source.index(fragment),
            "length": len(fragment),
            "sha256": hashlib.sha256(fragment.encode()).hexdigest(),
        },
        "edit": edit,
    }


class ApplyMutantTest(unittest.TestCase):
    def _packages(self, root: Path, candidate: str = SOURCE) -> tuple[Path, Path]:
        baseline, package = root / "baseline", root / "package"
        for tree, text in ((baseline, SOURCE), (package, candidate)):
            (tree / "sources").mkdir(parents=True)
            (tree / "sources" / "m.move").write_text(text, encoding="utf-8")
        return package, baseline

    def test_a_mutant_stores_no_source_yet_rewrites_it(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp))
            case = _case("i < n", {"kind": "substitute", "at": 2, "length": 1, "to": "<="})
            self.assertNotIn("i < n", repr(case), "the case must not quote the source")

            apply_mutant(package, baseline, case)

            self.assertIn("i <= n", (package / "sources/m.move").read_text())

    def test_a_swap_edit_stores_no_text_at_all(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp))
            # reorder `i` and `n` around the ` < ` separator
            case = _case(
                "i < n",
                {"kind": "swap", "at": 0, "a_length": 1, "separator_length": 3, "b_length": 1},
            )

            apply_mutant(package, baseline, case)

            self.assertIn("n < i", (package / "sources/m.move").read_text())

    def test_a_candidate_that_added_specification_is_still_mutated(self) -> None:
        # The agent's file is the implementation plus a spec block, so a plain
        # offset would land in the wrong place; the alignment must find the code.
        candidate = SOURCE.replace(
            "    fun f(", "    spec f {\n        aborts_if false;\n    }\n\n    fun f("
        )
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp), candidate)
            case = _case("i < n", {"kind": "substitute", "at": 2, "length": 1, "to": "<="})

            apply_mutant(package, baseline, case)

            text = (package / "sources/m.move").read_text()
            self.assertIn("i <= n", text)
            self.assertIn("aborts_if false;", text, "the candidate's spec survives")

    def test_a_stale_anchor_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp))
            case = _case("i < n", {"kind": "substitute", "at": 2, "length": 1, "to": "<="})
            case["anchor"]["sha256"] = "0" * 64

            with self.assertRaisesRegex(ValueError, "has changed since it was authored"):
                apply_mutant(package, baseline, case)

    def test_a_candidate_that_changed_the_implementation_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp), SOURCE.replace("i < n", "i < 7"))
            case = _case("i < n", {"kind": "substitute", "at": 2, "length": 1, "to": "<="})

            with self.assertRaisesRegex(ValueError, "not present unchanged"):
                apply_mutant(package, baseline, case)

    def test_an_unknown_edit_kind_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            package, baseline = self._packages(Path(tmp))
            case = _case("i < n", {"kind": "rewrite-everything"})

            with self.assertRaisesRegex(ValueError, "unknown edit kind"):
                apply_mutant(package, baseline, case)


def _result(returncode: int, timed_out: bool = False, error: str | None = None) -> CommandResult:
    return CommandResult(["move-flow"], returncode, 1, timed_out, error, "", "Error: one or more targets did not verify\n" if returncode else "")


def _report(*diagnostics: str, passed: bool = False) -> dict:
    return {"schema_version": 2, "stage": "prover", "passed": passed, "diagnostics": list(diagnostics)}


class ClassifyProverOutcomeTest(unittest.TestCase):
    """The prover reports through its output file, not its streams."""

    def test_a_verification_error_in_the_report_kills(self) -> None:
        report = _report(
            "warning: cannot derive `folds_of` exactly for this lambda argument",
            "error: induction case of the loop invariant does not hold\n   ┌─ m.move:5:9",
            "exiting with verification errors",
        )
        self.assertEqual(classify_prover_outcome(_result(1), report), "killed")

    def test_a_failure_without_a_report_is_unclassified(self) -> None:
        self.assertEqual(classify_prover_outcome(_result(1), None), "unclassified_prover_failure")

    def test_warnings_alone_do_not_kill(self) -> None:
        report = _report("warning: unused alias")
        self.assertEqual(classify_prover_outcome(_result(1), report), "unclassified_prover_failure")

    def test_solver_exhaustion_is_a_timeout_not_a_kill(self) -> None:
        report = _report("error: verification out of resources/timeout (timeout set to 40s)")
        self.assertEqual(classify_prover_outcome(_result(1), report), "prover_timeout")

    def test_an_infrastructure_failure_is_not_a_kill(self) -> None:
        report = _report("error: thread 'main' panicked at src/lib.rs:1")
        self.assertEqual(classify_prover_outcome(_result(1), report), "infrastructure_failure")
        self.assertEqual(
            classify_prover_outcome(_result(None, error="No such file"), None),
            "infrastructure_failure",
        )

    def test_success_and_wall_timeout(self) -> None:
        self.assertEqual(classify_prover_outcome(_result(0), _report(passed=True)), "survived")
        self.assertEqual(classify_prover_outcome(_result(None, timed_out=True), None), "prover_timeout")


if __name__ == "__main__":
    unittest.main()
