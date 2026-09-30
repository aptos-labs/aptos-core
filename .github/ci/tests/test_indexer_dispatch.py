import json
import string
import urllib.parse
from dataclasses import replace
import unittest
from unittest import mock

from hypothesis import given, strategies as st

from tests.property_support import POSITIVE_ID, REPOSITORY, SHA as PROPERTY_SHA, configure_profiles

configure_profiles()

from ci_actions.github import ActionError, GitHubClient, HttpResponse
from ci_actions.indexer_dispatch import _validate_run, DispatchResult, dispatch_and_poll, expected_run_name, main, parse_inputs
from tests.helpers import json_response, SHA, local_server, route_client, run_command, run_main

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
    def test_unhashable_run_status_fails_with_action_error(self):
        for status in ([], {}):
            with self.subTest(status=status):
                try:
                    dispatch(downstream({LIST: {"workflow_runs": [run(status=status)]}})[0])
                except Exception as error:
                    self.assertIsInstance(error, ActionError, "Malformed run status must produce ActionError")
                else:
                    self.fail("Malformed run status was accepted")

    def test_rejects_malformed_https_run_urls(self):
        for value in ("https://", "https:///run", "https://host:bad/run", "https://host:65536/run",
                      "https://[invalid]/run", "https://@host/run", "https://:@host/run", "https://host/a b"):
            bad = run(html_url=value)
            client, transport = downstream({LIST: {"workflow_runs": [bad]},
                                           f"{DOWNSTREAM}/actions/runs/777": bad})
            with self.subTest(value=value):
                with self.assertRaises(ActionError):
                    dispatch(client)
                self.assertEqual(len(transport.requests), 2)

    def test_completion_id_must_equal_the_discovered_id(self):
        client, transport = downstream({
            LIST: {"workflow_runs": [run(777)]},
            f"{DOWNSTREAM}/actions/runs/777": run(778),
        })
        with self.assertRaises(ActionError):
            dispatch(client)
        self.assertEqual(transport.urls()[-1], f"{DOWNSTREAM}/actions/runs/777")

    def test_accepts_enterprise_https_run_url_with_port_and_query(self):
        url = "https://github.enterprise.test:8443/org/repo/actions/runs/777?view=full#logs"
        client, _ = downstream({LIST: {"workflow_runs": [run(html_url=url)]},
                               f"{DOWNSTREAM}/actions/runs/777": run(html_url=url)})
        self.assertEqual(dispatch(client).run_url, url)

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


_DISPATCH_TEXT = st.text(alphabet=string.ascii_letters + string.digits + "._:/-", min_size=1, max_size=32)
_PENDING = ("queued", "in_progress", "waiting", "requested", "pending")
_FAILURES = ("failure", "neutral", "cancelled", "skipped", "timed_out", "action_required", "stale", "unknown")


class IndexerDispatchProperties(unittest.TestCase):
    @given(sha=PROPERTY_SHA, repository=REPOSITORY, branch=_DISPATCH_TEXT, event_type=_DISPATCH_TEXT,
           run_id=POSITIVE_ID, attempt=POSITIVE_ID, pr=st.integers(0, 2**53 - 1),
           discovery=st.integers(1, 4), completion=st.integers(1, 4), interval=st.integers(0, 3600))
    def test_input_identity_and_rejection_categories(self, sha, repository, branch, event_type, run_id, attempt,
                                                     pr, discovery, completion, interval):
        env = {**ENV, "INPUT_APPROVED_SHA": sha, "INPUT_DOWNSTREAM_REPOSITORY": repository,
               "INPUT_DOWNSTREAM_BRANCH": branch, "INPUT_EVENT_TYPE": event_type, "INPUT_PR_NUMBER": str(pr),
               "INPUT_SOURCE_RUN_ID": str(run_id), "INPUT_SOURCE_RUN_ATTEMPT": str(attempt),
               "INPUT_DISCOVERY_ATTEMPTS": str(discovery), "INPUT_COMPLETION_ATTEMPTS": str(completion),
               "INPUT_POLL_INTERVAL_SECONDS": str(interval)}
        inputs = parse_inputs(env)
        self.assertEqual((inputs.repository, inputs.approved_sha, inputs.branch, inputs.event_type),
                         (repository, sha, branch, event_type))
        self.assertEqual((inputs.pr_number, inputs.source_run_id, inputs.source_run_attempt,
                          inputs.discovery_attempts, inputs.completion_attempts, inputs.poll_interval_seconds),
                         (pr, run_id, attempt, discovery, completion, interval))
        for key, maximum, zero in (("INPUT_PR_NUMBER", 2**53 - 1, True), ("INPUT_SOURCE_RUN_ID", 2**53 - 1, False),
                                   ("INPUT_SOURCE_RUN_ATTEMPT", 2**53 - 1, False), ("INPUT_DISCOVERY_ATTEMPTS", 100, False),
                                   ("INPUT_COMPLETION_ATTEMPTS", 300, False), ("INPUT_POLL_INTERVAL_SECONDS", 3600, True)):
            parse_inputs({**env, key: str(maximum)})
            if zero:
                parse_inputs({**env, key: "0"})
            for bad in (None, True, 1, "", "+1", "-1", "01", "1.0", " 1", "1\n", "١", str(maximum + 1)) + (() if zero else ("0",)):
                with self.assertRaises(ActionError):
                    parse_inputs({**env, key: bad})
        for bad in (None, "", sha[:-1], sha + "0", ("a" + sha[1:]).upper(), "g" + sha[1:], sha + "\r"):
            with self.assertRaises(ActionError):
                parse_inputs({**env, "INPUT_APPROVED_SHA": bad})
        for key in ("INPUT_DOWNSTREAM_BRANCH", "INPUT_EVENT_TYPE"):
            for bad in (None, 1, "", " padded", "trailing ", "a b", "a+b", "x\r", "x\n", "é"):
                with self.assertRaises(ActionError):
                    parse_inputs({**env, key: bad})
        for bad in ("wrong", "a/b/c", "../b", "a/..", "a b/c", "a/b\n", "@/b"):
            with self.assertRaises(ActionError):
                parse_inputs({**env, "INPUT_DOWNSTREAM_REPOSITORY": bad})

    @given(data=st.data(), discovery=st.integers(1, 4), completion=st.integers(1, 4), interval=st.integers(0, 3),
           selected_id=POSITIVE_ID, sha=PROPERTY_SHA, correlation=st.uuids(version=4).map(str), branch=_DISPATCH_TEXT,
           pr=st.integers(0, 2**53 - 1), source_id=POSITIVE_ID, source_attempt=POSITIVE_ID)
    def test_dispatch_polling_sequence_model(self, data, discovery, completion, interval, selected_id, sha, correlation,
                                            branch, pr, source_id, source_attempt):
        inputs = replace(INPUTS, approved_sha=sha, branch=branch, pr_number=pr, source_run_id=source_id,
                         source_run_attempt=source_attempt, discovery_attempts=discovery,
                         completion_attempts=completion, poll_interval_seconds=interval)
        name = f"indexer-pr-ci:{sha}:pr:{pr}:source:{source_id}:{source_attempt}:correlation:{correlation}"
        self.assertEqual(expected_run_name(inputs, correlation), name)
        misses = data.draw(st.integers(0, discovery), label="discovery misses")
        pending = data.draw(st.integers(0, completion), label="completion pending polls")
        conclusion = data.draw(st.sampled_from(("success", *_FAILURES)), label="terminal conclusion")
        def record(**overrides):
            return {"id": selected_id, "event": "repository_dispatch", "head_branch": branch,
                    "display_title": name, "status": "completed", "conclusion": "success", **overrides}
        script = [("POST", "/dispatches", HttpResponse(204, {}, b""))]
        for _ in range(misses):
            script.append(("GET", "/actions/runs", json_response({"workflow_runs": [None, {"display_title": name + "-other"}]})))
        found = misses < discovery
        url = "https://enterprise.test:8443/org/repo/actions/runs/" + str(selected_id)
        if found:
            script.append(("GET", "/actions/runs", json_response({"workflow_runs": [record(status="queued", conclusion=None)]})))
            for _ in range(pending):
                status = data.draw(st.sampled_from(_PENDING), label="pending status")
                script.append(("GET", f"/actions/runs/{selected_id}", json_response(record(status=status, conclusion=None))))
            if pending < completion:
                script.append(("GET", f"/actions/runs/{selected_id}", json_response(record(conclusion=conclusion, html_url=url))))
        requests = []
        def transport(request, max_bytes, follow_redirect):
            # An unexpected request is AssertionError, never a qualifying fail-closed result.
            self.assertLess(len(requests), len(script))
            method, path, response = script[len(requests)]
            parsed = urllib.parse.urlsplit(request.full_url)
            self.assertEqual(request.get_method(), method)
            self.assertEqual(parsed.path, DOWNSTREAM + path)
            self.assertEqual(parsed.netloc, "api.github.test")
            query = urllib.parse.parse_qs(parsed.query)
            self.assertEqual(query, {"event": ["repository_dispatch"], "branch": [branch], "per_page": ["100"]}
                             if path == "/actions/runs" else {})
            self.assertEqual(request.get_header("Authorization"), "Bearer test-token")
            self.assertFalse(follow_redirect)
            requests.append(request)
            return response
        client = GitHubClient(api_base="https://api.github.test", repository=inputs.repository,
                              token="test-token", user_agent="property-test", transport=transport)
        sleeps = []
        success = found and pending < completion and conclusion == "success"
        if success:
            self.assertEqual(dispatch_and_poll(client, inputs, new_correlation=lambda: correlation, sleep=sleeps.append),
                             DispatchResult(correlation, selected_id, url))
        else:
            with self.assertRaises(ActionError):
                dispatch_and_poll(client, inputs, new_correlation=lambda: correlation, sleep=sleeps.append)
        self.assertEqual(len(requests), len(script))
        self.assertEqual(sleeps, [interval] * (misses + (pending if found else 0)))
        self.assertEqual(json.loads(requests[0].data), {
            "event_type": inputs.event_type,
            "client_payload": {"commit_hash": sha, "pr_number": pr, "source_run_id": source_id, "source_run_attempt": source_attempt,
                               "correlation": correlation, "expected_run_name": name}})

    @given(run_id=POSITIVE_ID)
    def test_run_schema_rejection_categories(self, run_id):
        baseline = run(run_id)
        self.assertEqual(_validate_run(baseline, INPUTS, RUN_NAME), baseline)
        for field, bad_values in (
            ("id", (None, True, 0, -1, 2**53, "1", 1.0)),
            ("status", (None, True, [], {}, "unknown")),
            ("event", (None, "push")), ("head_branch", (None, "other")), ("display_title", (None, "other")),
            ("conclusion", (None, True, [], {})),
            ("html_url", (None, "", "http://host/run", "https://", "https:///run", "https://host:bad/run",
                          "https://host:65536/run", "https://[invalid]/run", "https://@host/run", "https://:@host/run",
                          "https://user:secret@host/run", "https://host/a b", "https://host/\x7f", " https://host/run"))):
            for bad in bad_values:
                with self.assertRaises(ActionError):
                    _validate_run({**baseline, field: bad}, INPUTS, RUN_NAME)
        for bad in (None, [], "text"):
            with self.assertRaises(ActionError):
                _validate_run(bad, INPUTS, RUN_NAME)
        for status in _PENDING:
            pending = {**baseline, "status": status, "conclusion": None}
            self.assertEqual(_validate_run(pending, INPUTS, RUN_NAME), pending)

    @given(correlation=st.uuids(version=4).map(str), budget=st.integers(1, 4))
    def test_correlation_generation_and_discovery_categories(self, correlation, budget):
        for bad in (None, [], "", correlation[:-1], "0" * 36,
                    correlation[:14] + "0" + correlation[15:], correlation[:19] + "7" + correlation[20:]):
            client, transport = downstream({})
            with self.assertRaises(ActionError):
                dispatch_and_poll(client, INPUTS, new_correlation=lambda: bad, sleep=lambda _: self.fail("unexpected sleep"))
            self.assertEqual(transport.requests, [])
        inputs = replace(INPUTS, discovery_attempts=budget)
        for body, sleeps_expected in (({"workflow_runs": []}, budget),
                                      ({"workflow_runs": [None, {"display_title": "unrelated"}]}, budget),
                                      ({"workflow_runs": [run(1), run(2)]}, 0),
                                      ({"workflow_runs": [run(0)]}, 0),
                                      ({"runs": []}, 0), ({"workflow_runs": {}}, 0), (None, 0)):
            client, transport = downstream({LIST: json_response(body), f"{DOWNSTREAM}/actions/runs/777": run()})
            sleeps = []
            with self.assertRaises(ActionError):
                dispatch_and_poll(client, inputs, new_correlation=lambda: CORRELATION, sleep=sleeps.append)
            self.assertEqual(sleeps, [0] * sleeps_expected)
            self.assertEqual(transport.urls(), [f"{DOWNSTREAM}/dispatches"] + [LIST] * (budget if sleeps_expected else 1))

    @given(run_id=st.integers(1, 2**53 - 2))
    def test_completion_revalidates_selected_run(self, run_id):
        for overrides in ({"id": run_id + 1}, {"event": "push"}, {"head_branch": "other"},
                          {"display_title": RUN_NAME + "-other"}, {"status": []}, {"conclusion": None},
                          {"html_url": "https://"}):
            client, transport = downstream({LIST: {"workflow_runs": [run(run_id, status="queued", conclusion=None)]},
                                           f"{DOWNSTREAM}/actions/runs/{run_id}": run(run_id, **overrides)})
            with self.assertRaises(ActionError):
                dispatch(client)
            self.assertEqual(transport.urls(), [f"{DOWNSTREAM}/dispatches", LIST, f"{DOWNSTREAM}/actions/runs/{run_id}"])

    @given(run_id=POSITIVE_ID)
    def test_each_non_success_conclusion_fails(self, run_id):
        for conclusion in _FAILURES:
            failed = run(run_id, conclusion=conclusion)
            client, transport = downstream({LIST: {"workflow_runs": [failed]}, f"{DOWNSTREAM}/actions/runs/{run_id}": failed})
            with self.assertRaises(ActionError):
                dispatch(client)
            self.assertEqual(len(transport.requests), 3)

    @given(run_id=POSITIVE_ID, correlation=st.uuids(version=4).map(str))
    def test_main_binds_downstream_client_and_serializes_outputs(self, run_id, correlation):
        result = DispatchResult(correlation, run_id, f"https://enterprise.test/runs/{run_id}")
        with mock.patch.dict("os.environ", {**ENV, "INPUT_TOKEN": "dispatch-token"}, clear=True), \
                mock.patch("ci_actions.indexer_dispatch.GitHubClient.from_env") as from_env, \
                mock.patch("ci_actions.indexer_dispatch.dispatch_and_poll", return_value=result) as polling, \
                mock.patch("ci_actions.indexer_dispatch.write_outputs") as outputs:
            main()
        from_env.assert_called_once_with(user_agent="aptos-indexer-processor-dispatch", repository=INPUTS.repository,
                                         token_env="INPUT_TOKEN")
        polling.assert_called_once_with(from_env.return_value, INPUTS)
        outputs.assert_called_once_with({"correlation": correlation, "downstream_run_id": str(run_id), "downstream_run_url": result.run_url})


if __name__ == "__main__":
    unittest.main()
