"""Collect request-level Codex usage from its local OpenTelemetry exporter."""

from __future__ import annotations

import asyncio
from collections import Counter
import json
from pathlib import Path
import socket
from typing import Any

from .artifacts import JsonlWriter, write_json


USAGE_ATTRIBUTES = {
    "input_token_count": "input_tokens",
    "cached_token_count": "cached_input_tokens",
    "cache_write_token_count": "cache_write_input_tokens",
    "output_token_count": "output_tokens",
    "reasoning_token_count": "reasoning_output_tokens",
}
MAX_BATCH_BYTES = 16 * 1024 * 1024


class CodexOtelCollector:
    """Receive sanitized OTLP/HTTP JSON logs for one Codex attempt.

    Codex's public JSONL stream reports usage once per controller turn. Its OTel
    ``codex.sse_event`` record reports usage for each underlying
    ``response.completed`` request. Pricing thresholds apply at that latter
    boundary, so both views are retained and reconciled before a turn is
    accepted.
    """

    def __init__(self, output: Path, attempt: int):
        self.output = output
        self.attempt = attempt
        self.endpoint: str | None = None
        self.errors: list[str] = []
        self._server: asyncio.Server | None = None
        self._writer: JsonlWriter | None = None
        self._usage: list[dict[str, Any]] = []
        self._controller_turn: int | None = None

    async def __aenter__(self) -> "CodexOtelCollector":
        self._writer = JsonlWriter(self.output)
        self._server = await asyncio.start_server(
            self._receive, host="127.0.0.1", port=0, family=socket.AF_INET
        )
        address = self._server.sockets[0].getsockname()
        self.endpoint = f"http://127.0.0.1:{address[1]}/v1/logs"
        return self

    async def __aexit__(self, *_: object) -> None:
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
        if self._writer is not None:
            self._writer.close()

    def begin_turn(self, controller_turn: int) -> int:
        self._controller_turn = controller_turn
        return len(self._usage)

    def reconcile_turn(
        self, marker: int, native_usage: dict[str, Any]
    ) -> tuple[bool, str | None]:
        expected = {
            "input_tokens": int(native_usage.get("input_tokens", 0)),
            "cached_input_tokens": int(native_usage.get("cached_input_tokens", 0)),
            "cache_write_input_tokens": int(
                native_usage.get("cache_write_input_tokens", 0)
            ),
            "output_tokens": int(native_usage.get("output_tokens", 0)),
            "reasoning_output_tokens": int(
                native_usage.get("reasoning_output_tokens", 0)
            ),
        }
        records = [
            record
            for record in self._usage[marker:]
            if record["classification"] == "turn_request"
        ]
        observed = {
            field: sum(record["usage"][field] for record in records)
            for field in expected
        }
        matched = bool(records) and expected == observed and not self.errors
        detail = None
        if not matched:
            detail = (
                "request-level Codex telemetry mismatch: "
                f"expected={expected}, observed={observed}, "
                f"requests={len(records)}, collector_errors={self.errors}"
            )
        if self._writer is not None:
            self._writer.emit(
                "codex_usage_reconciliation",
                attempt=self.attempt,
                controller_turn=self._controller_turn,
                request_count=len(records),
                expected=expected,
                observed=observed,
                matched=matched,
                detail=detail,
            )
        return matched, detail

    async def _receive(
        self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter
    ) -> None:
        status = "200 OK"
        try:
            header = await reader.readuntil(b"\r\n\r\n")
            lines = header.decode("ascii").split("\r\n")
            method, path, _ = lines[0].split(" ", 2)
            headers = {
                name.strip().lower(): value.strip()
                for line in lines[1:]
                if line and (name_value := line.split(":", 1))
                for name, value in [name_value]
            }
            length = int(headers.get("content-length", "0"))
            if method != "POST" or path != "/v1/logs":
                raise ValueError(f"unexpected OTLP request {method} {path}")
            if not 0 < length <= MAX_BATCH_BYTES:
                raise ValueError(f"invalid OTLP body length {length}")
            body = await reader.readexactly(length)
            self._observe_batch(json.loads(body))
        except Exception as error:
            status = "400 Bad Request"
            self.errors.append(f"{type(error).__name__}: {error}")
        response = (
            f"HTTP/1.1 {status}\r\n"
            "Content-Type: application/json\r\n"
            "Content-Length: 2\r\n"
            "Connection: close\r\n\r\n{}"
        )
        writer.write(response.encode("ascii"))
        await writer.drain()
        writer.close()
        await writer.wait_closed()

    def _observe_batch(self, batch: dict[str, Any]) -> None:
        for resource in batch.get("resourceLogs", []):
            for scope in resource.get("scopeLogs", []):
                for log_record in scope.get("logRecords", []):
                    attributes = _attributes(log_record)
                    if not (
                        attributes.get("event.name") == "codex.sse_event"
                        and attributes.get("event.kind") == "response.completed"
                    ):
                        continue
                    effort = attributes.get("model_reasoning_effort")
                    usage = {
                        field: _integer(attributes.get(source, 0), source)
                        for source, field in USAGE_ATTRIBUTES.items()
                    }
                    record = {
                        "attempt": self.attempt,
                        "controller_turn": self._controller_turn,
                        "conversation_id": attributes.get("conversation.id"),
                        "model": attributes.get("model"),
                        "model_reasoning_effort": effort,
                        # Codex emits one zero-output WebSocket warm-up before
                        # the first real request. It has no reasoning-effort
                        # field and is excluded by reconciliation with the
                        # public turn total, while remaining visible here.
                        "classification": (
                            "turn_request" if effort is not None else "startup_warmup"
                        ),
                        "usage": usage,
                    }
                    self._usage.append(record)
                    if self._writer is not None:
                        self._writer.emit("codex_response_usage", **record)


def write_codex_request_metrics(events_path: Path, output_path: Path) -> None:
    counts: Counter[str] = Counter()
    tokens: dict[str, Counter[str]] = {
        "turn_request": Counter(),
        "startup_warmup": Counter(),
    }
    reconciliations = 0
    matched = 0
    for line in events_path.read_text(encoding="utf-8").splitlines():
        record = json.loads(line)
        event = record.get("event")
        if event == "codex_response_usage":
            classification = str(record["classification"])
            counts[classification] += 1
            for name, value in record["usage"].items():
                tokens[classification][name] += int(value)
        elif event == "codex_usage_reconciliation":
            reconciliations += 1
            matched += bool(record.get("matched"))
    write_json(
        output_path,
        {
            "schema_version": 1,
            "source": events_path.name,
            "response_counts": dict(counts),
            "token_totals": {
                kind: dict(values) for kind, values in tokens.items()
            },
            "turn_reconciliations": reconciliations,
            "matched_turn_reconciliations": matched,
            "complete": reconciliations > 0 and reconciliations == matched,
        },
    )


def _attributes(record: dict[str, Any]) -> dict[str, Any]:
    attributes: dict[str, Any] = {}
    for entry in record.get("attributes", []):
        key = entry.get("key")
        value = entry.get("value")
        if not isinstance(key, str) or not isinstance(value, dict) or len(value) != 1:
            continue
        attributes[key] = next(iter(value.values()))
    return attributes


def _integer(value: Any, name: str) -> int:
    try:
        result = int(value)
    except (TypeError, ValueError) as error:
        raise ValueError(f"invalid {name}: {value!r}") from error
    if result < 0:
        raise ValueError(f"negative {name}: {result}")
    return result
