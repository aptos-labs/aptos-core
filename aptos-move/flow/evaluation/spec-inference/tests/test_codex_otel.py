from __future__ import annotations

import asyncio
import json
from pathlib import Path
import tempfile
import unittest

from harness.codex_otel import CodexOtelCollector, write_codex_request_metrics


def _value(value: object) -> dict[str, object]:
    if isinstance(value, bool):
        return {"boolValue": value}
    return {"stringValue": str(value)}


def _completed(**attributes: object) -> dict[str, object]:
    fields = {
        "event.name": "codex.sse_event",
        "event.kind": "response.completed",
        "conversation.id": "thread-1",
        "model": "gpt-5.6-terra",
        "input_token_count": 0,
        "cached_token_count": 0,
        "cache_write_token_count": 0,
        "output_token_count": 0,
        "reasoning_token_count": 0,
        **attributes,
    }
    return {
        "attributes": [
            {"key": key, "value": _value(value)} for key, value in fields.items()
        ]
    }


def _batch(*records: dict[str, object]) -> dict[str, object]:
    return {"resourceLogs": [{"scopeLogs": [{"logRecords": list(records)}]}]}


class CodexOtelCollectorTest(unittest.TestCase):
    def test_receives_otlp_http_json(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "request-usage.jsonl"

            async def exercise() -> None:
                async with CodexOtelCollector(events, attempt=1) as collector:
                    marker = collector.begin_turn(1)
                    body = json.dumps(_batch(_completed(
                        input_token_count=10,
                        output_token_count=2,
                        reasoning_token_count=1,
                        model_reasoning_effort="high",
                    ))).encode()
                    assert collector.endpoint is not None
                    port = int(collector.endpoint.split(":")[-1].split("/")[0])
                    reader, writer = await asyncio.open_connection("127.0.0.1", port)
                    writer.write(
                        b"POST /v1/logs HTTP/1.1\r\n"
                        b"Host: 127.0.0.1\r\n"
                        b"Content-Type: application/json\r\n"
                        + f"Content-Length: {len(body)}\r\n".encode()
                        + b"Connection: close\r\n\r\n"
                        + body
                    )
                    await writer.drain()
                    response = await reader.read()
                    writer.close()
                    await writer.wait_closed()
                    self.assertIn(b"200 OK", response)
                    matched, detail = collector.reconcile_turn(marker, {
                        "input_tokens": 10,
                        "output_tokens": 2,
                        "reasoning_output_tokens": 1,
                    })
                    self.assertTrue(matched, detail)

            asyncio.run(exercise())

    def test_request_usage_reconciles_and_keeps_warmup_separate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "request-usage.jsonl"
            metrics = Path(temporary) / "request-metrics.json"

            async def exercise() -> None:
                async with CodexOtelCollector(events, attempt=1) as collector:
                    marker = collector.begin_turn(1)
                    collector._observe_batch(_batch(
                        _completed(input_token_count=8_000),
                        _completed(
                            input_token_count=11_000,
                            cached_token_count=2_000,
                            output_token_count=50,
                            reasoning_token_count=20,
                            model_reasoning_effort="high",
                        ),
                        _completed(
                            input_token_count=12_000,
                            cached_token_count=11_000,
                            output_token_count=30,
                            reasoning_token_count=10,
                            model_reasoning_effort="high",
                        ),
                    ))
                    matched, detail = collector.reconcile_turn(marker, {
                        "input_tokens": 23_000,
                        "cached_input_tokens": 13_000,
                        "cache_write_input_tokens": 0,
                        "output_tokens": 80,
                        "reasoning_output_tokens": 30,
                    })
                    self.assertTrue(matched, detail)

            asyncio.run(exercise())
            write_codex_request_metrics(events, metrics)
            summary = json.loads(metrics.read_text(encoding="utf-8"))
            self.assertTrue(summary["complete"])
            self.assertEqual(2, summary["response_counts"]["turn_request"])
            self.assertEqual(1, summary["response_counts"]["startup_warmup"])
            self.assertEqual(
                23_000,
                summary["token_totals"]["turn_request"]["input_tokens"],
            )
            self.assertEqual(
                8_000,
                summary["token_totals"]["startup_warmup"]["input_tokens"],
            )

    def test_mismatch_is_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "request-usage.jsonl"

            async def exercise() -> None:
                async with CodexOtelCollector(events, attempt=2) as collector:
                    marker = collector.begin_turn(1)
                    collector._observe_batch(_batch(_completed(
                        input_token_count=10,
                        model_reasoning_effort="high",
                    )))
                    matched, detail = collector.reconcile_turn(marker, {
                        "input_tokens": 11,
                    })
                    self.assertFalse(matched)
                    self.assertIn("telemetry mismatch", detail or "")

            asyncio.run(exercise())


if __name__ == "__main__":
    unittest.main()
