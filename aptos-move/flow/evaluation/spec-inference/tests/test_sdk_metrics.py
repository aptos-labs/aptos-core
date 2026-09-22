from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness.artifacts import JsonlWriter
from harness.sdk_metrics import SdkTelemetry, recording_transport, write_sdk_metrics


class SdkMetricsTest(unittest.TestCase):
    def test_retains_unknown_metrics_and_coalesces_only_payloads(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.jsonl"
            with JsonlWriter(path) as log, patch.dict(os.environ, {"CLAUDE_CODE_OAUTH_TOKEN": "test-secret"}):
                recorder = SdkTelemetry(log)
                for index in range(500):
                    recorder.observe({"type": "stream_event", "session_id": "s", "uuid": str(index), "event": {
                        "type": "content_block_delta", "index": 0,
                        "delta": {"type": "text_delta", "text": "x"},
                    }})
                recorder.observe({"type": "stream_event", "event": {
                    "type": "message_delta", "usage": {"output_tokens": 27, "new_counter": 9},
                }})
                recorder.observe({"type": "future_metric", "counter": 44, "value": "test-secret"})
                recorder.observe({"type": "stream_event", "event": {
                    "type": "content_block_delta", "index": 0,
                    "delta": {"type": "text_delta", "text": "x", "new_metric": 42},
                }})
                recorder.flush()
            lines = [json.loads(line) for line in path.read_text().splitlines()]
            chunks = [e for e in lines if e['event'] == 'sdk_stream_summary']
            self.assertEqual(sum(e['events'] for e in chunks), 500)
            self.assertEqual(sum(e['payload_characters'] for e in chunks), 500)
            raw = [e['message'] for e in lines if e['event'] == 'sdk_message']
            self.assertEqual(raw[0]['event']['usage']['new_counter'], 9)
            self.assertEqual(raw[1]['counter'], 44)
            self.assertEqual(raw[2]['event']['delta']['new_metric'], 42)
            self.assertNotIn('test-secret', path.read_text())
            self.assertLess(len(lines), 10)

    def test_query_totals_and_session_costs_do_not_double_count(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'events.jsonl'
            output = Path(directory) / 'metrics.json'
            with JsonlWriter(path) as log:
                recorder = SdkTelemetry(log)
                for session, tokens, cost in [('a', 10, 1.0), ('a', 20, 2.5), ('b', 5, 0.5)]:
                    log.emit('sdk_query_start', received_monotonic_ns=0)
                    recorder.observe({'type': 'assistant', 'session_id': session, 'message': {
                        'id': 'id-' + session, 'model': 'claude-opus-5', 'usage': {'output_tokens': tokens},
                    }})
                    recorder.observe({'type': 'result', 'session_id': session, 'usage': {
                        'output_tokens': tokens, 'cache_creation': {'ephemeral_1h_input_tokens': 12},
                    }, 'total_cost_usd': cost, 'modelUsage': {'claude-opus-5': {
                        'costUSD': cost, 'contextWindow': 1000000, 'provider': 'firstParty',
                    }}, 'extra_sdk_metric': 99})
                recorder.observe({'type': 'rate_limit_event', 'rate_limit_info': {
                    'status': 'allowed_warning', 'utilization': 0.8, 'resetsAt': 1234,
                }})
            write_sdk_metrics(path, output)
            summary = json.loads(output.read_text())
            self.assertEqual(summary['result_usage_totals']['output_tokens'], 35)
            self.assertEqual(summary['sdk_estimated_cost_usd'], 3.0)
            self.assertEqual(len(summary['api_messages_by_id']), 2)
            self.assertEqual(summary['latest_result_by_session']['a']['extra_sdk_metric'], 99)
            self.assertEqual(summary['rate_limits'][0]['message']['rate_limit_info']['utilization'], 0.8)
            self.assertEqual(len(summary['query_receipt_timings']), 3)
            self.assertIsNotNone(summary['query_receipt_timings'][0]['first_response_ms'])

    def test_missing_results_are_not_reported_as_zero_cost(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'events.jsonl'
            output = Path(directory) / 'metrics.json'
            path.write_text('')
            write_sdk_metrics(path, output)
            self.assertIsNone(json.loads(output.read_text())['sdk_estimated_cost_usd'])

    def test_interrupted_query_marks_totals_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'events.jsonl'
            output = Path(directory) / 'metrics.json'
            with JsonlWriter(path) as log:
                log.emit('sdk_query_start', received_monotonic_ns=0)
                SdkTelemetry(log).observe({'type': 'assistant', 'session_id': 's', 'message': {
                    'id': 'msg', 'usage': {'output_tokens': 7},
                }})
            write_sdk_metrics(path, output)
            summary = json.loads(output.read_text())
            self.assertFalse(summary['totals_complete'])
            self.assertEqual(summary['incomplete_query_count'], 1)
            self.assertEqual(summary['api_messages_by_id']['s:msg']['usage']['output_tokens'], 7)


class RecordingTransportTest(unittest.IsolatedAsyncioTestCase):
    async def test_observes_before_parser_and_flushes_on_stream_failure(self) -> None:
        from claude_agent_sdk import ClaudeAgentOptions
        from claude_agent_sdk._internal.transport.subprocess_cli import SubprocessCLITransport

        messages = [
            {'type': 'new_sdk_metric', 'future_field': 7},
            {'type': 'stream_event', 'event': {'type': 'content_block_delta', 'index': 0, 'delta': {'type': 'text_delta', 'text': 'partial'}}},
        ]

        async def source(_self):
            for message in messages:
                yield message
            raise RuntimeError('interrupted stream')

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'events.jsonl'
            received = []
            with JsonlWriter(path) as log, patch.object(SubprocessCLITransport, 'read_messages', source):
                transport = recording_transport(ClaudeAgentOptions(), log)
                with self.assertRaisesRegex(RuntimeError, 'interrupted stream'):
                    async for message in transport.read_messages():
                        received.append(message)
            self.assertEqual(received, messages)
            records = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertEqual(records[0]['message']['future_field'], 7)
            self.assertEqual(records[-1]['payload_characters'], 7)


if __name__ == '__main__':
    unittest.main()


class InterruptedMetricsTest(unittest.TestCase):
    def test_tolerating_a_partial_tail_does_not_hide_corrupt_middle_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            events = root / "events.jsonl"
            events.write_bytes(b'{broken}\n{"event":"ignored"}\n')
            with self.assertRaises(ValueError):
                write_sdk_metrics(events, root / "metrics.json", allow_incomplete_tail=True)
