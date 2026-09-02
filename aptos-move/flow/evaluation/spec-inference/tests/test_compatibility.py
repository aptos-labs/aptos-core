import tempfile
import unittest
from pathlib import Path

from harness.compatibility import _failure_kind, _find_untrusted_inferred_conditions
from harness.judge import CommandResult


def result(
    *,
    returncode: int | None = 0,
    timed_out: bool = False,
    infrastructure_error: str | None = None,
    stderr: str = "",
) -> CommandResult:
    return CommandResult(
        argv=["tool"],
        returncode=returncode,
        duration_ms=1,
        timed_out=timed_out,
        infrastructure_error=infrastructure_error,
        stdout="",
        stderr=stderr,
    )


class CompatibilityFailureKindTest(unittest.TestCase):
    def test_timeout_is_the_only_exclusion_kind(self) -> None:
        self.assertEqual(
            _failure_kind({"compile": result(timed_out=True), "wp": None}),
            "compatibility_timeout",
        )

    def test_missing_backend_is_infrastructure(self) -> None:
        self.assertEqual(
            _failure_kind(
                {
                    "compile": result(),
                    "prover": result(
                        returncode=1,
                        stderr="No such file or directory: /tools/boogie",
                    ),
                }
            ),
            "infrastructure_failure",
        )

    def test_wp_error_requires_implementation_fix(self) -> None:
        self.assertEqual(
            _failure_kind(
                {
                    "compile": result(),
                    "wp": result(returncode=1, stderr="internal WP diagnostic"),
                }
            ),
            "implementation_failure",
        )

    def test_flagged_conditions_are_reported_without_mutating_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "target.spec.move"
            path.write_text(
                """spec 0x42::m {
    spec f {
        ensures [inferred = vacuous] forall x: vector<u64>: {
            let y = x;
            y == x
        };
        // ensures [inferred = sathard] false;
        ensures [inferred] result == 1;
    }
}
""",
                encoding="utf-8",
            )
            findings = _find_untrusted_inferred_conditions(Path(temporary))
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0]["path"], "target.spec.move")
            self.assertEqual(findings[0]["line"], 3)
            result = path.read_text(encoding="utf-8")
            self.assertIn("inferred = vacuous", result)
            self.assertIn("// ensures [inferred = sathard] false;", result)
            self.assertIn("ensures [inferred] result == 1;", result)

    def test_reports_hard_clause_without_changing_state_labels(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "target.spec.move"
            path.write_text(
                """spec 0x42::m {
    spec f {
        ensures [inferred] ..S1 |~ ensures_of<first>();
        ensures [inferred = sathard] S1 |~ ensures_of<second>();
        ensures [inferred] S0..S2 |~ ensures_of<third>();
    }
    spec g {
        ensures [inferred] ..S1 |~ ensures_of<producer>();
        ensures [inferred] S1 |~ ensures_of<consumer>();
    }
}
""",
                encoding="utf-8",
            )
            findings = _find_untrusted_inferred_conditions(Path(temporary))
            self.assertEqual(len(findings), 1)
            result = path.read_text(encoding="utf-8")
            self.assertIn("ensures [inferred] ..S1 |~ ensures_of<first>();", result)
            self.assertIn("ensures [inferred] S0..S2 |~ ensures_of<third>();", result)
            self.assertIn("ensures [inferred] ..S1 |~ ensures_of<producer>();", result)
            self.assertIn("..S2", result)


if __name__ == "__main__":
    unittest.main()
