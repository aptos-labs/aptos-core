import json
import unittest

from ci_actions.github import ActionError, HttpResponse
from ci_actions.indexer_dispatch import DispatchResult, dispatch_and_poll, expected_run_name, main, parse_inputs
from tests.helpers import SHA, local_server, route_client, run_command, run_main

CORRELATION = "123e4567-e89b-42d3-a456-426614174000"
DOWNSTREAM = "/repos/aptos-labs/aptos-indexer-processors"
LIST = f"{DOWNSTREAM}/actions/runs?event=repository_dispatch&branch=main&per_page=100"
RUN_URL = "https://github.test/aptos-labs/aptos-indexer-processors/actions/runs/777"
ENV = {
    "INPUT_DOWNSTREAM_REPOSITORY": "aptos-labs/aptos-indexer-processors",
    "INPUT_EVENT_TYPE": "test-txn-json-change-detected",
    "INPUT_APPROVED_SHA": SHA,
    "INPUT_PR_NUMBER": "42",
    "INPUT_SOURCE_RUN_ID": "9001",
    "INPUT_SOURCE_RUN_ATTEMPT": "3",
    "INPUT_DOWNSTREAM_BRANCH": "main",
    "INPUT_DISCOVERY_ATTEMPTS": "1",
    "INPUT_COMPLETION_ATTEMPTS": "1",
    "INPUT_POLL_INTERVAL_SECONDS": "0",
}
INPUTS = parse_inputs(ENV)
RUN_NAME = expected_run_name(INPUTS, CORRELATION)


def run(run_id=777, **overrides):
    return {"id": run_id, "event": "repository_dispatch", "head_branch": "main", "display_title": RUN_NAME,
            "status": "completed", "conclusion": "success", **overrides}


def downstream(routes):
    return route_client({f"{DOWNSTREAM}/dispatches": HttpResponse(204, {}, b""), **routes},
                        repository="aptos-labs/aptos-indexer-processors")


def dispatch(client):
    return dispatch_and_poll(client, INPUTS, new_correlation=lambda: CORRELATION, sleep=lambda _: None)


class DispatchTests(unittest.TestCase):
    def test_binds_the_exact_source_and_completes_only_the_correlated_run(self):
        client, transport = downstream({
            LIST: {"workflow_runs": [run(status="in_progress", conclusion=None, html_url=RUN_URL)]},
            f"{DOWNSTREAM}/actions/runs/777": run(html_url=RUN_URL),
        })
        result = dispatch(client)
        self.assertEqual(result, DispatchResult(CORRELATION, 777, RUN_URL))
        self.assertEqual(json.loads(transport.requests[0]["body"]), {
            "event_type": "test-txn-json-change-detected",
            "client_payload": {"commit_hash": SHA, "pr_number": 42, "source_run_id": 9001, "source_run_attempt": 3,
                               "correlation": CORRELATION, "expected_run_name": RUN_NAME},
        })
        self.assertEqual(RUN_NAME, f"indexer-pr-ci:{SHA}:pr:42:source:9001:3:correlation:{CORRELATION}")
        self.assertEqual(transport.urls()[1], LIST)
        self.assertEqual(len(transport.requests), 3)
        self.assertTrue(all(request["authorization"] == "Bearer test-token" for request in transport.requests))

    def test_fails_closed_when_the_correlation_is_not_echoed(self):
        client, transport = downstream(
            {LIST: {"workflow_runs": [run(778, display_title=RUN_NAME.replace(CORRELATION, "wrong"))]}})
        with self.assertRaisesRegex(ActionError, "did not echo the exact correlation"):
            dispatch(client)
        self.assertEqual(len(transport.requests), 2)

    def test_rejects_ambiguous_correlated_runs(self):
        with self.assertRaisesRegex(ActionError, "Multiple downstream runs"):
            dispatch(downstream(
                {LIST: {"workflow_runs": [run(779, status="queued"), run(780, status="queued")]}})[0])

    def test_revalidates_correlation_on_the_selected_run(self):
        with self.assertRaisesRegex(ActionError, "lost exact source correlation"):
            dispatch(downstream({
                LIST: {"workflow_runs": [run(781, status="in_progress", conclusion=None)]},
                f"{DOWNSTREAM}/actions/runs/781": run(781, display_title="changed-after-discovery"),
            })[0])

    def test_propagates_downstream_failure(self):
        failed = run(782, conclusion="failure")
        with self.assertRaisesRegex(ActionError, "completed with conclusion failure"):
            dispatch(downstream({LIST: {"workflow_runs": [failed]}, f"{DOWNSTREAM}/actions/runs/782": failed})[0])

    def test_rejects_malformed_runs(self):
        for bad in (run(0), run(status="unknown"), run(conclusion=None), run(html_url="http://x"),
                    run(html_url=None), run(event="push"), run(head_branch="other")):
            with self.subTest(run=bad), self.assertRaises(ActionError):
                dispatch(downstream({LIST: {"workflow_runs": [bad]}})[0])
        with self.assertRaisesRegex(ActionError, "run list is malformed"):
            dispatch(downstream({LIST: {"runs": []}})[0])

    def test_times_out_when_the_run_does_not_complete(self):
        pending = run(status="in_progress", conclusion=None)
        with self.assertRaisesRegex(ActionError, "did not complete"):
            dispatch(downstream({LIST: {"workflow_runs": [pending]}, f"{DOWNSTREAM}/actions/runs/777": pending})[0])

    def test_http_failures_expose_neither_token_nor_body(self):
        body = b"test-token and internal response details"
        with self.assertRaises(ActionError) as caught:
            dispatch(downstream({f"{DOWNSTREAM}/dispatches": HttpResponse(500, {}, body)})[0])
        self.assertIn("HTTP 500", str(caught.exception))
        self.assertNotIn("test-token", str(caught.exception))
        self.assertNotIn("internal", str(caught.exception))


class InputTests(unittest.TestCase):
    def test_rejects_malformed_identity_inputs(self):
        for overrides in (
            {"INPUT_APPROVED_SHA": "not-a-sha"}, {"INPUT_APPROVED_SHA": SHA.upper()},
            {"INPUT_PR_NUMBER": "-1"}, {"INPUT_SOURCE_RUN_ID": "0"}, {"INPUT_SOURCE_RUN_ATTEMPT": "0"},
            {"INPUT_DOWNSTREAM_REPOSITORY": "wrong"}, {"INPUT_DOWNSTREAM_BRANCH": "main branch"},
            {"INPUT_EVENT_TYPE": " padded"}, {"INPUT_DISCOVERY_ATTEMPTS": "101"},
            {"INPUT_COMPLETION_ATTEMPTS": "301"}, {"INPUT_POLL_INTERVAL_SECONDS": "3601"},
        ):
            with self.subTest(overrides=overrides), self.assertRaises(ActionError):
                parse_inputs({**ENV, **overrides})

    def test_accepts_pr_number_zero_for_manual_dispatch(self):
        self.assertEqual(parse_inputs({**ENV, "INPUT_PR_NUMBER": "0"}).pr_number, 0)


class IndexerDispatchCommandTests(unittest.TestCase):
    def test_invalid_inputs_fail_before_any_request(self):
        code, stderr, outputs, _ = run_command("indexer-processor-dispatch", {
            **ENV, "INPUT_APPROVED_SHA": "not-a-sha", "INPUT_TOKEN": "t", "GITHUB_API_URL": "https://api.github.test",
        })
        self.assertEqual(code, 1)
        self.assertIn("indexer-processor-dispatch failed closed: approved_sha", stderr)
        self.assertEqual(outputs, {})

    def test_uses_the_input_token_against_the_downstream_repository(self):
        def echoed_run(**overrides):
            # The first request is the dispatch; the downstream run echoes its run name.
            payload = json.loads(requests[0]["body"])["client_payload"]
            return run(display_title=payload["expected_run_name"], html_url=RUN_URL, **overrides)

        routes = {
            f"{DOWNSTREAM}/dispatches": lambda handler: (handler.send_response(204), handler.end_headers()),
            LIST: lambda handler: handler.send_json(200, {"workflow_runs": [echoed_run(status="queued")]}),
            f"{DOWNSTREAM}/actions/runs/777": lambda handler: handler.send_json(200, echoed_run()),
        }
        with local_server(routes) as (base, requests):
            code, stderr, outputs, _ = run_main("indexer-processor-dispatch", main, {
                **ENV, "INPUT_TOKEN": "dispatch-token", "GITHUB_API_URL": base,
            })
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs["downstream_run_id"], "777")
        self.assertEqual(outputs["downstream_run_url"], RUN_URL)
        self.assertRegex(outputs["correlation"], r"^[0-9a-f-]{36}$")
        self.assertTrue(all(request["authorization"] == "Bearer dispatch-token" for request in requests))


if __name__ == "__main__":
    unittest.main()
