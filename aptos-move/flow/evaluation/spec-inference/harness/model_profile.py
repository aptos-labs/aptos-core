"""Select a model before scheduling, and launch with its matching credentials."""

from __future__ import annotations

import argparse
from dataclasses import asdict, replace
import json
import os
from pathlib import Path
import shutil
import subprocess

from .config import ExperimentConfig


PROFILES = {
    "glm": ("glm-5.3[1m]", "https://api.z.ai/api/anthropic"),
    "opus": ("claude-opus-5", "https://api.anthropic.com"),
    "sonnet": ("claude-sonnet-5", "https://api.anthropic.com"),
    "sol56": ("gpt-5.6-sol", "https://chatgpt.com/backend-api"),
}
SUBSCRIPTION_PROFILES = {PROFILES["opus"], PROFILES["sonnet"]}
CODEX_CLI_VERSION = "0.153.2"
ROOT = Path(__file__).resolve().parent.parent


def select_model(
    base: Path, output: Path, model: str, source_commit: str | None = None
) -> None:
    base_config = ExperimentConfig.load(base)
    config = replace(
        base_config,
        model=PROFILES[model][0],
        provider_base_url=PROFILES[model][1],
        effort=(
            "xhigh" if model in ("opus", "sonnet") else
            "high" if model == "sol56" else "max"
        ),
        agent_runtime="codex" if model == "sol56" else "claude",
        codex_cli_version=CODEX_CLI_VERSION if model == "sol56" else None,
        source_commit=source_commit or base_config.source_commit,
        allowed_builtin_tools=(
            ["shell", "apply_patch"]
            if model == "sol56"
            else base_config.allowed_builtin_tools
        ),
        denied_builtin_tools=(
            ["web_search", "agents"]
            if model == "sol56"
            else base_config.denied_builtin_tools
        ),
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    # A scheduled configuration is immutable; never overwrite an existing one.
    with output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(asdict(config), indent=2) + "\n")


def subscription_environment(config: ExperimentConfig, environment: dict[str, str]) -> dict[str, str]:
    env = dict(environment)
    token = env.get("CLAUDE_CODE_OAUTH_TOKEN", "")
    if not token and (token_file := env.get("MOVE_INFERENCE_CLAUDE_TOKEN_FILE")):
        token = Path(token_file).read_text(encoding="utf-8").strip()
    if not token:
        raise ValueError(
            "subscription token missing: run `claude setup-token` locally and set "
            "CLAUDE_CODE_OAUTH_TOKEN or MOVE_INFERENCE_CLAUDE_TOKEN_FILE"
        )
    # API keys and alternate providers take precedence over subscription OAuth.
    for name in list(env):
        if name.startswith(("ANTHROPIC_", "CLAUDE_CODE_USE_")) or name in (
            "ZAI_API_KEY", "GLM_TOKEN", "GML_TOKEN", "MOONSHOT_API_KEY",
            "CLAUDE_CODE_SUBAGENT_MODEL", "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            "MOVE_INFERENCE_CLAUDE_TOKEN_FILE",
        ):
            env.pop(name)
    env.update({
        "CLAUDE_CODE_OAUTH_TOKEN": token,
        "ANTHROPIC_BASE_URL": config.provider_base_url,
        "ANTHROPIC_MODEL": config.model,
        "CLAUDE_CODE_EFFORT_LEVEL": config.effort,
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
        "ENABLE_TOOL_SEARCH": "false",
    })
    return env


def launch(config_path: Path, command: list[str]) -> None:
    config = ExperimentConfig.load(config_path)
    pair = (config.model, config.provider_base_url)
    if config.agent_runtime == "codex":
        if pair != PROFILES["sol56"]:
            raise ValueError("no Codex credential profile for this model/endpoint pair")
        executable = os.environ.get("MOVE_INFERENCE_CODEX_EXECUTABLE") or shutil.which("codex")
        if not executable:
            raise ValueError("Codex executable not found")
        version_output = subprocess.run(
            [executable, "--version"], capture_output=True, text=True, timeout=15,
            check=True,
        ).stdout.strip()
        version = version_output.removeprefix("codex-cli ")
        if version != config.codex_cli_version:
            raise ValueError(
                f"expected Codex CLI {config.codex_cli_version}, found {version_output}"
            )
        auth_file = Path(
            os.environ.get("MOVE_INFERENCE_CODEX_AUTH_FILE", Path.home() / ".codex/auth.json")
        ).resolve()
        if not auth_file.is_file():
            raise ValueError(
                "Codex authentication missing: run `codex login` or set "
                "MOVE_INFERENCE_CODEX_AUTH_FILE"
            )
        env = dict(os.environ)
        env["MOVE_INFERENCE_CODEX_EXECUTABLE"] = str(Path(executable).resolve())
        env["MOVE_INFERENCE_CODEX_AUTH_FILE"] = str(auth_file)
        if command == ["--preflight"]:
            print(
                f"Codex authentication resolved; Codex CLI {version}; model {config.model}"
            )
            return
        os.execvpe(command[0], command, env)
    if pair == PROFILES["glm"]:
        env = dict(os.environ)
        env.pop("CLAUDE_CODE_OAUTH_TOKEN", None)
        env["MOVE_INFERENCE_CLAUDE_VERSION"] = config.claude_code_version
        wrapper = str(ROOT / "sandbox/with-glm-env.sh")
        os.execve(wrapper, [wrapper, *command], env)
    if pair not in SUBSCRIPTION_PROFILES:
        raise ValueError("no credential profile for this model/endpoint pair")
    env = subscription_environment(config, dict(os.environ))
    versioned = Path.home() / ".local/share/claude/versions" / config.claude_code_version
    executable = env.get("MOVE_INFERENCE_CLAUDE_EXECUTABLE") or (
        str(versioned) if versioned.is_file() else shutil.which("claude")
    )
    if not executable:
        raise ValueError("Claude Code executable not found")
    version = subprocess.run(
        [executable, "--version"], capture_output=True, text=True, timeout=15,
        check=True, env=env,
    ).stdout.split()[0]
    if version != config.claude_code_version:
        raise ValueError(f"expected Claude Code {config.claude_code_version}, found {version}")
    env["CLAUDE_CODE_EXECUTABLE"] = executable
    if command == ["--preflight"]:
        print(f"subscription token resolved; Claude Code {version}; model {config.model}")
        return
    os.execvpe(command[0], command, env)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    select = sub.add_parser("select")
    select.add_argument("--model", choices=PROFILES, required=True)
    select.add_argument("--config", type=Path, required=True)
    select.add_argument("--output", type=Path, required=True)
    select.add_argument("--source-commit")
    run = sub.add_parser("exec")
    run.add_argument("--config", type=Path, required=True)
    run.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    try:
        if args.action == "select":
            select_model(
                args.config, args.output, args.model, source_commit=args.source_commit
            )
        else:
            command = args.command[1:] if args.command[:1] == ["--"] else args.command
            if not command:
                parser.error("exec requires a command after --")
            launch(args.config, command)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f"model profile: {error}\n")


if __name__ == "__main__":
    main()
