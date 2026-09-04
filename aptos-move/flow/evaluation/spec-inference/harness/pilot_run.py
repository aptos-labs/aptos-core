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
    refutation_mutants_root: Path | None = None,
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
        )
    )
    print(json.dumps({"complete": result["complete"], "runs": len(result["results"])}))
    if not result["complete"]:
        raise SystemExit("one or more pilot launches failed")


if __name__ == "__main__":
    main()
