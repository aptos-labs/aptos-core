import io
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest import mock

from ci_actions import github
from ci_actions.github import ActionError, GitHubClient, HttpResponse, paginate
from tests.helpers import CI_DIR, REPO_PATH, json_response, json_route, local_server, route_client


class InputParsingTests(unittest.TestCase):
    def test_require_env_rejects_missing_empty_and_multiline_values(self):
        for value in (None, "", "a\nb", "a\rb"):
            env = {} if value is None else {"NAME": value}
            with self.subTest(value=value), mock.patch.dict(os.environ, env, clear=True):
                with self.assertRaisesRegex(ActionError, "NAME"):
                    github.require_env("NAME")
        with mock.patch.dict(os.environ, {"NAME": "value"}, clear=True):
            self.assertEqual(github.require_env("NAME"), "value")

    def test_parse_positive_int_accepts_only_safe_canonical_integers(self):
        for value in ("0", "1.5", "-1", "01", "+1", "42\n", " 42", "9007199254740992", "٤٢"):
            with self.subTest(value=value), self.assertRaises(ActionError):
                github.parse_positive_int(value, "pr_number")
        self.assertEqual(github.parse_positive_int("42", "pr_number"), 42)
        self.assertEqual(github.parse_positive_int("9007199254740991", "pr_number"), 2**53 - 1)

    def test_parse_repository(self):
        self.assertEqual(github.parse_repository("aptos-labs/aptos-core"), ("aptos-labs", "aptos-core"))
        for value in ("wrong", "a/b/c", "a b/c", "a/b\n", "", "../..", "./x", "x/.."):
            with self.subTest(value=value), self.assertRaises(ActionError):
                github.parse_repository(value)

    def test_parse_api_base_requires_https_without_credentials_query_or_fragment(self):
        self.assertEqual(github.parse_api_base("https://api.github.com/"), "https://api.github.com")
        self.assertEqual(github.parse_api_base("https://ghes.test/api/v3"), "https://ghes.test/api/v3")
        self.assertEqual(
            github.parse_api_base("http://127.0.0.1:8080", allow_loopback_http=True), "http://127.0.0.1:8080"
        )
        for value, loopback in [
            ("http://127.0.0.1:8080", False),
            ("http://example.test", True),
            ("https://user:pass@api.github.com", False),
            ("https://api.github.com/?x=1", False),
            ("https://api.github.com/?", False),
            ("https://api.github.com/#x", False),
            ("https://api.github.com:bad", False),
            ("https://api.github.com/\tx", False),
            ("not a url", False),
        ]:
            with self.subTest(value=value), self.assertRaises(ActionError):
                github.parse_api_base(value, allow_loopback_http=loopback)

    def test_parse_utc_timestamp_accepts_offsets_and_rejects_invalid_instants(self):
        self.assertEqual(
            github.parse_utc_timestamp("2020-01-20T09:42:40.000-08:00"),
            github.parse_utc_timestamp("2020-01-20T17:42:40Z"),
        )
        for value in (
            None, "not-a-timestamp", "2020-01-20T09:42:40.000+24:00", "2020-01-20T09:42:40.000-08:60",
            "2020-02-30T09:42:40.000-08:00", "2020-01-20T09:42:60Z", "2020-01-20 09:42:40Z", "2020-01-20T09:42:40",
        ):
            with self.subTest(value=value):
                self.assertIsNone(github.parse_utc_timestamp(value))


class ClientTests(unittest.TestCase):
    def test_builds_repository_urls_and_sends_api_headers(self):
        client, transport = route_client({f"{REPO_PATH}/pulls?state=all&head=o%3Aa%2Fb%2Bc": [1]})
        self.assertEqual(client.repository, "aptos-labs/aptos-core")
        self.assertEqual(client.get_json("/pulls", {"state": "all", "head": "o:a/b+c"}), [1])
        request = transport.requests[0]
        self.assertEqual(request["authorization"], "Bearer test-token")
        self.assertEqual(request["max_bytes"], github.MAX_JSON_BYTES)
        self.assertFalse(request["follow_redirect"])

    def test_keeps_an_api_base_path_prefix(self):
        client, transport = route_client({f"/api/v3{REPO_PATH}/pulls/1": {}}, api_base="https://ghes.test/api/v3/")
        client.get_json("/pulls/1")
        self.assertEqual(transport.urls(), [f"/api/v3{REPO_PATH}/pulls/1"])

    def test_rejects_paths_outside_the_repository(self):
        client, transport = route_client({})
        for path in (
            "pulls", "/pulls?x=1", "/pulls#x", "/../../orgs", "/a/./b", "/a\\b",
            "/%2e%2e/x", "/pulls/1%2F..%2F..", "/pulls/1\r\nX: y", "/pulls/ 1", "/a?b", "/a#b",
        ):
            with self.subTest(path=path), self.assertRaisesRegex(ActionError, "invalid repository API path"):
                client.get_json(path)
        self.assertEqual(transport.requests, [])

    def test_accepts_a_normal_nested_path(self):
        client, transport = route_client({f"{REPO_PATH}/actions/runs/1/jobs": {}})
        client.get_json("/actions/runs/1/jobs")
        self.assertEqual(transport.urls(), [f"{REPO_PATH}/actions/runs/1/jobs"])

    def test_non_2xx_raises_with_status_and_without_body_or_token(self):
        body = b"test-token and internal response details"
        client, _ = route_client({f"{REPO_PATH}/x": HttpResponse(500, {}, body)})
        with self.assertRaises(ActionError) as caught:
            client.get_json("/x")
        self.assertEqual(caught.exception.status, 500)
        self.assertIn("HTTP 500", str(caught.exception))
        self.assertNotIn("test-token", str(caught.exception))
        self.assertNotIn("internal", str(caught.exception))

    def test_rejects_malformed_and_non_finite_json(self):
        for body in (b"{", b"\xff", b'{"a": NaN}', b"[Infinity]"):
            client, _ = route_client({f"{REPO_PATH}/x": HttpResponse(200, {}, body)})
            with self.subTest(body=body), self.assertRaisesRegex(ActionError, "malformed JSON"):
                client.get_json("/x")

    def test_enforces_declared_and_actual_size_limits(self):
        client, _ = route_client({
            f"{REPO_PATH}/declared": HttpResponse(200, {"content-length": "101"}, b"x"),
            f"{REPO_PATH}/bad-declared": HttpResponse(200, {"content-length": "1e3"}, b"x"),
            f"{REPO_PATH}/actual": HttpResponse(200, {}, b"x" * 101),
            f"{REPO_PATH}/ok": HttpResponse(200, {"content-length": "2"}, b"ok"),
        })
        with self.assertRaisesRegex(ActionError, "declared size"):
            client.get_bytes("/declared", max_bytes=100)
        with self.assertRaisesRegex(ActionError, "declared size"):
            client.get_bytes("/bad-declared", max_bytes=100)
        with self.assertRaisesRegex(ActionError, "exceeded"):
            client.get_bytes("/actual", max_bytes=100)
        self.assertEqual(client.get_bytes("/ok", max_bytes=100), b"ok")

    def test_post_json_sends_a_json_body_and_accepts_204(self):
        client, transport = route_client({f"{REPO_PATH}/dispatches": HttpResponse(204, {}, b"")})
        client.post_json("/dispatches", {"event_type": "e"})
        self.assertEqual(transport.requests[0]["method"], "POST")
        self.assertEqual(transport.requests[0]["body"], b'{"event_type":"e"}')

    def test_from_env_validates_every_input(self):
        env = {"GITHUB_API_URL": "https://api.github.test", "GH_TOKEN": "t", "GITHUB_REPOSITORY": "a/b"}
        with mock.patch.dict(os.environ, env, clear=True):
            self.assertEqual(GitHubClient.from_env(user_agent="x").repository, "a/b")
            self.assertEqual(GitHubClient.from_env(user_agent="x", repository="c/d").repository, "c/d")
        for key, value in [("GITHUB_API_URL", "http://api.github.test"), ("GH_TOKEN", ""), ("GITHUB_REPOSITORY", "ab")]:
            with self.subTest(key=key), mock.patch.dict(os.environ, {**env, key: value}, clear=True):
                with self.assertRaises(ActionError):
                    GitHubClient.from_env(user_agent="x")

    def test_loopback_http_is_rejected_unless_the_caller_opts_in(self):
        env = {"GITHUB_API_URL": "http://127.0.0.1:8080", "GH_TOKEN": "t", "GITHUB_REPOSITORY": "a/b"}
        with mock.patch.dict(os.environ, env, clear=True):
            with self.assertRaisesRegex(ActionError, "HTTPS"):
                GitHubClient.from_env(user_agent="x")
            self.assertEqual(GitHubClient.from_env(user_agent="x", allow_loopback_http=True).repository, "a/b")
        with self.assertRaisesRegex(ActionError, "HTTPS"):
            GitHubClient(api_base="http://localhost", token="t", repository="a/b", user_agent="x")


class DefaultTransportTests(unittest.TestCase):
    def client(self, base):
        return GitHubClient(api_base=base, token="secret-token", repository="a/b", user_agent="test",
                            allow_loopback_http=True)

    def test_sends_authorization_to_the_api(self):
        with local_server({"/repos/a/b/x": json_route({"ok": True})}) as (base, requests):
            self.assertEqual(self.client(base).get_json("/x"), {"ok": True})
        self.assertEqual(requests[0]["authorization"], "Bearer secret-token")

    def test_refuses_redirects_by_default(self):
        with local_server({"/target": json_route({})}) as (target, target_requests):
            def redirect(handler):
                handler.send_response(302)
                handler.send_header("location", f"{target}/target")
                handler.end_headers()

            with local_server({"/repos/a/b/x": redirect}) as (base, _):
                with self.assertRaisesRegex(ActionError, "HTTP 302"):
                    self.client(base).get_json("/x")
        self.assertEqual(target_requests, [])

    def test_followed_redirect_does_not_forward_authorization(self):
        def blob(handler):
            handler.send_response(200)
            handler.send_header("content-length", "5")
            handler.end_headers()
            handler.wfile.write(b"bytes")

        with local_server({"/blob": blob}) as (target, target_requests):
            def redirect(handler):
                handler.send_response(302)
                handler.send_header("location", f"{target}/blob")
                handler.end_headers()

            with local_server({"/repos/a/b/zip": redirect}) as (base, api_requests):
                data = self.client(base).get_bytes("/zip", max_bytes=100, follow_redirect=True)
        self.assertEqual(data, b"bytes")
        self.assertEqual(api_requests[0]["authorization"], "Bearer secret-token")
        self.assertIsNone(target_requests[0]["authorization"])

    def test_followed_redirect_must_stay_on_https(self):
        def redirect(handler):
            handler.send_response(302)
            handler.send_header("location", "http://blob.example.invalid/zip")
            handler.end_headers()

        with local_server({"/repos/a/b/zip": redirect}) as (base, _):
            with self.assertRaisesRegex(ActionError, "HTTP 302"):
                self.client(base).get_bytes("/zip", max_bytes=100, follow_redirect=True)

    def test_reads_at_most_max_bytes_plus_one(self):
        def unbounded(handler):
            handler.send_response(200)
            handler.end_headers()
            handler.wfile.write(b"x" * 1000)

        with local_server({"/repos/a/b/big": unbounded}) as (base, _):
            with self.assertRaisesRegex(ActionError, "exceeded"):
                self.client(base).get_bytes("/big", max_bytes=100)

    def test_connection_failure_fails_closed(self):
        with local_server({}) as (base, _):
            pass
        with self.assertRaisesRegex(ActionError, "request failed"):
            self.client(base).get_json("/x")


class PaginateTests(unittest.TestCase):
    PATH = f"{REPO_PATH}/issues/42/timeline"

    def test_plain_list_stops_on_a_short_page(self):
        client, transport = route_client({
            f"{self.PATH}?per_page=2&page=1": [1, 2],
            f"{self.PATH}?per_page=2&page=2": [3],
        })
        self.assertEqual(paginate(client, "/issues/42/timeline", items_key=None, max_pages=5, per_page=2), [1, 2, 3])
        self.assertEqual(len(transport.requests), 2)

    def test_short_page_with_a_next_link_continues(self):
        next_link = {"link": '<https://attacker.example/timeline?page=2>; rel="next"'}
        client, transport = route_client({
            f"{self.PATH}?per_page=100&page=1": json_response([1], headers=next_link),
            f"{self.PATH}?per_page=100&page=2": [2],
        })
        self.assertEqual(paginate(client, "/issues/42/timeline", items_key=None, max_pages=5), [1, 2])
        self.assertEqual(transport.urls(), [f"{self.PATH}?per_page=100&page=1", f"{self.PATH}?per_page=100&page=2"])

    def test_fails_closed_at_the_page_limit(self):
        client, transport = route_client({
            f"{self.PATH}?per_page=1&page=1": [1],
            f"{self.PATH}?per_page=1&page=2": [2],
        })
        with self.assertRaisesRegex(ActionError, "pagination limit"):
            paginate(client, "/issues/42/timeline", items_key=None, max_pages=2, per_page=1)
        self.assertEqual(len(transport.requests), 2)

    def test_rejects_malformed_and_oversized_pages(self):
        for body in ({"events": []}, [1, 2, 3], "text"):
            client, _ = route_client({f"{self.PATH}?per_page=2&page=1": body})
            with self.subTest(body=body), self.assertRaisesRegex(ActionError, "malformed list page"):
                paginate(client, "/issues/42/timeline", items_key=None, max_pages=5, per_page=2)

    def test_total_count_must_be_stable_and_match(self):
        path = f"{REPO_PATH}/actions/runs/1/artifacts"
        cases = [
            ([{"total_count": 3, "artifacts": [1, 2]}, {"total_count": 4, "artifacts": [3]}], "changed during"),
            ([{"total_count": 3, "artifacts": [1]}], "does not match"),
            ([{"total_count": -1, "artifacts": []}], "total count"),
            ([{"total_count": True, "artifacts": []}], "total count"),
        ]
        for pages, error in cases:
            routes = {f"{path}?per_page=2&page={index}": page for index, page in enumerate(pages, 1)}
            client, _ = route_client(routes)
            with self.subTest(pages=pages), self.assertRaisesRegex(ActionError, error):
                paginate(client, "/actions/runs/1/artifacts", items_key="artifacts",
                         total_count_key="total_count", max_pages=5, per_page=2)
        client, _ = route_client({
            f"{path}?per_page=2&page=1": {"total_count": 2, "artifacts": [1, 2]},
            f"{path}?per_page=2&page=2": {"total_count": 2, "artifacts": []},
        })
        self.assertEqual(
            paginate(client, "/actions/runs/1/artifacts", items_key="artifacts",
                     total_count_key="total_count", max_pages=5, per_page=2),
            [1, 2],
        )

    def test_passes_caller_params_before_page_params(self):
        client, transport = route_client({f"{REPO_PATH}/actions/runs/1/jobs?filter=latest&per_page=100&page=1": {
            "total_count": 0, "jobs": []}})
        paginate(client, "/actions/runs/1/jobs", items_key="jobs", total_count_key="total_count",
                 max_pages=10, params={"filter": "latest"})
        self.assertEqual(len(transport.requests), 1)


class OutputAndRunnerTests(unittest.TestCase):
    def test_write_outputs_appends_all_values_or_none(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "out"
            path.write_text("existing=1\n")
            with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(path)}):
                github.write_outputs({"a": "1", "b": ""})
                for values in ({"ok": "1", "bad": "x\ny"}, {"ok": "1", "bad": "x\r"}, {"bad name": "1"}, {"n": 1}):
                    with self.subTest(values=values), self.assertRaises(ActionError):
                        github.write_outputs(values)
            self.assertEqual(path.read_text(), "existing=1\na=1\nb=\n")

    def test_run_action_reports_failures_and_returns_exit_codes(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            self.assertEqual(github.run_action("demo", lambda: None), 0)
            self.assertEqual(github.run_action("demo", lambda: (_ for _ in ()).throw(ValueError("boom"))), 1)
        self.assertEqual(stderr.getvalue(), "demo failed closed: boom\n")

    def test_launcher_rejects_unknown_commands(self):
        result = subprocess.run(
            [sys.executable, "-I", str(CI_DIR / "run_action.py"), "no-such-command"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("usage", result.stderr)

    def test_every_launcher_command_resolves_to_a_function_and_an_action(self):
        sys.path.insert(0, str(CI_DIR))
        import importlib
        import run_action

        for command, target in run_action.COMMANDS.items():
            module_name, function_name = target.split(":")
            with self.subTest(command=command):
                self.assertTrue(callable(getattr(importlib.import_module(f"ci_actions.{module_name}"), function_name)))
                action = (CI_DIR.parent / "actions" / command / "action.yml").read_text()
                self.assertIn(f'python3 -I "$GITHUB_ACTION_PATH/../../ci/run_action.py" {command}', action)


if __name__ == "__main__":
    unittest.main()
