from __future__ import annotations

import asyncio
import os
import unittest
from pathlib import Path
from unittest.mock import patch

from harness.pilot_preflight import (
    _check_dispatch_abort,
    _check_solver_executable,
    _check_versioned_executable,
)


class PilotPreflightTest(unittest.TestCase):
    def test_version_check_prefers_configured_executable(self) -> None:
        checks: list[dict[str, object]] = []
        _check_versioned_executable(
            checks,
            "configured",
            "missing-from-path",
            None,
            configured_path="/usr/bin/true",
        )

        self.assertTrue(checks[0]["passed"], checks[0]["detail"])
        self.assertIn("/usr/bin/true", str(checks[0]["detail"]))

    def test_solver_check_rejects_nonexistent_configured_path(self) -> None:
        checks: list[dict[str, object]] = []
        with patch.dict(os.environ, {"TEST_SOLVER_EXE": "/not/a/solver"}):
            _check_solver_executable(
                checks,
                "test_solver",
                "TEST_SOLVER_EXE",
                "test-solver",
                ("--version",),
            )

        self.assertEqual("test_solver", checks[0]["name"])
        self.assertFalse(checks[0]["passed"])
        self.assertIn("not executable", str(checks[0]["detail"]))

    def test_solver_check_probes_executable(self) -> None:
        checks: list[dict[str, object]] = []
        executable = Path("/usr/bin/true")
        with patch.dict(os.environ, {"TEST_SOLVER_EXE": str(executable)}):
            _check_solver_executable(
                checks,
                "test_solver",
                "TEST_SOLVER_EXE",
                "test-solver",
                (),
            )

        self.assertTrue(checks[0]["passed"], checks[0]["detail"])
        self.assertIn("sha256=", str(checks[0]["detail"]))


if __name__ == "__main__":
    unittest.main()


class DispatchAbortCheckTest(unittest.TestCase):
    """The dispatcher calls preflight from inside its own event loop."""

    def test_rehearsal_runs_standalone(self) -> None:
        checks: list[dict[str, object]] = []
        _check_dispatch_abort(checks)
        self.assertTrue(checks[0]["passed"], checks[0]["detail"])

    def test_rehearsal_runs_inside_a_running_event_loop(self) -> None:
        async def from_the_dispatcher() -> list[dict[str, object]]:
            checks: list[dict[str, object]] = []
            _check_dispatch_abort(checks)
            return checks

        checks = asyncio.run(from_the_dispatcher())
        self.assertTrue(checks[0]["passed"], checks[0]["detail"])
