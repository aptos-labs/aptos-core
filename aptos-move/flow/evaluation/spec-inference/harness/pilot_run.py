"""Run one versioned Phase 4 round through an explicit sandbox wrapper."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any

from .config import RunSpec
from .artifacts import write_json
from .dispatch import dispatch_round
from .pilot import load_round_shape
from .pilot_preflight import preflight


async def run_pilot(
    schedule_dir: Path,
    artifacts_dir: Path,
    config_path: Path,
    sandbox_wrapper: Path,
    concurrency: int,
    report_path: Path,
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
        # Hidden mutants never enter this command. The agent shares the
        # wrapper's mount namespace, so anything the controller could read here
        # the agent could read too. Strict scoring runs after the round, in
        # `harness.score_round`, against the finished workspaces.
        return [
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

    report = await dispatch_round(
        [(RunSpec.load(path).run_id, path) for path in run_paths],
        artifacts_dir,
        launch_command,
        concurrency,
        Path(__file__).resolve().parent.parent,
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
        )
    )
    print(json.dumps({"complete": result["complete"], "runs": len(result["results"])}))
    if not result["complete"]:
        raise SystemExit("one or more pilot launches failed")


if __name__ == "__main__":
    main()
