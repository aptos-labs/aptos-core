"""Provider adapter and deterministic fake agent for controller tests."""

from __future__ import annotations

import asyncio
import dataclasses
import importlib.metadata
import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Protocol

from .artifacts import JsonlWriter
from .config import ExperimentConfig
from .credentials import redact as redact_credentials


@dataclass(frozen=True)
class AgentTurn:
    result: str
    is_error: bool
    session_id: str | None
    duration_ms: int
    duration_api_ms: int
    num_turns: int
    usage: dict[str, Any]
    model_usage: dict[str, Any]
    total_cost_usd: float | None
    system_init: dict[str, Any] | None
    terminal_reason: str | None
    api_error_status: int | None


class AgentSession(Protocol):
    async def __aenter__(self) -> "AgentSession": ...

    async def __aexit__(self, *args: object) -> None: ...

    async def send(self, prompt: str) -> AgentTurn: ...

    async def interrupt(self) -> None: ...


class ClaudeAgentSession:
    def __init__(
        self,
        config: ExperimentConfig,
        workspace: Path,
        plugin: Path,
        mcp_config: Path,
        event_log: JsonlWriter,
        stderr_sink: Callable[[str], None],
        flow_telemetry: Path,
        candidate_check: Path,
        baseline_package: Path,
        move_flow_args: str,
    ):
        try:
            installed = importlib.metadata.version("claude-agent-sdk")
        except importlib.metadata.PackageNotFoundError as error:
            raise RuntimeError(
                "claude-agent-sdk is not installed; install the `claude` optional dependency"
            ) from error
        if installed != config.claude_agent_sdk_version:
            raise RuntimeError(
                f"claude-agent-sdk version mismatch: expected {config.claude_agent_sdk_version}, got {installed}"
            )
        from claude_agent_sdk import ClaudeAgentOptions, ClaudeSDKClient

        plugin_manifest = json.loads((plugin / "move-flow-manifest.json").read_text(encoding="utf-8"))
        mcp_tools = [f"mcp__move-flow__{name}" for name in plugin_manifest["mcp_tools"]]
        cli_path = shutil.which("claude")
        if cli_path is None:
            raise RuntimeError("the pinned Claude Code executable is not on PATH")
        options = ClaudeAgentOptions(
            # Skills are loaded through the generated plugin exactly as in a
            # normal Flow session (``claude --plugin-dir <plugin>``). The
            # controller activates the selected inference skill with its slash
            # command, rather than advertising an unusable ``Skill`` tool.
            tools=config.allowed_builtin_tools,
            allowed_tools=[*config.allowed_builtin_tools, *mcp_tools],
            disallowed_tools=config.denied_builtin_tools,
            model=config.model,
            fallback_model=None,
            effort=config.effort,
            max_turns=config.max_model_turns_per_controller_turn,
            cwd=workspace,
            cli_path=cli_path,
            plugins=[{"type": "local", "path": str(plugin)}],
            mcp_servers=mcp_config,
            strict_mcp_config=True,
            setting_sources=[],
            permission_mode="dontAsk",
            env={
                "MOVE_FLOW_TELEMETRY_JSONL": str(flow_telemetry),
                # The plugin's MCP entry expands `${MOVE_FLOW_ARGS:-mcp}`
                # against this environment; without it the server silently
                # starts with no tactic, feedback level, or candidate check.
                "MOVE_FLOW_ARGS": move_flow_args,
                "CLAUDE_AGENT_SDK_CLIENT_APP": "move-spec-inference-evaluation/0.1.0",
                "ANTHROPIC_BASE_URL": config.provider_base_url,
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
                "API_TIMEOUT_MS": "3000000",
                "MOVE_INFERENCE_SANDBOX_WORKSPACE": str(workspace),
                "MOVE_INFERENCE_SANDBOX_PLUGIN": str(plugin),
                "MOVE_INFERENCE_SANDBOX_MCP_CONFIG": str(mcp_config),
                "MOVE_INFERENCE_SANDBOX_FLOW_TELEMETRY": str(flow_telemetry),
                "MOVE_INFERENCE_SANDBOX_CANDIDATE_CHECK": str(candidate_check),
                "MOVE_INFERENCE_SANDBOX_BASELINE": str(baseline_package),
            },
            stderr=stderr_sink,
        )
        self._client = ClaudeSDKClient(options=options)
        self._event_log = event_log
        self._mcp_status: dict[str, Any] | None = None
        # Session identity, not turn identity. The SDK announces `system/init`
        # once, when the client starts, so a per-response variable is `None`
        # from the first follow-up onwards -- and the controller validates
        # identity after every turn. Held on the session, it keeps saying what
        # this session is; a retry builds a new session and a new init.
        self._system_init: dict[str, Any] | None = None

    async def __aenter__(self) -> "ClaudeAgentSession":
        await self._client.__aenter__()
        self._mcp_status = normalize_sdk_value(await self._client.get_mcp_status())
        self._event_log.emit("mcp_status", status=self._mcp_status)
        return self

    async def __aexit__(self, *args: object) -> None:
        await self._client.__aexit__(*args)

    async def interrupt(self) -> None:
        await self._client.interrupt()

    async def send(self, prompt: str) -> AgentTurn:
        from claude_agent_sdk import ResultMessage, SystemMessage

        await self._client.query(prompt)
        result_message: Any | None = None
        announced_init = False
        thinking_tokens = 0
        async for message in self._client.receive_response():
            # The SDK reports extended thinking as one event per token. Logging
            # each verbatim cost 14.7 MB of a 16 MB transcript on a single hard
            # cell -- 38,270 lines carrying an incrementing counter -- plus a
            # redaction walk, a serialize and a flush apiece, in the hot path of
            # every turn. The count is kept and emitted once; the totals are in
            # `ResultMessage.usage` regardless.
            if isinstance(message, SystemMessage) and message.subtype == "thinking_tokens":
                thinking_tokens = max(
                    thinking_tokens, int(message.data.get("estimated_tokens") or 0)
                )
                continue
            normalized = redact_credentials(normalize_sdk_value(message))
            self._event_log.emit("claude_message", message=normalized)
            if isinstance(message, SystemMessage) and message.subtype == "init":
                self._system_init = dict(message.data)
                announced_init = True
            if isinstance(message, ResultMessage):
                result_message = message
        if thinking_tokens:
            self._event_log.emit("thinking_tokens", estimated_tokens=thinking_tokens)
        if result_message is None:
            raise RuntimeError("Claude Agent SDK response ended without ResultMessage")
        # The SDK can report an MCP server as `pending` immediately after
        # client startup even though the subsequent system/init event already
        # reports it connected. Refresh after the first response so runtime
        # identity validation observes the settled tool inventory.
        if announced_init:
            self._mcp_status = normalize_sdk_value(await self._client.get_mcp_status())
            self._event_log.emit("mcp_status_settled", status=self._mcp_status)
        return AgentTurn(
            result=redact_credentials(result_message.result or ""),
            is_error=result_message.is_error,
            session_id=result_message.session_id,
            duration_ms=result_message.duration_ms,
            duration_api_ms=result_message.duration_api_ms,
            num_turns=result_message.num_turns,
            usage=dict(result_message.usage or {}),
            model_usage=dict(result_message.model_usage or {}),
            total_cost_usd=result_message.total_cost_usd,
            system_init={"system": self._system_init, "mcp_status": self._mcp_status},
            terminal_reason=result_message.terminal_reason,
            api_error_status=result_message.api_error_status,
        )


class FakeAgentSession:
    """A scripted agent that can apply deterministic workspace edits per turn.

    A scripted turn may also append Flow telemetry records, which a real session
    receives from the MCP child. That makes apparatus conditions such as a
    supervisor restart reproducible without a provider.
    """

    def __init__(
        self,
        script: Path,
        workspace: Path,
        event_log: JsonlWriter,
        flow_telemetry: Path | None = None,
        attempt: int = 1,
    ):
        value = json.loads(script.read_text(encoding="utf-8"))
        # A retry builds a fresh session, so a flat script replays from its
        # first turn on every attempt. That cannot express what a retry does
        # differently from the attempt it followed -- which is where the
        # controller's per-attempt state lives -- so a script may instead give
        # one turn list per attempt. The last list covers any further attempts.
        if isinstance(value, dict):
            attempts = value.get("attempts")
            if not isinstance(attempts, list) or not attempts:
                raise ValueError("fake-agent script `attempts` must be a non-empty list")
            value = attempts[min(attempt, len(attempts)) - 1]
        if not isinstance(value, list) or not value:
            raise ValueError("fake-agent script must be a non-empty JSON list")
        self._turns: list[dict[str, Any]] = value
        self._workspace = workspace
        self._event_log = event_log
        self._flow_telemetry = flow_telemetry
        self._index = 0
        self._interrupted = asyncio.Event()

    async def __aenter__(self) -> "FakeAgentSession":
        return self

    async def __aexit__(self, *args: object) -> None:
        return None

    async def interrupt(self) -> None:
        self._interrupted.set()

    async def send(self, prompt: str) -> AgentTurn:
        if self._index >= len(self._turns):
            turn: dict[str, Any] = {"result": "No scripted turn remains."}
        else:
            turn = self._turns[self._index]
        self._index += 1
        for relative, content in turn.get("writes", {}).items():
            target = (self._workspace / relative).resolve()
            if not target.is_relative_to(self._workspace.resolve()):
                raise ValueError(f"fake-agent write escapes workspace: {relative}")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")
        for relative in turn.get("deletes", []):
            target = (self._workspace / relative).resolve()
            if not target.is_relative_to(self._workspace.resolve()):
                raise ValueError(f"fake-agent delete escapes workspace: {relative}")
            target.unlink(missing_ok=True)
        if turn.get("hang", False):
            # Model a turn still in flight at the wall deadline: the turn
            # completes only when the controller interrupts the session.
            await self._interrupted.wait()
            self._interrupted.clear()
        flow_events = turn.get("flow_events", [])
        if flow_events:
            if self._flow_telemetry is None:
                raise ValueError("fake-agent script emits Flow telemetry without a sink")
            with self._flow_telemetry.open("a", encoding="utf-8") as stream:
                for record in flow_events:
                    stream.write(json.dumps(record, sort_keys=True) + "\n")
        self._event_log.emit("fake_agent_turn", prompt=prompt, scripted_turn=self._index)
        return AgentTurn(
            result=str(turn.get("result", "")),
            is_error=bool(turn.get("is_error", False)),
            session_id="00000000-0000-0000-0000-000000000000",
            duration_ms=int(turn.get("duration_ms", 1)),
            duration_api_ms=int(turn.get("duration_api_ms", 1)),
            num_turns=int(turn.get("num_turns", 1)),
            usage=dict(turn.get("usage", {"input_tokens": 1, "output_tokens": 1})),
            model_usage=dict(turn.get("model_usage", {"fake": {"inputTokens": 1, "outputTokens": 1}})),
            total_cost_usd=turn.get("total_cost_usd", 0.0),
            system_init=dict(turn.get("system_init", {"system": {"model": "fake", "claude_code_version": "fake", "tools": []}, "mcp_status": {"mcpServers": [{"name": "move-flow", "status": "connected", "tools": []}]}})),
            terminal_reason=turn.get("terminal_reason", "completed"),
            api_error_status=turn.get("api_error_status"),
        )


def normalize_sdk_value(value: Any) -> Any:
    if dataclasses.is_dataclass(value):
        return {"type": type(value).__name__, **normalize_sdk_value(dataclasses.asdict(value))}
    if isinstance(value, dict):
        return {str(key): normalize_sdk_value(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [normalize_sdk_value(item) for item in value]
    if isinstance(value, Path):
        return str(value)
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    return repr(value)
