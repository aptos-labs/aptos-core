"""Run one versioned Phase 4 round through an explicit sandbox wrapper."""

from __future__ import annotations

import argparse
import asyncio
from datetime import datetime, timezone
import fcntl
import json
import os
import sys
from pathlib import Path
from typing import Any

from .config import RunSpec
from .artifacts import write_json
from .dispatch import INFRASTRUCTURE_TERMINAL_STATUS, dispatch_round, read_terminal_status
from .pilot import load_round_shape
from .pilot_preflight import preflight


async def run_pilot(
    schedule_dir: Path,
    artifacts_dir: Path,
    config_path: Path,
    sandbox_wrapper: Path,
    concurrency: int,
    report_path: Path,
    refutation_mutants_root: Path | None = None,
    resume: bool = False,
) -> dict[str, Any]:
    if concurrency < 1:
        raise ValueError("concurrency must be positive")
    preflight_result = preflight(config_path, schedule_dir, sandbox_wrapper)
    if not preflight_result["ready"]:
        failed = [
            check["name"]
            for check in preflight_result["checks"]
            if not check["passed"]
        ]
        raise RuntimeError(
            f"refusing to launch pilot; failed preflight checks: {', '.join(failed)}"
        )
    expected_runs = load_round_shape(schedule_dir).runs
    run_paths = sorted((schedule_dir / "runs").glob("*.json"), key=_schedule_key)
    if len(run_paths) != expected_runs:
        raise ValueError(
            f"pilot round requires {expected_runs} manifests, found {len(run_paths)}"
        )

    def launch_command(manifest: Path) -> list[str]:
        # The scoring set never enters this command: the agent shares the
        # wrapper's namespace, so only Landlock separates it from what the
        # controller can read. `score_round` runs afterwards, outside. A
        # *refutation* set may be passed, and must be a different set.
        command = [
            str(sandbox_wrapper),
            sys.executable,
            "-m",
            "harness.controller",
            "--config",
            str(config_path),
            "--run",
            str(manifest),
            "--artifacts",
            str(artifacts_dir),
            "--skip-hidden-scoring",
        ]
        if refutation_mutants_root is not None:
            command += ["--refutation-mutants-root", str(refutation_mutants_root)]
        return command

    artifacts_dir.mkdir(parents=True, exist_ok=True)
    with (artifacts_dir / ".dispatch.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        cells = [(RunSpec.load(path).run_id, path) for path in run_paths]
        acknowledged = frozenset(
            run_id for run_id, _ in cells
            if resume and (artifacts_dir / run_id / "judge.json").is_file()
            and read_terminal_status(artifacts_dir / run_id) == INFRASTRUCTURE_TERMINAL_STATUS
        )
        if report_path.exists():
            if not resume:
                raise ValueError("launch report already exists; use --resume to archive and continue")
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
            archive = report_path.with_name(f"{report_path.stem}.before-resume-{stamp}.json")
            with archive.open("x") as output:
                output.write(report_path.read_text())
        if resume:
            write_json(report_path.with_suffix(".resume.json"), {
                "started_utc": datetime.now(timezone.utc).isoformat(),
                "acknowledged_failures": sorted(acknowledged),
                "concurrency": concurrency,
                "preflight": preflight_result,
            })
        report = await dispatch_round(
            cells, artifacts_dir, launch_command, concurrency,
            Path(__file__).resolve().parent.parent,
            acknowledged_failures=acknowledged,
        )
        write_json(report_path, report)
    return report


def _schedule_key(path: Path) -> tuple[int, int, str]:
    spec = RunSpec.load(path)
    return spec.block, spec.order, spec.run_id


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--schedule-dir", type=Path, required=True)
    parser.add_argument("--artifacts-dir", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--sandbox-wrapper", type=Path, required=True)
    parser.add_argument("--concurrency", type=int, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument(
        "--resume", action="store_true",
        help="retain recorded outcomes, archive the prior launch report, and acknowledge "
        "historical infrastructure failures without counting them as new outages; "
        "the original apparatus preflight still applies",
    )
    parser.add_argument(
        "--refutation-mutants-root",
        type=Path,
        help="mutants the controller refutes an accepted contract against, sending "
        "a too-weak one back. Mounted in the agent's namespace and withheld only by "
        "Landlock, so never pass the set the round is scored on.",
    )
    args = parser.parse_args()
    wrapper = args.sandbox_wrapper.resolve()
    if not wrapper.is_file() or not os.access(wrapper, os.X_OK):
        raise SystemExit(f"sandbox wrapper is not executable: {wrapper}")
    result = asyncio.run(
        run_pilot(
            args.schedule_dir.resolve(),
            args.artifacts_dir.resolve(),
            args.config.resolve(),
            wrapper,
            args.concurrency,
            args.report.resolve(),
            args.refutation_mutants_root.resolve() if args.refutation_mutants_root else None,
            args.resume,
        )
    )
    print(json.dumps({"complete": result["complete"], "runs": len(result["results"])}))
    if not result["complete"]:
        raise SystemExit("one or more pilot launches failed")


if __name__ == "__main__":
    main()
