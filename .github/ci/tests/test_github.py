import importlib.util
import json
import string
import urllib.parse
import urllib.request
from datetime import datetime, timedelta
import os
import subprocess
import sys
import unittest
from unittest import mock

from hypothesis import example, given, strategies as st

from tests.property_support import POSITIVE_ID, REPOSITORY, SAFE_COMPONENT, configure_profiles

configure_profiles()

from ci_actions import github
from ci_actions.github import ActionError, GitHubClient, HttpResponse, paginate
from tests.helpers import CI_DIR, REPO_PATH, json_response, json_route, local_server, route_client


class ClientTests(unittest.TestCase):
    def test_from_env_binds_validated_inputs_and_rejects_loopback_by_default(self):
        env = {"GITHUB_API_URL": "https://api.github.test", "GH_TOKEN": "t", "GITHUB_REPOSITORY": "a/b"}
        loopback = {**env, "GITHUB_API_URL": "http://127.0.0.1:8080"}
        for values, options, repository in ((env, {}, "a/b"), (env, {"repository": "c/d"}, "c/d"),
                                            (loopback, {"allow_loopback_http": True}, "a/b")):
            with self.subTest(options=options), mock.patch.dict(os.environ, values, clear=True):
                self.assertEqual(GitHubClient.from_env(user_agent="x", **options).repository, repository)
        for key, value in [("GITHUB_API_URL", loopback["GITHUB_API_URL"]), ("GITHUB_API_URL", "http://api.github.test"),
                           ("GH_TOKEN", ""), ("GITHUB_REPOSITORY", "ab")]:
            with self.subTest(key=key, value=value), mock.patch.dict(os.environ, {**env, key: value}, clear=True):
                with self.assertRaises(ActionError):
                    GitHubClient.from_env(user_agent="x")


class DefaultTransportTests(unittest.TestCase):
    def client(self, base):
        return GitHubClient(api_base=base, token="secret-token", repository="a/b", user_agent="test",
                            allow_loopback_http=True)

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

        with local_server({"/big": unbounded}) as (base, _):
            response = github.urllib_transport(urllib.request.Request(f"{base}/big"), 100, False)
        self.assertEqual(len(response.body), 101)

    def test_connection_failure_fails_closed(self):
        with local_server({}) as (base, _):
            pass
        with self.assertRaisesRegex(ActionError, "request failed"):
            self.client(base).get_json("/x")


class OutputAndRunnerTests(unittest.TestCase):
    def test_launcher_rejects_unknown_commands(self):
        result = subprocess.run(
            [sys.executable, "-I", str(CI_DIR / "run_action.py"), "no-such-command"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("usage", result.stderr)

    def test_every_launcher_command_resolves_to_a_function_and_an_action(self):
        spec = importlib.util.spec_from_file_location("run_action", CI_DIR / "run_action.py")
        run_action = importlib.util.module_from_spec(spec)
        # The launcher adds its own directory to sys.path; keep that out of the test process.
        with mock.patch.object(sys, "path", list(sys.path)):
            spec.loader.exec_module(run_action)
        for command, target in run_action.COMMANDS.items():
            module_name, function_name = target.split(":")
            with self.subTest(command=command):
                self.assertTrue(callable(getattr(importlib.import_module(f"ci_actions.{module_name}"), function_name)))
                action = (CI_DIR.parent / "actions" / command / "action.yml").read_text()
                self.assertIn(f'/ci/run_action.py" {command}\n', action)


_JSON_VALUES = st.recursive(
    st.none() | st.booleans() | st.integers(-1000, 1000) | st.text(alphabet=string.ascii_letters, max_size=32),
    lambda children: st.lists(children, max_size=4) | st.dictionaries(SAFE_COMPONENT, children, max_size=4),
    max_leaves=20,
)


class GitHubProperties(unittest.TestCase):
    @example(number=42)
    @example(number=2**53 - 1)
    @given(number=POSITIVE_ID)
    def test_canonical_integer_and_rejection_categories(self, number):
        text = str(number)
        self.assertEqual(github.parse_positive_int(text, "number"), number)
        for bad in (None, True, number, "", "0", "0" + text, "+" + text, "-" + text,
                    text + ".0", text + "\n", " " + text, "٤٢", str(2**53)):
            with self.assertRaises(ActionError):
                github.parse_positive_int(bad, "number")

    @given(value=st.text(alphabet=string.ascii_letters + " =_-", min_size=1, max_size=64))
    def test_environment_categories(self, value):
        with mock.patch.dict(os.environ, {"NAME": value}, clear=True):
            self.assertEqual(github.require_env("NAME"), value)
        for bad in (None, "", value + "\r", value + "\n"):
            env = {} if bad is None else {"NAME": bad}
            with mock.patch.dict(os.environ, env, clear=True), self.assertRaises(ActionError):
                github.require_env("NAME")

    @example(owner="aptos-labs", name="aptos-core")
    @given(owner=SAFE_COMPONENT, name=SAFE_COMPONENT)
    def test_repository_categories(self, owner, name):
        self.assertEqual(github.parse_repository(f"{owner}/{name}"), (owner, name))
        for bad in (None, "", owner, f"{owner}/{name}/extra", f" {owner}/{name}", f"{owner} {name}/{name}",
                    f"{owner}/{name}\n", f"./{name}", f"../{name}", f"{owner}/..", f"{owner}/."):
            with self.assertRaises(ActionError):
                github.parse_repository(bad)
        self.assertEqual(github.parse_repository(f".{owner}/{name}."), (f".{owner}", f"{name}."))

    @given(component=SAFE_COMPONENT, port=st.integers(1, 65535))
    def test_api_base_categories(self, component, port):
        host = "enterprise.test"
        base = f"https://{host}:{port}/api/{component}"
        self.assertEqual(github.parse_api_base(base + "///"), base)
        for loopback in ("127.0.0.1", "localhost", "[::1]"):
            url = f"http://{loopback}:{port}"
            self.assertEqual(github.parse_api_base(url, allow_loopback_http=True), url)
            with self.assertRaises(ActionError):
                github.parse_api_base(url)
        for bad in (f"http://{host}", f"https://@{host}", f"https://:@{host}",
                    f"https://user:secret@{host}", base + "?", base + "?x=1", base + "#",
                    f"https://{host}:bad", f"https://{host}:65536", "https:///path", "https://[bad]",
                    base + "\t", base + "\x7f", base + " ", "not-a-url"):
            with self.assertRaises(ActionError):
                github.parse_api_base(bad, allow_loopback_http=True)

    @given(instant=st.datetimes(min_value=datetime(2000, 1, 2), max_value=datetime(2099, 12, 30)),
           offset=st.integers(-1439, 1439), digits=st.integers(0, 3), fraction=st.integers(0, 999))
    def test_timestamp_integer_epoch_model(self, instant, offset, digits, fraction):
        # Build the expected epoch from integer timedelta components, not the parser.
        milliseconds = fraction // (10 ** (3 - digits)) * (10 ** (3 - digits)) if digits else 0
        instant = instant.replace(microsecond=milliseconds * 1000)
        local = instant + timedelta(minutes=offset)
        suffix = "Z" if offset == 0 else f"{'+' if offset >= 0 else '-'}{abs(offset)//60:02}:{abs(offset)%60:02}"
        fractional = "" if not digits else "." + f"{milliseconds:03}"[:digits]
        value = local.strftime("%Y-%m-%dT%H:%M:%S") + fractional + suffix
        delta = instant - datetime(1970, 1, 1)
        expected = (delta.days * 86400 + delta.seconds) * 1000 + milliseconds
        self.assertEqual(github.parse_utc_timestamp(value), expected)
        for bad in (None, 1, "2000-02-30T00:00:00Z", "2000-01-01T24:00:00Z", "2000-01-01T00:00:60Z",
                    "2000-01-01T00:00:00+24:00", "2000-01-01T00:00:00-08:60", value + "\n",
                    "2000-01-01 00:00:00Z", "2000-01-01T00:00:00", "2000-01-01T00:00:00.1234Z", "2000-01-01T00:00:00.0000Z"):
            self.assertIsNone(github.parse_utc_timestamp(bad))

    @given(repository=REPOSITORY, segments=st.lists(SAFE_COMPONENT, min_size=1, max_size=4),
           query=st.text(alphabet="ab /+:é&=#?%", max_size=32), value=_JSON_VALUES)
    def test_request_scope_headers_query_and_json(self, repository, segments, query, value):
        requests = []

        def transport(request, max_bytes, follow_redirect):
            requests.append((request, max_bytes, follow_redirect))
            return json_response(value)

        client = GitHubClient(api_base="https://enterprise.test/api/v3/", token="secret",
                              repository=repository, user_agent="property-test", transport=transport)
        path = "/" + "/".join(segments)
        self.assertEqual(client.get_json(path, {"filter": query}), value)
        request, limit, follow = requests[0]
        parsed = urllib.parse.urlsplit(request.full_url)
        self.assertEqual(parsed.scheme, "https")
        self.assertEqual(parsed.netloc, "enterprise.test")
        self.assertEqual(parsed.path, f"/api/v3/repos/{repository}{path}")
        self.assertEqual(urllib.parse.parse_qs(parsed.query, keep_blank_values=True), {"filter": [query]})
        self.assertEqual(request.get_method(), "GET")
        self.assertEqual(request.get_header("Authorization"), "Bearer secret")
        self.assertEqual(request.get_header("Accept"), "application/vnd.github+json")
        self.assertEqual(request.get_header("X-github-api-version"), github.API_VERSION)
        self.assertEqual(request.get_header("User-agent"), "property-test")
        self.assertEqual(limit, github.MAX_JSON_BYTES)
        self.assertFalse(follow)
        client.post_json(path, {"value": value})
        posted, _, posted_follow = requests[1]
        self.assertEqual(posted.get_method(), "POST")
        self.assertEqual(posted.get_header("Content-type"), "application/json")
        self.assertEqual(json.loads(posted.data), {"value": value})
        self.assertFalse(posted_follow)

    @given(segment=SAFE_COMPONENT)
    def test_invalid_path_categories_make_no_request(self, segment):
        client, transport = route_client({})
        for bad in (segment, "/", f"/{segment}//x", f"/{segment}?x", f"/{segment}#x", "/../x", "/./x",
                    "/a/../b", f"/{segment}\\x", "/%2e%2e/x", "/a%2Fb", f"/{segment} x", f"/{segment}\r\nX: y"):
            with self.assertRaises(ActionError):
                client.get_json(bad)
        self.assertEqual(transport.requests, [])

    @given(limit=st.integers(0, 128), size=st.integers(0, 160))
    def test_status_and_size_partition(self, limit, size):
        body = b"x" * size
        for status in (199, 200, 204, 299, 300, 500):
            for declared, declared_ok in ((None, True), (str(limit), True), ("00", True),
                                          (str(limit + 1), False), ("1e3", False), ("+1", False), ("١", False)):
                headers = {} if declared is None else {"content-length": declared}
                client, transport = route_client({f"{REPO_PATH}/x": HttpResponse(status, headers, body)})
                if 200 <= status <= 299 and declared_ok and size <= limit:
                    self.assertEqual(client.get_bytes("/x", max_bytes=limit), body)
                else:
                    with self.assertRaises(ActionError) as caught:
                        client.get_bytes("/x", max_bytes=limit)
                    if status < 200 or status > 299:
                        self.assertEqual(caught.exception.status, status)
                        self.assertNotIn("test-token", str(caught.exception))
                        if size > 20:
                            self.assertNotIn(body.decode(), str(caught.exception))
                self.assertEqual(len(transport.requests), 1)

    @given(value=_JSON_VALUES)
    def test_json_roundtrip_and_malformed_categories(self, value):
        # Supply an explicit response because None is also the missing-route sentinel.
        client, _ = route_client({f"{REPO_PATH}/x": json_response(value)})
        self.assertEqual(client.get_json("/x"), value)
        for body in (b"{", b"\xff", b'"\xff"', b'{"a":NaN}', b"[Infinity]", b"[-Infinity]",
                     b'{"a": 1, "a": 2}', b'[{"a": 1, "a": 2}]', b"[1e309]"):
            client, _ = route_client({f"{REPO_PATH}/x": HttpResponse(200, {}, body)})
            with self.assertRaisesRegex(ActionError, "malformed JSON"):
                client.get_json("/x")

    @given(data=st.data(), per_page=st.integers(1, 4), max_pages=st.integers(1, 5), totals=st.booleans())
    def test_pagination_sequence_model(self, data, per_page, max_pages, totals):
        sizes = data.draw(st.lists(st.integers(0, per_page), min_size=max_pages, max_size=max_pages), label="page sizes")
        next_flags = data.draw(st.lists(st.booleans(), min_size=max_pages, max_size=max_pages), label="next flags")
        stop = next((i + 1 for i, (size, has_next) in enumerate(zip(sizes, next_flags))
                     if size < per_page and not has_next), None)
        read_count = stop if stop is not None else max_pages
        expected = [(page, item) for page in range(read_count) for item in range(sizes[page])]
        routes = {}
        for page, (size, has_next) in enumerate(zip(sizes, next_flags), 1):
            items = [[page - 1, item] for item in range(size)]
            body = {"items": items, "total": len(expected)} if totals else items
            headers = {"link": '<https://attacker.test/other?after=opaque>; rel="prev next", <https://other.test/>; rel="next"'} if has_next else {"link": '<https://attacker.test/>; rel="last"'}
            routes[f"{REPO_PATH}/x?filter=latest&per_page={per_page}&page={page}"] = json_response(body, headers=headers)
        client, transport = route_client(routes)
        kwargs = dict(items_key="items" if totals else None, total_count_key="total" if totals else None,
                      per_page=per_page, max_pages=max_pages, params={"filter": "latest", "per_page": 999, "page": 99})
        if stop is None:
            with self.assertRaises(ActionError):
                paginate(client, "/x", **kwargs)
        else:
            try:
                actual = paginate(client, "/x", **kwargs)
            except ActionError as error:
                self.fail(f"Valid pagination sequence rejected after requests {transport.urls()}: {error}")
            self.assertEqual(actual, [list(item) for item in expected])
        self.assertEqual(transport.urls(), [f"{REPO_PATH}/x?filter=latest&per_page={per_page}&page={page}" for page in range(1, read_count + 1)])

    @given(item=st.integers(-100, 100))
    def test_pagination_malformed_and_count_categories(self, item):
        key = f"{REPO_PATH}/x?per_page=2&page=1"
        for body in ({"events": []}, "text", None, [item] * 3):
            client, transport = route_client({key: json_response(body)})
            with self.assertRaises(ActionError):
                paginate(client, "/x", items_key=None, max_pages=2, per_page=2)
            self.assertEqual(len(transport.requests), 1)
        for total in (None, True, -1, 2**53, "1", 1.0):
            client, transport = route_client({key: {"items": [item], "total": total}})
            with self.assertRaisesRegex(ActionError, "malformed total count"):
                paginate(client, "/x", items_key="items", total_count_key="total", max_pages=2, per_page=2)
            self.assertEqual(len(transport.requests), 1)
        for totals in ((3, 4), (4, 4)):
            client, _ = route_client({key: {"items": [item, item], "total": totals[0]},
                                     f"{REPO_PATH}/x?per_page=2&page=2": {"items": [item], "total": totals[1]}})
            with self.assertRaises(ActionError):
                paginate(client, "/x", items_key="items", total_count_key="total", max_pages=2, per_page=2)
        client, transport = route_client({})
        for maximum, page_size in ((0, 2), (-1, 2), (2, 0), (2, 101)):
            with self.assertRaises(ActionError):
                paginate(client, "/x", items_key=None, max_pages=maximum, per_page=page_size)
        self.assertEqual(transport.requests, [])

    @given(values=st.dictionaries(SAFE_COMPONENT, st.text(alphabet="ab =é", max_size=64), max_size=4))
    def test_output_validation_before_open(self, values):
        values = {"out_" + name: value for name, value in values.items()}
        with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": "unused"}, clear=True), mock.patch("builtins.open", mock.mock_open()) as opened:
            github.write_outputs(values)
            opened.assert_called_once_with("unused", "a", encoding="utf-8")
            opened().write.assert_called_once_with("".join(f"{name}={value}\n" for name, value in values.items()))
        for bad in ({"ok": "1", "bad": "x\ny"}, {"ok": "1", "bad": "x\r"}, {"bad name": "1"}, {"n": 1}, {"1bad": "x"}):
            with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": "unused"}, clear=True), \
                    mock.patch("builtins.open") as opened:
                try:
                    github.write_outputs(bad)
                except Exception as error:
                    self.assertIsInstance(error, ActionError, "Unsafe output must fail with ActionError before opening a file")
                else:
                    self.fail("Unsafe output was accepted")
            opened.assert_not_called()


if __name__ == "__main__":
    unittest.main()
