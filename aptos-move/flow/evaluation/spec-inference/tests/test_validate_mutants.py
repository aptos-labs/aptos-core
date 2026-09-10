from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock

from harness.judge import CommandResult
from harness.validate_mutants import validate_mutants


def _ok(stdout: str = "") -> CommandResult:
    return CommandResult(
        argv=["stub"],
        returncode=0,
        duration_ms=1,
        timed_out=False,
        infrastructure_error=None,
        stdout=stdout,
        stderr="",
    )


class ModuleShapeTest(unittest.TestCase):
    """Every definition must precede the script entrypoint.

    A `def` below `if __name__ == "__main__": main()` exists when the module is
    imported -- so a unit test sees it -- and does not exist when the module is
    run as a script, which is how the harness invokes it. That is how a guard
    shipped here could never once have executed.
    """

    def test_no_definition_follows_the_entrypoint(self) -> None:
        import inspect

        from harness import validate_mutants

        source = inspect.getsource(validate_mutants).split("\n")
        entry = next(
            i for i, line in enumerate(source) if line.startswith('if __name__ ==')
        )
        late = [
            line.split("(")[0]
            for line in source[entry:]
            if line.startswith("def ") or line.startswith("async def ")
        ]
        self.assertEqual([], late, "these are undefined when run as a script")


class ReferenceVacuityTest(unittest.IsolatedAsyncioTestCase):
    """Essentiality rests on the reference not being vacuous.

    A reference whose assumptions are contradictory proves every postcondition,
    so every mutant survives it and none is marked essential. Strict success --
    the study's success criterion -- is built on those values, so the check that
    establishes non-vacuity must never pass by default.
    """

    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        root = Path(self._temporary.name)
        self.manifest = root / "mutants.json"
        self.manifest.write_text(
            json.dumps({"mutants": [{"mutant_id": "m1"}]}), encoding="utf-8"
        )
        self.reference = root / "reference"
        (self.reference / "sources").mkdir(parents=True)
        self.baseline = root / "baseline"
        self.baseline.mkdir()

    def tearDown(self) -> None:
        self._temporary.cleanup()

    async def _run(
        self, inconsistency: CommandResult, implementation_equal: bool = True
    ) -> None:
        # compile, prove, the inconsistency check, then the reference/baseline
        # implementation comparison.
        results = [_ok(), _ok(), inconsistency, _ok()]
        self._implementation_equal = implementation_equal
        def write_verdict(*_args, **_kwargs):
            path = self._verdict_path
            if path is not None:
                path.write_text(
                    json.dumps(
                        {
                            "implementation": {
                                "ran": True,
                                "equal": self._implementation_equal,
                                "changed_modules": ["0x1::m"],
                            }
                        }
                    ),
                    encoding="utf-8",
                )
            return results.pop(0)

        self._verdict_path = None

        def capture(command, config=None, output=None, **_kwargs):
            if output is not None:
                self._verdict_path = Path(output)
            return ["stub"]

        with mock.patch(
            "harness.validate_mutants.run_command", side_effect=write_verdict
        ), mock.patch("harness.validate_mutants.render_command", side_effect=capture):
            await validate_mutants(
                mock.MagicMock(),
                self.reference,
                self.baseline,
                "0x1::m::f",
                self.manifest,
                10,
            )

    async def test_a_reference_that_changes_the_implementation_is_refused(self) -> None:
        # Essentiality is a claim about the specification. A reference whose
        # executable code differs would certify mutants against different
        # behaviour and carry that into the strict scores.
        with self.assertRaises(ValueError) as raised:
            await self._run(_ok(), implementation_equal=False)
        self.assertIn("differs from the baseline", str(raised.exception))

    async def test_an_inconsistent_reference_is_refused(self) -> None:
        with self.assertRaises(ValueError) as raised:
            await self._run(
                replace(
                    _ok(),
                    returncode=1,
                    stdout="error: there is an inconsistent assumption in the function",
                )
            )
        self.assertIn("vacuous", str(raised.exception))

    async def test_an_inconsistency_check_that_reached_no_verdict_is_refused(
        self,
    ) -> None:
        # Silence from a check that never ran is not evidence of non-vacuity.
        for outcome, expected in (
            (replace(_ok(), returncode=1, stderr="boogie crashed"), "exited 1"),
            (replace(_ok(), returncode=None, timed_out=True), "timed out"),
            (
                replace(_ok(), returncode=None, infrastructure_error="no solver"),
                "could not run",
            ),
        ):
            with self.subTest(outcome=outcome):
                with self.assertRaises(ValueError) as raised:
                    await self._run(outcome)
                message = str(raised.exception)
                self.assertIn("vacuity is unproven", message)
                self.assertIn(expected, message)


if __name__ == "__main__":
    unittest.main()
