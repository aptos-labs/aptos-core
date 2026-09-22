"""Retain CLI telemetry before SDK parsing, without per-token text duplication."""

from __future__ import annotations

from collections import Counter
import json
from pathlib import Path
import time
from typing import Any

from .artifacts import JsonlWriter, write_json
from .credentials import redact


class SdkTelemetry:
    def __init__(self, log: JsonlWriter):
        self.log = log
        self.pending: dict[str, Any] | None = None

    def observe(self, message: dict[str, Any]) -> None:
        event = message.get("event")
        event = event if isinstance(event, dict) else {}
        delta = event.get("delta")
        delta = delta if isinstance(delta, dict) else {}
        payload_key = {
            "text_delta": "text", "thinking_delta": "thinking",
            "input_json_delta": "partial_json", "signature_delta": "signature",
        }.get(delta.get("type"))
        # Only coalesce the known payload-only shapes. New fields, including
        # future metrics, are retained verbatim rather than silently discarded.
        payload_only = (
            message.get("type") == "stream_event"
            and event.get("type") == "content_block_delta"
            and payload_key is not None
            and set(delta) <= {"type", payload_key}
            and set(event) <= {"type", "index", "delta"}
            and set(message) <= {"type", "uuid", "session_id", "parent_tool_use_id", "event"}
        )
        thinking = message.get("type") == "system" and message.get("subtype") == "thinking_tokens"
        if payload_only or thinking:
            key = (
                message.get("session_id"), message.get("parent_tool_use_id"),
                event.get("index"), delta.get("type") if payload_only else "thinking_tokens",
            )
            if self.pending is not None and self.pending["key"] != key:
                self.flush()
            now = time.monotonic_ns()
            if self.pending is None:
                self.pending = {
                    "key": key, "events": 0, "payload_characters": 0,
                    "first_received_monotonic_ns": now,
                    "first_uuid": message.get("uuid"),
                }
            self.pending["events"] += 1
            self.pending["last_received_monotonic_ns"] = now
            self.pending["last_uuid"] = message.get("uuid")
            if payload_only:
                self.pending["payload_characters"] += len(delta.get(payload_key) or "")
            else:
                # Preserve every distinct metric value in a compact record:
                # the counter is an estimate, not an independently billed token.
                # Unknown thinking metadata should still be available verbatim.
                known = {"type", "subtype", "session_id", "uuid", "estimated_tokens", "estimated_tokens_delta"}
                if set(message) - known:
                    self.log.emit("sdk_message", message=redact(message))
                self.pending.setdefault("first_counter", message.get("estimated_tokens"))
                self.pending["last_counter"] = message.get("estimated_tokens")
                self.pending["estimated_tokens_delta_sum"] = (
                    self.pending.get("estimated_tokens_delta_sum", 0)
                    + (message.get("estimated_tokens_delta") or 0)
                )
            # Bound how much receipt timing can be lost on abrupt termination.
            if self.pending["events"] >= 256:
                self.flush()
            return
        self.flush()
        self.log.emit("sdk_message", received_monotonic_ns=time.monotonic_ns(), message=redact(message))

    def flush(self) -> None:
        if self.pending is not None:
            self.log.emit("sdk_stream_summary", **redact(self.pending))
            self.pending = None


def recording_transport(options: Any, log: JsonlWriter) -> Any:
    # Pinned SDK implementation: the public custom-transport argument lets us
    # observe stdout before parse_message drops unknown top-level fields/types.
    from claude_agent_sdk._internal.transport.subprocess_cli import SubprocessCLITransport

    telemetry = SdkTelemetry(log)

    async def empty_prompt():
        if False:
            yield {}

    class RecordingTransport(SubprocessCLITransport):
        async def read_messages(self):
            try:
                async for message in super().read_messages():
                    telemetry.observe(message)
                    yield message
            finally:
                telemetry.flush()

    return RecordingTransport(prompt=empty_prompt(), options=options)


def write_sdk_metrics(events_path: Path, output_path: Path, *, allow_incomplete_tail: bool = False) -> None:
    """Summarize known counters; the original records retain all extra fields."""
    results: list[dict[str, Any]] = []
    rate_limits: list[dict[str, Any]] = []
    sessions: dict[str, dict[str, Any]] = {}
    requests: dict[str, dict[str, Any]] = {}
    counts: Counter = Counter()
    tokens: Counter = Counter()
    queries: list[dict[str, Any]] = []
    query: dict[str, Any] | None = None
    lines = events_path.read_bytes().splitlines(keepends=True)
    truncated_tail_bytes = 0
    for index, line in enumerate(lines):
        try:
            event = json.loads(line)
        except (ValueError, UnicodeDecodeError):
            if allow_incomplete_tail and index == len(lines) - 1 and not line.endswith(b"\n"):
                truncated_tail_bytes = len(line)
                break
            raise
        if event.get("event") == "sdk_query_start":
            query = {"started_monotonic_ns": event["received_monotonic_ns"], "first_response_ms": None, "result_received_ms": None}
            queries.append(query)
            continue
        if event.get("event") != "sdk_message":
            continue
        message = event["message"]
        kind = message.get("type", "unknown")
        counts[kind] += 1
        if query is not None and kind in ("assistant", "stream_event", "result"):
            elapsed = (event["received_monotonic_ns"] - query["started_monotonic_ns"]) / 1_000_000
            if query["first_response_ms"] is None:
                query["first_response_ms"] = elapsed
            if kind == "result":
                query["result_received_ms"] = elapsed
                query = None
        if kind == "result":
            # Retain all native result fields, including extensions not yet
            # understood by the typed SDK or this summary.
            results.append({"utc_ms": event["utc_ms"], "result": message})
            sessions[message.get("session_id") or f"unknown-{len(results)}"] = message
            for name in ("input_tokens", "output_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"):
                value = (message.get("usage") or {}).get(name)
                if isinstance(value, int):
                    tokens[name] += value
        elif kind == "rate_limit_event":
            rate_limits.append({"utc_ms": event["utc_ms"], "message": message})
        elif kind == "assistant":
            assistant = message.get("message") or {}
            if identifier := assistant.get("id"):
                # One API message may arrive in several assistant chunks.
                # Preserve its final observed usage once, never sum chunks.
                key = f"{message.get('session_id')}:{identifier}"
                requests[key] = {"utc_ms": event["utc_ms"], **assistant}
    costs = [result.get("total_cost_usd") for result in sessions.values()]
    write_json(output_path, redact({
        "schema_version": 1,
        "source": events_path.name,
        "coverage": "CLI stdout before SDK parsing; no hidden HTTP headers or account billing data",
        "usage_scope": "result usage is per query; modelUsage, cost and API duration are cumulative per session",
        "cost_semantics": "SDK-reported API-equivalent estimate, not a subscription charge",
        "message_counts": dict(counts),
        "result_count": len(results),
        "incomplete_query_count": sum(q["result_received_ms"] is None for q in queries),
        "totals_complete": bool(results) and not truncated_tail_bytes and all(q["result_received_ms"] is not None for q in queries),
        "truncated_tail_bytes": truncated_tail_bytes,
        "result_usage_totals": dict(tokens),
        "sdk_estimated_cost_usd": sum(costs) if costs and all(isinstance(c, (int, float)) for c in costs) else None,
        "latest_result_by_session": sessions,
        "results": results,
        "api_messages_by_id": requests,
        "rate_limits": rate_limits,
        "query_receipt_timings": queries,
    }))
