from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from pathlib import Path

from harness.dispatch import INFRASTRUCTURE_ABORT_THRESHOLD
from types import SimpleNamespace
from unittest.mock import patch

from harness.pilot_run import run_pilot


class PilotRunTest(unittest.TestCase):
    def test_infrastructure_failure_aborts_queued_cells(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            schedule = root / "schedule" / "runs"
            schedule.mkdir(parents=True)
            for index in range(72):
                (schedule / f"run-{index:02}.json").touch()
            (root / "schedule" / "pilot-manifest.json").write_text(
                json.dumps(
                    {"runs": 72, "blocks": 24, "replicates": 3, "task_count": 8}
                ),
                encoding="utf-8",
            )
            artifacts = root / "artifacts"
            launches = 0

            class FakeProcess:
                returncode = 0

                async def communicate(self) -> tuple[bytes, bytes]:
                    return b"", b""

            async def create_process(*command: str, **_kwargs: object) -> FakeProcess:
                nonlocal launches
                launches += 1
                manifest = Path(command[command.index("--run") + 1])
                artifact = artifacts / manifest.stem
                artifact.mkdir(parents=True)
                (artifact / "judge.json").write_text("{}\n", encoding="utf-8")
                (artifact / "controller-events.jsonl").write_text(
                    json.dumps(
                        {
                            "event": "run_end",
                            "terminal_status": "invalid_infrastructure_failure",
                        }
                    )
                    + "\n",
                    encoding="utf-8",
                )
                return FakeProcess()

            def load_run(path: Path) -> SimpleNamespace:
                return SimpleNamespace(run_id=path.stem, block=1, order=1)

            with (
                patch(
                    "harness.pilot_run.preflight",
                    return_value={"ready": True, "checks": []},
                ),
                patch("harness.pilot_run.RunSpec.load", side_effect=load_run),
                patch(
                    "harness.dispatch.asyncio.create_subprocess_exec",
                    side_effect=create_process,
                ),
            ):
                report = asyncio.run(
                    run_pilot(
                        root / "schedule",
                        artifacts,
                        root / "config.json",
                        root / "wrapper",
                        1,
                        root / "report.json",
                    )
                )

            # A real outage reaches every cell, so the threshold's worth
            # launch and corroborate it; the remaining 70 are withheld.
            self.assertEqual(INFRASTRUCTURE_ABORT_THRESHOLD, launches)
            self.assertTrue(report["aborted"])
            self.assertFalse(report["complete"])
            self.assertEqual(
                72 - INFRASTRUCTURE_ABORT_THRESHOLD,
                sum(result["status"] == "batch_aborted" for result in report["results"]),
            )

    def test_failed_preflight_prevents_any_launch_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = root / "runs"
            with patch(
                "harness.pilot_run.preflight",
                return_value={
                    "ready": False,
                    "checks": [
                        {"name": "provider_auth", "passed": False, "detail": "missing"}
                    ],
                },
            ):
                with self.assertRaisesRegex(RuntimeError, "provider_auth"):
                    asyncio.run(
                        run_pilot(
                            root / "schedule",
                            artifacts,
                            root / "config.json",
                            root / "wrapper",
                            1,
                            root / "report.json",
                        )
                    )
            self.assertFalse(artifacts.exists())


if __name__ == "__main__":
    unittest.main()
