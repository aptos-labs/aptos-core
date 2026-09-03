"""Concurrent dispatch of one scheduled round with infrastructure gating.

Dispatch is separated from the readiness gate in `pilot_preflight` so that
preflight can exercise this loop against a stub launcher without a circular
dependency. Both the real round and the preflight rehearsal therefore share one
implementation of the abort rule.
"""

from __future__ import annotations

import asyncio
import json
import os
import signal
from contextlib import suppress
from pathlib import Path
from typing import Any, Callable, Sequence

from .credentials import redact


INFRASTRUCTURE_TERMINAL_STATUS = "invalid_infrastructure_failure"
#: Cells that must report an infrastructure failure before the batch is
#: abandoned.
#:
#: The signal is derived from telemetry the agent's own MCP server writes, and
#: that file is in the agent's writable set, so one cell can produce it at
#: will. A genuine outage -- no solver, a broken toolchain -- fails every cell
#: it reaches, so requiring corroboration keeps the fast abort for the real
#: case while denying one session the power to end the round.
INFRASTRUCTURE_ABORT_THRESHOLD = 2
COMPLETED_STATUSES = frozenset({"complete", "already_complete"})


#: How long a cell may take to honour `SIGTERM` before it is killed outright.
TERMINATE_GRACE_SECONDS = 30


def _signal(process: asyncio.subprocess.Process, number: int) -> None:
    """Signal a launched cell and the sandbox tree it started.

    The launcher execs bubblewrap, which execs the controller, so signalling
    the immediate child alone can leave the session running; each cell is
    started in its own process group for exactly this reason. The direct
    signal is the fallback for a platform where the group signal does not
    reach it. A process that has already exited is not an error.
    """
    with suppress(ProcessLookupError):
        os.killpg(process.pid, number)
    with suppress(ProcessLookupError):
        if number == signal.SIGKILL:
            process.kill()
        else:
            process.terminate()


async def _stop(process: asyncio.subprocess.Process) -> None:
    """Stop a cell, escalating if it does not honour the first signal.

    `SIGTERM` alone is a request. A session that ignores it -- or a wrapper
    that does not pass it on -- would otherwise keep spending model and solver
    budget after the batch has already discarded its result, which is the
    outcome the abort exists to prevent.
    """
    _signal(process, signal.SIGTERM)
    with suppress(Exception):
        await asyncio.wait_for(process.wait(), TERMINATE_GRACE_SECONDS)
        return
    _signal(process, signal.SIGKILL)
    with suppress(Exception):
        await asyncio.wait_for(process.wait(), TERMINATE_GRACE_SECONDS)


async def dispatch_round(
    cells: Sequence[tuple[str, Path]],
    artifacts_dir: Path,
    launch_command: Callable[[Path], list[str]],
    concurrency: int,
    cwd: Path,
) -> dict[str, Any]:
    """Launch each `(run_id, manifest)` cell, stopping at the first outage.

    A terminal infrastructure failure means the controller already exhausted its
    configured retry, so the cause is external. Every queued cell would record
    the same invalid observation, and the randomized order would be spent on it,
    so the batch closes instead of draining.
    """
    if concurrency < 1:
        raise ValueError("concurrency must be positive")
    artifacts_dir.mkdir(parents=True, exist_ok=True)
    semaphore = asyncio.Semaphore(concurrency)
    abort_batch = asyncio.Event()
    infrastructure_failures: set[str] = set()
    running: set[asyncio.subprocess.Process] = set()

    def record_infrastructure_failure(run_id: str) -> None:
        infrastructure_failures.add(run_id)
        if len(infrastructure_failures) >= INFRASTRUCTURE_ABORT_THRESHOLD:
            abort_batch.set()

    async def launch(run_id: str, manifest: Path) -> dict[str, Any]:
        artifact = artifacts_dir / run_id
        if (artifact / "judge.json").is_file():
            terminal_status = read_terminal_status(artifact)
            if terminal_status == INFRASTRUCTURE_TERMINAL_STATUS:
                record_infrastructure_failure(run_id)
                return {
                    "run_id": run_id,
                    "status": "existing_infrastructure_failure",
                    "returncode": None,
                    "terminal_status": terminal_status,
                }
            return {"run_id": run_id, "status": "already_complete", "returncode": 0}
        if artifact.exists():
            return {"run_id": run_id, "status": "partial_artifact", "returncode": None}
        environment = os.environ.copy()
        # The wrapper must set the sandbox marker inside the isolated process;
        # setting it here would let an unsandboxed invocation pass the guard.
        environment.pop("MOVE_INFERENCE_EVAL_SANDBOXED", None)
        async with semaphore:
            if abort_batch.is_set():
                return {
                    "run_id": run_id,
                    "status": "batch_aborted",
                    "returncode": None,
                }
            process = await asyncio.create_subprocess_exec(
                *launch_command(manifest),
                cwd=cwd,
                env=environment,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                # Its own process group, so abandoning the batch can stop the
                # whole sandbox tree rather than just the launcher, matching
                # how `judge.run_command` starts the prover.
                start_new_session=True,
            )
            running.add(process)
            try:
                # Cells already running when the batch is abandoned have to be
                # stopped, not merely uncounted: at a concurrency above the
                # threshold they would otherwise each spend a full model and
                # solver budget producing a result the round then discards.
                # Waiting on the abort event alongside the process is what
                # makes the threshold a limit rather than a tally.
                communicate = asyncio.ensure_future(process.communicate())
                abort = asyncio.ensure_future(abort_batch.wait())
                done, _ = await asyncio.wait(
                    (communicate, abort), return_when=asyncio.FIRST_COMPLETED
                )
                if communicate not in done:
                    await _stop(process)
                    communicate.cancel()
                    abort.cancel()
                    return {
                        "run_id": run_id,
                        "status": "batch_aborted",
                        "returncode": None,
                    }
                abort.cancel()
                stdout, stderr = communicate.result()
            finally:
                running.discard(process)
        terminal_status = read_terminal_status(artifact)
        if terminal_status == INFRASTRUCTURE_TERMINAL_STATUS:
            record_infrastructure_failure(run_id)
        return {
            "run_id": run_id,
            "status": (
                "infrastructure_failure"
                if terminal_status == INFRASTRUCTURE_TERMINAL_STATUS
                else "complete" if process.returncode == 0 else "launch_failed"
            ),
            "returncode": process.returncode,
            "terminal_status": terminal_status,
            # The controller redacts its own artifact tree, but its stdout can
            # still quote agent-written source -- a compiler diagnostic echoes
            # the line it rejected -- and this report is written outside that
            # tree, so it is redacted here.
            "stdout_tail": redact(stdout.decode(errors="replace")[-2000:]),
            "stderr_tail": redact(stderr.decode(errors="replace")[-2000:]),
        }

    async def launch_or_report(run_id: str, manifest: Path) -> dict[str, Any]:
        # A cell that raises is one failed cell, not a failed round. Without
        # this, `gather` cancels every sibling on the first exception while the
        # subprocesses -- deliberately in their own sessions -- keep spending
        # model and solver budget with nothing left to collect them.
        try:
            return await launch(run_id, manifest)
        except Exception as error:
            return {
                "run_id": run_id,
                "status": "launch_error",
                "returncode": None,
                "detail": f"{type(error).__name__}: {error}",
            }

    try:
        results = await asyncio.gather(
            *(launch_or_report(run_id, path) for run_id, path in cells)
        )
    finally:
        # Whatever ends the dispatch -- an unexpected error here, or the
        # operator interrupting it -- no cell outlives it. `start_new_session`
        # is what makes them survivable, so it is also what makes this needed.
        # Signals are sent without awaiting first, because this may be running
        # during a cancellation where an await would not resume.
        stragglers = list(running)
        for process in stragglers:
            _signal(process, signal.SIGTERM)
        with suppress(BaseException):
            await asyncio.wait_for(
                asyncio.gather(*(p.wait() for p in stragglers), return_exceptions=True),
                TERMINATE_GRACE_SECONDS,
            )
        for process in stragglers:
            _signal(process, signal.SIGKILL)
    return {
        "schema_version": 1,
        "scheduled_runs": len(cells),
        "concurrency": concurrency,
        "aborted": abort_batch.is_set(),
        "complete": not abort_batch.is_set()
        and all(result["status"] in COMPLETED_STATUSES for result in results),
        "results": results,
    }


def read_terminal_status(artifact: Path) -> str | None:
    """Return the terminal status the controller recorded for a run, if any."""
    events = artifact / "controller-events.jsonl"
    if not events.is_file():
        return None
    terminal_status = None
    for line in events.read_text(encoding="utf-8").splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("event") == "run_end":
            value = event.get("terminal_status")
            terminal_status = value if isinstance(value, str) else None
    return terminal_status


async def rehearse_abort(root: Path, launcher: Callable[[Path], list[str]]) -> dict[str, Any]:
    """Dispatch three synthetic cells whose first launch reports an outage.

    This runs the deployed dispatch loop, so a regression that lets a round
    drain during a provider outage fails here rather than during the round.
    """
    schedule = root / "runs"
    schedule.mkdir(parents=True)
    cells = []
    for index in range(3):
        manifest = schedule / f"rehearsal-{index}.json"
        manifest.write_text("{}\n", encoding="utf-8")
        cells.append((manifest.stem, manifest))
    return await dispatch_round(cells, root / "artifacts", launcher, 1, root)
