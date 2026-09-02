"""Strict JSON configuration loading without runtime dependencies."""

from __future__ import annotations

import json
from dataclasses import dataclass, fields
from pathlib import Path
from .artifacts import load_object
from typing import Any


FEEDBACK_LEVELS = ("baseline", "acceptance", "diagnostics", "progress")

ARM_TO_TACTIC = {
    "agent_only": "agent-only",
    "hybrid_guided": "hybrid-guided",
    "hybrid_flexible": "hybrid-flexible",
}


@dataclass(frozen=True)
class ExperimentConfig:
    schema_version: int
    source_commit: str
    claude_agent_sdk_version: str
    claude_code_version: str
    model: str
    provider_base_url: str
    effort: str
    feedback_level: str
    max_controller_turns: int
    max_model_turns_per_controller_turn: int
    max_wall_seconds: int
    max_output_tokens: int
    operational_timeout_seconds: int
    eventual_timeout_seconds: int
    infrastructure_retries: int
    allowed_builtin_tools: list[str]
    denied_builtin_tools: list[str]
    compile_command: list[str]
    prove_command: list[str]
    inference_command: list[str]
    check_candidate_command: list[str]

    @classmethod
    def load(cls, path: Path) -> "ExperimentConfig":
        data = load_object(path)
        _require_exact_fields(data, cls, path)
        config = cls(**data)
        if config.schema_version != 1:
            raise ValueError(f"unsupported config schema {config.schema_version}")
        if config.effort != "max":
            raise ValueError("the canonical experiment requires effort=max")
        if config.feedback_level not in FEEDBACK_LEVELS:
            raise ValueError(
                f"feedback_level must be one of {list(FEEDBACK_LEVELS)}"
            )
        if config.max_controller_turns < 1 or config.max_wall_seconds < 1:
            raise ValueError("controller and wall limits must be positive")
        return config


@dataclass(frozen=True)
class RunSpec:
    schema_version: int
    run_id: str
    task_id: str
    target: str
    arm: str
    replicate: int
    shared_package: str
    task_patch: str
    plugin_dir: str
    plugin_manifest_sha256: str
    initial_tree_sha256: str
    mutant_manifest_sha256: str
    required_contract_categories: tuple[str, ...]
    order: int = 1
    block: int = 1
    package_relpath: str = "."
    allowed_edit_paths: tuple[str, ...] = ("**/*.move",)
    round_id: str = "legacy"
    # A round that predates the feedback factor declares no level; the
    # experiment configuration supplies it in that case.
    feedback_level: str | None = None
    experiment_config_sha256: str | None = None
    # Apparatus the cell was scheduled against. The controller refuses to run
    # when what is mounted differs, so a round cannot silently change beneath
    # its own observations.
    controller_harness_sha256: str | None = None
    move_flow_sha256: str | None = None
    # Solver budget for judging and scoring this task, when the round's single
    # budget does not fit it. One target sitting at the solver boundary would
    # otherwise force the whole round to run at its timeout, which hides the
    # cost in an aggregate instead of recording it against the task.
    prove_timeout_seconds: int | None = None

    @classmethod
    def load(cls, path: Path) -> "RunSpec":
        data = load_object(path)
        known = {item.name for item in fields(cls)}
        unknown = set(data) - known
        if unknown:
            raise ValueError(f"unknown run fields in {path}: {sorted(unknown)}")
        if "allowed_edit_paths" in data:
            data["allowed_edit_paths"] = tuple(data["allowed_edit_paths"])
        if "required_contract_categories" in data:
            data["required_contract_categories"] = tuple(data["required_contract_categories"])
        spec = cls(**data)
        if spec.schema_version != 2:
            raise ValueError(f"unsupported run schema {spec.schema_version}")
        if spec.arm not in ARM_TO_TACTIC:
            raise ValueError(f"unknown arm `{spec.arm}`")
        if spec.feedback_level is not None and spec.feedback_level not in FEEDBACK_LEVELS:
            raise ValueError(f"unknown feedback level `{spec.feedback_level}`")
        if not spec.run_id or any(char not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-" for char in spec.run_id):
            raise ValueError("run_id must contain only letters, digits, '.', '_', or '-'")
        if not spec.round_id or any(
            char not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
            for char in spec.round_id
        ):
            raise ValueError(
                "round_id must contain only letters, digits, '.', '_', or '-'"
            )
        if spec.replicate < 1 or spec.block < 1 or spec.order < 1:
            raise ValueError("replicate, block, or order is out of range")
        allowed_categories = {
            "normal-result",
            "abort",
            "state-transition",
            "frame",
            "loop-invariant",
        }
        if not spec.required_contract_categories or not set(
            spec.required_contract_categories
        ) <= allowed_categories:
            raise ValueError(
                "required_contract_categories is empty or contains an unknown category"
            )
        for label, value in (
            ("plugin_manifest_sha256", spec.plugin_manifest_sha256),
            ("initial_tree_sha256", spec.initial_tree_sha256),
            ("mutant_manifest_sha256", spec.mutant_manifest_sha256),
        ):
            if len(value) != 64 or any(char not in "0123456789abcdef" for char in value):
                raise ValueError(f"{label} must be lowercase SHA-256 hex")
        if spec.experiment_config_sha256 is not None and (
            len(spec.experiment_config_sha256) != 64
            or any(
                char not in "0123456789abcdef"
                for char in spec.experiment_config_sha256
            )
        ):
            raise ValueError(
                "experiment_config_sha256 must be lowercase SHA-256 hex"
            )
        return spec

    def resolve_paths(self, manifest: Path) -> "ResolvedRunSpec":
        base = manifest.parent.resolve()
        shared_package = (base / self.shared_package).resolve()
        task_patch = (base / self.task_patch).resolve()
        plugin = (base / self.plugin_dir).resolve()
        if (
            not shared_package.is_dir()
            or not task_patch.is_file()
            or not plugin.is_dir()
        ):
            raise ValueError(
                "shared_package, task_patch, and plugin_dir must exist"
            )
        return ResolvedRunSpec(self, shared_package, task_patch, plugin)


@dataclass(frozen=True)
class ResolvedRunSpec:
    spec: RunSpec
    shared_package: Path
    task_patch: Path
    plugin_dir: Path



def _require_exact_fields(data: dict[str, Any], cls: type[Any], path: Path) -> None:
    expected = {item.name for item in fields(cls)}
    actual = set(data)
    if missing := expected - actual:
        raise ValueError(f"missing fields in {path}: {sorted(missing)}")
    if unknown := actual - expected:
        raise ValueError(f"unknown fields in {path}: {sorted(unknown)}")
