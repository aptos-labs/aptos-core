from __future__ import annotations

import asyncio
import json
import sys
import tempfile
import unittest
from pathlib import Path

import os

from harness.dispatch import (
    INFRASTRUCTURE_ABORT_THRESHOLD,
    dispatch_round,
    read_terminal_status,
    rehearse_abort,
)


def _write_ledger(artifact: Path, terminal_status: str) -> None:
    artifact.mkdir(parents=True)
    (artifact / "judge.json").write_text("{}\n", encoding="utf-8")
    (artifact / "controller-events.jsonl").write_text(
        json.dumps({"event": "run_end", "terminal_status": terminal_status}) + "\n",
        encoding="utf-8",
    )


def _launcher(artifacts: Path, terminal_status: str):
    program = (
        "import json,pathlib,sys;"
        "artifact=pathlib.Path(sys.argv[1])/pathlib.Path(sys.argv[2]).stem;"
        "artifact.mkdir(parents=True);"
        "(artifact/'judge.json').write_text('{}\\n');"
        "(artifact/'controller-events.jsonl').write_text("
        "json.dumps({'event':'run_end','terminal_status':sys.argv[3]})+'\\n')"
    )
    return lambda manifest: [
        sys.executable,
        "-c",
        program,
        str(artifacts),
        str(manifest),
        terminal_status,
    ]


class TerminalStatusTest(unittest.TestCase):
    def test_reads_last_run_end_and_skips_unparsable_lines(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary)
            (artifact / "controller-events.jsonl").write_text(
                "not-json\n"
                + json.dumps({"event": "prompt"})
                + "\n"
                + json.dumps(
                    {
                        "event": "run_end",
                        "terminal_status": "invalid_infrastructure_failure",
                    }
                )
                + "\n",
                encoding="utf-8",
            )

            self.assertEqual(
                "invalid_infrastructure_failure", read_terminal_status(artifact)
            )

    def test_missing_ledger_has_no_terminal_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            self.assertIsNone(read_terminal_status(Path(temporary)))


class RehearsalTest(unittest.TestCase):
    def test_outage_withholds_every_queued_cell(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report = asyncio.run(
                rehearse_abort(
                    root, _launcher(root / "artifacts", "invalid_infrastructure_failure")
                )
            )

            self.assertTrue(report["aborted"])
            self.assertFalse(report["complete"])
            # Two cells run and corroborate the outage; the third is withheld.
            self.assertEqual(
                1,
                sum(
                    result["status"] == "batch_aborted" for result in report["results"]
                ),
            )

    def test_healthy_launches_complete_the_batch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report = asyncio.run(
                rehearse_abort(root, _launcher(root / "artifacts", "operational_success"))
            )

            self.assertFalse(report["aborted"])
            self.assertTrue(report["complete"])
            self.assertEqual(
                3, sum(result["status"] == "complete" for result in report["results"])
            )

    def test_one_infrastructure_failure_does_not_close_the_batch(self) -> None:
        # The signal comes from telemetry the agent can write, so a single cell
        # must not be able to end the round. It is still reported.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _write_ledger(
                root / "artifacts" / "rehearsal-0", "invalid_infrastructure_failure"
            )

            report = asyncio.run(
                rehearse_abort(root, _launcher(root / "artifacts", "operational_success"))
            )

            self.assertFalse(report["aborted"])
            self.assertEqual(
                "existing_infrastructure_failure", report["results"][0]["status"]
            )

    def test_corroborated_infrastructure_failures_close_the_batch(self) -> None:
        # A genuine outage reaches every cell, so the threshold is met at once.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for index in range(INFRASTRUCTURE_ABORT_THRESHOLD):
                _write_ledger(
                    root / "artifacts" / f"rehearsal-{index}",
                    "invalid_infrastructure_failure",
                )

            report = asyncio.run(
                rehearse_abort(root, _launcher(root / "artifacts", "operational_success"))
            )

            self.assertTrue(report["aborted"])


class InFlightAbortTest(unittest.TestCase):
    def test_running_cells_are_stopped_when_the_batch_is_abandoned(self) -> None:
        # Above the corroboration threshold, several cells run at once. Once
        # enough of them report an outage the rest must be stopped, not merely
        # left to finish spending a model and solver budget on a result the
        # round discards.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = root / "artifacts"
            failing = (
                "import json,pathlib,sys\n"
                "a=pathlib.Path(sys.argv[1])/pathlib.Path(sys.argv[2]).stem\n"
                "a.mkdir(parents=True)\n"
                "(a/'judge.json').write_text('{}')\n"
                "(a/'controller-events.jsonl').write_text(json.dumps("
                "{'event':'run_end','terminal_status':"
                "'invalid_infrastructure_failure'})+'\\n')\n"
            )
            slow = "import time\ntime.sleep(600)\n"

            def launcher(manifest: Path) -> list[str]:
                program = failing if manifest.stem.startswith("bad") else slow
                return [sys.executable, "-c", program, str(artifacts), str(manifest)]

            cells = [
                ("bad-0", root / "bad-0.json"),
                ("bad-1", root / "bad-1.json"),
                ("slow-0", root / "slow-0.json"),
                ("slow-1", root / "slow-1.json"),
            ]
            report = asyncio.run(
                dispatch_round(cells, artifacts, launcher, 4, root)
            )

            self.assertTrue(report["aborted"])
            statuses = {r["run_id"]: r["status"] for r in report["results"]}
            for run_id in ("slow-0", "slow-1"):
                self.assertEqual("batch_aborted", statuses[run_id], statuses)


class LaunchFailureTest(unittest.TestCase):
    def test_one_failing_cell_neither_cancels_nor_orphans_the_others(self) -> None:
        # `gather` cancels siblings on the first exception, and cells run in
        # their own sessions, so an unhandled error would leave them spending
        # budget with nothing left to collect them.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = root / "artifacts"

            def launcher(manifest: Path) -> list[str]:
                if manifest.stem == "bad":
                    raise RuntimeError("cannot build a command for this cell")
                return [sys.executable, "-c", "pass"]

            report = asyncio.run(
                dispatch_round(
                    [("bad", root / "bad.json"), ("good", root / "good.json")],
                    artifacts,
                    launcher,
                    2,
                    root,
                )
            )

            statuses = {r["run_id"]: r["status"] for r in report["results"]}
            self.assertEqual("launch_error", statuses["bad"])
            self.assertIn("cannot build a command", statuses and
                          next(r for r in report["results"] if r["run_id"] == "bad")["detail"])
            # The sibling still ran and was recorded.
            self.assertIn(statuses["good"], {"complete", "launch_failed"})
            self.assertFalse(report["complete"])


class TerminationEscalationTest(unittest.IsolatedAsyncioTestCase):
    async def test_a_cell_that_ignores_sigterm_is_killed(self) -> None:
        # SIGTERM is a request. A session that traps it would otherwise keep
        # spending model and solver budget after the batch discarded its
        # result -- the outcome the abort exists to prevent.
        import harness.dispatch as dispatch

        deaf = (
            "import signal, time\n"
            "signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
            "print('ready', flush=True)\n"
            "time.sleep(600)\n"
        )
        process = await asyncio.create_subprocess_exec(
            sys.executable, "-c", deaf,
            stdout=asyncio.subprocess.PIPE,
            start_new_session=True,
        )
        await process.stdout.readline()

        original = dispatch.TERMINATE_GRACE_SECONDS
        dispatch.TERMINATE_GRACE_SECONDS = 1
        try:
            await dispatch._stop(process)
            # Read before the safety net below, which would otherwise be the
            # thing that killed it and make this assertion vacuous.
            stopped = process.returncode
        finally:
            dispatch.TERMINATE_GRACE_SECONDS = original
            if process.returncode is None:
                process.kill()
                await process.wait()

        self.assertIsNotNone(stopped, "a cell ignoring SIGTERM must still be stopped")
        self.assertEqual(-9, stopped)


class ReportRedactionTest(unittest.TestCase):
    def test_the_credential_does_not_survive_in_a_launch_report(self) -> None:
        # The controller redacts its own artifact tree; its stdout can still
        # quote agent-written source, and this report lives outside that tree.
        secret = "sk-test-launch-report-credential"
        program = "import sys;sys.stdout.write(sys.argv[1]);sys.stderr.write(sys.argv[1])"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous = os.environ.get("ANTHROPIC_AUTH_TOKEN")
            os.environ["ANTHROPIC_AUTH_TOKEN"] = secret
            try:
                report = asyncio.run(
                    dispatch_round(
                        [("cell", root / "cell.json")],
                        root / "artifacts",
                        lambda manifest: [sys.executable, "-c", program, secret],
                        1,
                        root,
                    )
                )
            finally:
                if previous is None:
                    del os.environ["ANTHROPIC_AUTH_TOKEN"]
                else:
                    os.environ["ANTHROPIC_AUTH_TOKEN"] = previous

            result = report["results"][0]
            self.assertNotIn(secret, json.dumps(report))
            self.assertEqual("[REDACTED]", result["stdout_tail"])
            self.assertEqual("[REDACTED]", result["stderr_tail"])


if __name__ == "__main__":
    unittest.main()
