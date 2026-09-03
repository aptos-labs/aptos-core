"""Credential-safe executable and schedule preflight for the unscored pilot."""

from __future__ import annotations

import argparse
import asyncio
from concurrent.futures import ThreadPoolExecutor
import importlib.metadata
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from .artifacts import sha256_file, tree_hash, write_json
from .config import ExperimentConfig, RunSpec
from .credentials import CREDENTIAL_VARIABLES, configured_credentials
from .dispatch import INFRASTRUCTURE_ABORT_THRESHOLD, rehearse_abort
from .materialize import materialize_task
from .pilot import load_round_shape


def preflight(config_path: Path, schedule_dir: Path, sandbox_wrapper: Path) -> dict[str, Any]:
    config = ExperimentConfig.load(config_path)
    checks: list[dict[str, Any]] = []
    _check_sandbox_wrapper(checks, sandbox_wrapper)
    _check_versioned_executable(
        checks,
        "claude_code",
        "claude",
        config.claude_code_version,
        configured_path=os.environ.get("CLAUDE_CODE_EXECUTABLE"),
    )
    _check_versioned_executable(checks, "move_flow", "move-flow", None)
    _check_solver_executable(checks, "boogie", "BOOGIE_EXE", "boogie", ("/help",))
    _check_solver_executable(checks, "z3", "Z3_EXE", "z3", ("--version",))

    try:
        sdk_version = importlib.metadata.version("claude-agent-sdk")
        _record(
            checks,
            "claude_agent_sdk",
            sdk_version == config.claude_agent_sdk_version,
            f"expected {config.claude_agent_sdk_version}, found {sdk_version}",
        )
    except importlib.metadata.PackageNotFoundError:
        _record(checks, "claude_agent_sdk", False, "claude-agent-sdk is not installed")

    _check_dispatch_abort(checks)

    configured_auth = configured_credentials()
    _record(
        checks,
        "provider_auth",
        bool(configured_auth),
        "credential present" if configured_auth else f"none of {', '.join(CREDENTIAL_VARIABLES)} is set",
    )
    endpoint = os.environ.get("ANTHROPIC_BASE_URL")
    _record(
        checks,
        "provider_endpoint",
        endpoint in (None, config.provider_base_url),
        f"expected {config.provider_base_url}; environment is {endpoint or 'unset (controller injects expected value)'}",
    )

    try:
        shape = load_round_shape(schedule_dir)
    except Exception as error:
        _record(checks, "schedule", False, f"invalid round manifest: {error}")
        return {
            "schema_version": 1,
            "phase": "unscored_pilot_preflight",
            "ready": False,
            "model": config.model,
            "provider_base_url": config.provider_base_url,
            "checks": checks,
        }
    run_paths = sorted((schedule_dir / "runs").glob("*.json"))
    schedule_ok = len(run_paths) == shape.runs
    identities: set[tuple[str, int, str, str]] = set()
    blocks: dict[tuple[str, int], set[tuple[str, str]]] = {}
    checked_recipes: set[tuple[Path, Path, str]] = set()
    for path in run_paths:
        try:
            resolved = RunSpec.load(path).resolve_paths(path)
            spec = resolved.spec
            recipe = (
                resolved.shared_package,
                resolved.task_patch,
                spec.initial_tree_sha256,
            )
            if recipe not in checked_recipes:
                with tempfile.TemporaryDirectory(
                    prefix="move-inference-preflight-"
                ) as temporary:
                    materialize_task(
                        resolved.shared_package,
                        resolved.task_patch,
                        Path(temporary) / "package",
                        spec.initial_tree_sha256,
                    )
                checked_recipes.add(recipe)
            schedule_ok &= (
                sha256_file(resolved.plugin_dir / "move-flow-manifest.json")
                == spec.plugin_manifest_sha256
                and tree_hash(resolved.plugin_dir) == spec.plugin_tree_sha256
            )
            identities.add(
                (spec.task_id, spec.replicate, spec.arm, spec.feedback_level)
            )
            blocks.setdefault((spec.task_id, spec.replicate), set()).add(
                (spec.arm, spec.feedback_level)
            )
        except Exception:
            schedule_ok = False
    schedule_ok &= len(identities) == shape.runs and len(blocks) == shape.blocks
    # A control level is scheduled for fewer replicates, so a block carries only
    # the cells its own replicate reaches rather than a fixed number.
    schedule_ok &= all(
        len(cells) == shape.cells_in_block(replicate)
        for (_, replicate), cells in blocks.items()
    )
    _record(
        checks,
        "schedule",
        schedule_ok,
        f"{len(run_paths)} manifests, {len(blocks)} randomized blocks, "
        f"{len(identities)} unique cells"
        + (f" over {'/'.join(shape.feedback_levels)}" if shape.feedback_levels else ""),
    )
    return {
        "schema_version": 1,
        "phase": "unscored_pilot_preflight",
        "ready": all(check["passed"] for check in checks),
        "model": config.model,
        "provider_base_url": config.provider_base_url,
        "checks": checks,
    }


def _record(checks: list[dict[str, Any]], name: str, passed: bool, detail: str) -> None:
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


def _check_sandbox_wrapper(checks: list[dict[str, Any]], path: Path) -> None:
    resolved = path.resolve()
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        _record(checks, "sandbox_wrapper", False, f"not executable: {resolved}")
        return
    try:
        process = subprocess.run(
            [str(resolved), "--preflight"], capture_output=True, text=True, timeout=30, check=False
        )
    except Exception as error:
        _record(checks, "sandbox_wrapper", False, f"probe failed: {error}")
        return
    detail = (process.stdout or process.stderr).strip()
    _record(
        checks,
        "sandbox_wrapper",
        process.returncode == 0,
        detail or f"{resolved} --preflight exited {process.returncode}",
    )


def _run_coroutine(coroutine: Any) -> Any:
    """Run a coroutine to completion, with or without an ambient event loop.

    Preflight is a synchronous check but is also called from inside the
    dispatcher's event loop, where `asyncio.run` refuses to nest. A dedicated
    loop on its own thread runs the same coroutine either way.
    """
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(coroutine)
    with ThreadPoolExecutor(max_workers=1) as pool:
        return pool.submit(asyncio.run, coroutine).result()


def _check_dispatch_abort(checks: list[dict[str, Any]]) -> None:
    """Rehearse the dispatch loop against a simulated provider outage.

    The rehearsal writes an invalid-infrastructure controller ledger for the
    first cell. Dispatch must then close the batch, leaving the two queued cells
    unlaunched rather than spending them on the same outage.
    """
    # Assembled outside the argument list and joined explicitly. Adjacent string
    # literals inside a list are how a missing comma silently becomes
    # concatenation, and this is one program rather than several arguments.
    rehearsal_program = "\n".join(
        (
            "import json, pathlib, sys",
            "artifact = pathlib.Path(sys.argv[1]) / pathlib.Path(sys.argv[2]).stem",
            "artifact.mkdir(parents=True)",
            "(artifact / 'judge.json').write_text('{}\\n')",
            "(artifact / 'controller-events.jsonl').write_text(json.dumps("
            + "{'event': 'run_end', 'terminal_status': "
            + "'invalid_infrastructure_failure'}) + '\\n')",
        )
    )
    launcher = [sys.executable, "-c", rehearsal_program]
    try:
        with tempfile.TemporaryDirectory(prefix="move-inference-dispatch-") as temporary:
            root = Path(temporary)
            report = _run_coroutine(
                rehearse_abort(
                    root,
                    lambda manifest: [*launcher, str(root / "artifacts"), str(manifest)],
                )
            )
    except Exception as error:
        _record(checks, "dispatch_abort", False, f"rehearsal failed: {error}")
        return
    aborted = sum(
        result["status"] == "batch_aborted" for result in report["results"]
    )
    # Every cell in the rehearsal reports the outage, so the abort fires once
    # the threshold's worth have corroborated it and the rest are withheld.
    expected = len(report["results"]) - INFRASTRUCTURE_ABORT_THRESHOLD
    passed = report["aborted"] and not report["complete"] and aborted == expected
    _record(
        checks,
        "dispatch_abort",
        passed,
        f"{aborted}/{expected} queued cells withheld after a simulated outage",
    )


def _check_versioned_executable(
    checks: list[dict[str, Any]],
    name: str,
    executable: str,
    expected_version: str | None,
    configured_path: str | None = None,
) -> None:
    path = configured_path or shutil.which(executable)
    if path is None:
        _record(checks, name, False, f"{executable} is not on PATH")
        return
    try:
        process = subprocess.run(
            [path, "--version"], capture_output=True, text=True, timeout=10, check=False
        )
        version = (process.stdout or process.stderr).strip()
    except Exception as error:
        _record(checks, name, False, f"version probe failed: {error}")
        return
    passed = process.returncode == 0 and (expected_version is None or expected_version in version)
    expected = f", expected {expected_version}" if expected_version else ""
    _record(checks, name, passed, f"{path}: {version}{expected}")


def _check_solver_executable(
    checks: list[dict[str, Any]],
    name: str,
    environment_name: str,
    fallback: str,
    probe_arguments: tuple[str, ...],
) -> None:
    configured = os.environ.get(environment_name)
    value = configured or shutil.which(fallback)
    if not value:
        _record(
            checks,
            name,
            False,
            f"{environment_name} is unset and {fallback} is not on PATH",
        )
        return
    path = Path(value).resolve()
    if not path.is_file() or not os.access(path, os.X_OK):
        _record(checks, name, False, f"not executable: {path}")
        return
    try:
        process = subprocess.run(
            [str(path), *probe_arguments],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except Exception as error:
        _record(checks, name, False, f"probe failed for {path}: {error}")
        return
    output = (process.stdout or process.stderr).splitlines()
    detail = output[0].strip() if output else "probe produced no output"
    _record(
        checks,
        name,
        process.returncode == 0,
        f"{path}: {detail}; sha256={sha256_file(path)}",
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--schedule-dir", type=Path, required=True)
    parser.add_argument(
        "--sandbox-wrapper",
        type=Path,
        required=True,
        help="executable which launches its remaining argv in the study container/sandbox",
    )
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = preflight(args.config.resolve(), args.schedule_dir.resolve(), args.sandbox_wrapper)
    write_json(args.output, result)
    print(json.dumps({"ready": result["ready"], "checks": result["checks"]}, sort_keys=True))
    if not result["ready"]:
        raise SystemExit("pilot preflight failed")


if __name__ == "__main__":
    main()
