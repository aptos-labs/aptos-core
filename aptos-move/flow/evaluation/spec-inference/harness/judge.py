"""Fresh-process authority on candidate acceptance."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from .artifacts import tree_hash, write_json
from .config import ExperimentConfig


MAX_DIAGNOSTIC_BYTES = 256 * 1024

# Candidate verdicts map onto the controller's pre-registered transitions.
VERDICT_STATES = {
    "candidate_accepted": "operational_success",
    # A changed implementation or an out-of-scope edit. The follow-up for a
    # forbidden weakening already names "an out-of-scope runtime edit", so the
    # same transition applies.
    "policy_violation": "forbidden_weakening",
    "compile_failure": "compile_failure",
    "forbidden_weakening": "forbidden_weakening",
    "prover_failure": "prover_failure",
    "prover_timeout": "prover_timeout",
}

@dataclass(frozen=True)
class CommandResult:
    argv: list[str]
    returncode: int | None
    duration_ms: int
    timed_out: bool
    infrastructure_error: str | None
    stdout: str
    stderr: str

    @property
    def succeeded(self) -> bool:
        return self.returncode == 0 and not self.timed_out and not self.infrastructure_error

    @property
    def diagnostics(self) -> str:
        return _truncate("\n".join(part for part in (self.stdout, self.stderr) if part).strip())


@dataclass(frozen=True)
class JudgeResult:
    schema_version: int
    state: str
    diagnostics: str
    verdict: dict[str, Any]
    command: dict[str, Any]
    tree_sha256: str


class Judge:
    """Fresh-process authority on whether a candidate specification passes.

    Every check runs through `move-flow experiment check-candidate`, the same
    command the agent-visible candidate check calls, so an accepted candidate
    cannot be rejected here by a differently worded rule. This judge remains
    authoritative: a disagreement is an infrastructure defect, not a score.
    """

    def __init__(self, config: ExperimentConfig, artifact_dir: Path):
        self.config = config
        self.artifact_dir = artifact_dir

    async def evaluate(
        self,
        baseline: Path,
        package: Path,
        target: str,
        allowed_edit_paths: tuple[str, ...],
        required_contract_categories: tuple[str, ...],
        previous_tree_sha256: str | None,
        timeout_seconds: int | None = None,
    ) -> JudgeResult:
        timeout = timeout_seconds or self.config.operational_timeout_seconds
        current_hash = tree_hash(package)
        stamp = time.monotonic_ns()
        check_config = self.artifact_dir / f"candidate-check-{stamp}.json"
        report = self.artifact_dir / f"candidate-verdict-{stamp}.json"
        write_json(
            check_config,
            {
                "schema_version": 1,
                "baseline": str(baseline),
                "package": str(package),
                "target": target,
                "allowed_edit_paths": list(allowed_edit_paths),
                "required_contract_categories": list(required_contract_categories),
                "timeout_seconds": timeout,
                # The judge's verdict is the one recorded, so it enforces the
                # baseline comparison exactly as the agent-facing check does.
                "enforce_edit_policy": True,
            },
        )
        result = await run_command(
            render_command(
                self.config.check_candidate_command,
                config=check_config,
                output=report,
            ),
            timeout_seconds=max(timeout + 60, timeout * 4),
        )
        if result.infrastructure_error or result.timed_out or not report.is_file():
            detail = (
                result.diagnostics
                or result.infrastructure_error
                or "candidate check produced no verdict"
            )
            return JudgeResult(
                schema_version=2,
                state="infrastructure_failure",
                diagnostics=detail,
                verdict={},
                command=asdict(result),
                tree_sha256=current_hash,
            )
        verdict = json.loads(report.read_text(encoding="utf-8"))
        state = VERDICT_STATES.get(verdict.get("state", ""))
        if state is None:
            return JudgeResult(
                schema_version=2,
                state="infrastructure_failure",
                diagnostics=f"unknown candidate verdict {verdict.get('state')!r}",
                verdict=verdict,
                command=asdict(result),
                tree_sha256=current_hash,
            )
        diagnostics = verdict.get("diagnostics", "")
        if state == "operational_success" and previous_tree_sha256 == current_hash:
            state = "no_progress"
            diagnostics = "no relevant workspace change since the previous controller turn"
        return JudgeResult(
            schema_version=2,
            state=state,
            diagnostics=_truncate(diagnostics),
            verdict=verdict,
            command=asdict(result),
            tree_sha256=current_hash,
        )


async def run_command(argv: list[str], timeout_seconds: int) -> CommandResult:
    started = time.monotonic_ns()
    env = dict(os.environ)
    env["NO_COLOR"] = "1"
    try:
        process = await asyncio.create_subprocess_exec(
            *argv,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
            start_new_session=True,
        )
    except OSError as error:
        return CommandResult(argv, None, _elapsed_ms(started), False, str(error), "", "")
    try:
        stdout, stderr = await asyncio.wait_for(process.communicate(), timeout_seconds)
        return CommandResult(
            argv,
            process.returncode,
            _elapsed_ms(started),
            False,
            None,
            _decode(stdout),
            _decode(stderr),
        )
    except TimeoutError:
        try:
            os.killpg(process.pid, 15)
        except ProcessLookupError:
            pass
        try:
            stdout, stderr = await asyncio.wait_for(process.communicate(), 5)
        except TimeoutError:
            try:
                os.killpg(process.pid, 9)
            except ProcessLookupError:
                pass
            stdout, stderr = await process.communicate()
        return CommandResult(
            argv,
            process.returncode,
            _elapsed_ms(started),
            True,
            None,
            _decode(stdout),
            _decode(stderr),
        )


def render_command(template: list[str], **values: object) -> list[str]:
    rendered: list[str] = []
    substitutions = {key: str(value) for key, value in values.items()}
    for argument in template:
        rendered.append(argument.format_map(substitutions))
    return rendered


def _decode(value: bytes) -> str:
    return _truncate(value.decode("utf-8", errors="replace"))


def _truncate(value: str) -> str:
    encoded = value.encode("utf-8")
    if len(encoded) <= MAX_DIAGNOSTIC_BYTES:
        return value
    return encoded[:MAX_DIAGNOSTIC_BYTES].decode("utf-8", errors="replace") + "\n<truncated>"


def _elapsed_ms(started: int) -> int:
    return (time.monotonic_ns() - started) // 1_000_000


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--previous-tree-sha256")
    parser.add_argument("--allowed-edit-path", action="append", default=["**/*.move"])
    parser.add_argument("--required-contract-category", action="append", required=True)
    args = parser.parse_args()
    config = ExperimentConfig.load(args.config)
    judge = Judge(config, args.output.parent)
    result = asyncio.run(
        judge.evaluate(
            args.baseline.resolve(),
            args.package.resolve(),
            args.target,
            tuple(args.allowed_edit_path),
            tuple(args.required_contract_category),
            args.previous_tree_sha256,
        )
    )
    write_json(args.output, asdict(result))


if __name__ == "__main__":
    main()
