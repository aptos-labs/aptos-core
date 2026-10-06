"""Shared test doubles: an in-process route transport and a loopback HTTP server."""

import io
import json
import os
import subprocess
import sys
import tempfile
import threading
import urllib.parse
from contextlib import contextmanager, redirect_stderr
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from unittest import mock

from ci_actions.github import GitHubClient, HttpResponse, run_action

CI_DIR = Path(__file__).resolve().parents[1]
REPOSITORY = "aptos-labs/aptos-core"
REPO_PATH = "/repos/aptos-labs/aptos-core"
SHA = "0123456789abcdef0123456789abcdef01234567"


def json_response(value, status=200, headers=None):
    return HttpResponse(status, {"content-type": "application/json", **(headers or {})}, json.dumps(value).encode())


class RouteTransport:
    """Serves `routes[path_and_query]` and records every request.

    A route value is an HttpResponse, a callable(request) -> HttpResponse, or a
    JSON value returned with HTTP 200. Unknown routes return HTTP 404."""

    def __init__(self, routes):
        self.routes = routes
        self.requests = []

    def __call__(self, request, max_bytes, follow_redirect):
        url = urllib.parse.urlsplit(request.full_url)
        key = url.path + (f"?{url.query}" if url.query else "")
        self.requests.append({
            "method": request.get_method(),
            "url": key,
            "authorization": request.get_header("Authorization"),
            "body": request.data,
            "max_bytes": max_bytes,
            "follow_redirect": follow_redirect,
        })
        route = self.routes.get(key)
        if route is None:
            return HttpResponse(404, {}, b"")
        if callable(route):
            return route(request)
        if isinstance(route, HttpResponse):
            return route
        return json_response(route)

    def urls(self):
        return [request["url"] for request in self.requests]


def route_client(routes, repository=REPOSITORY, api_base="https://api.github.test"):
    transport = RouteTransport(routes)
    client = GitHubClient(
        api_base=api_base, token="test-token", repository=repository, user_agent="ci-actions-test", transport=transport
    )
    return client, transport


@contextmanager
def local_server(routes):
    """Loopback HTTP server. `routes[path_and_query](handler)` writes the response."""
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def _serve(self):
            length = int(self.headers.get("content-length") or 0)
            requests.append({
                "method": self.command,
                "url": self.path,
                "authorization": self.headers.get("authorization"),
                "body": self.rfile.read(length) if length else b"",
            })
            route = routes.get(self.path)
            if route is None:
                self.send_json(404, {"message": f"unexpected request {self.path}"})
            else:
                route(self)

        do_GET = _serve
        do_POST = _serve

        def send_json(self, status, value, headers=None):
            body = json.dumps(value).encode()
            self.send_response(status)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            for key, header in (headers or {}).items():
                self.send_header(key, header)
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    # A short poll interval keeps shutdown() from blocking for the default 0.5 s.
    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.02}, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}", requests
    finally:
        server.shutdown()
        server.server_close()


def json_route(value, status=200, headers=None):
    return lambda handler: handler.send_json(status, value, headers)


def _read_outputs(output):
    outputs = {}
    for line in output.read_text().splitlines():
        name, _, value = line.partition("=")
        outputs[name] = value
    files = {name: Path(value).read_text() for name, value in outputs.items()
             if name.endswith("_path") and value and Path(value).is_file()}
    return outputs, files


def run_main(command, main, env):
    """Run an action `main` in-process against a loopback server.

    Production `main` functions never allow loopback http, so this is the one
    place that turns it on: it wraps GitHubClient.from_env for the call."""
    from_env = GitHubClient.from_env.__func__

    def loopback_from_env(cls, **kwargs):
        return from_env(cls, allow_loopback_http=True, **kwargs)

    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "github-output"
        output.write_text("")
        stderr = io.StringIO()
        with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(output), "RUNNER_TEMP": directory, **env}, clear=True), \
                mock.patch.object(GitHubClient, "from_env", classmethod(loopback_from_env)), redirect_stderr(stderr):
            code = run_action(command, main)
        outputs, files = _read_outputs(output)
        return code, stderr.getvalue(), outputs, files


@contextmanager
def action_env(**env):
    """Run an action's main() in-process: set the Actions environment and yield
    the GITHUB_OUTPUT path. Pair it with `main(transport=RouteTransport(...))`."""
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "github-output"
        output.write_text("")
        values = {
            "GITHUB_API_URL": "https://api.github.test",
            "GITHUB_REPOSITORY": REPOSITORY,
            "GITHUB_SERVER_URL": "https://github.com",
            "GH_TOKEN": "test-token",
            "GITHUB_OUTPUT": str(output),
            **env,
        }
        with mock.patch.dict(os.environ, values):
            yield output


def read_outputs(path):
    return _read_outputs(Path(path))[0]


def run_command(command, env):
    """Run `python3 -I run_action.py <command>` as a subprocess, exactly as the
    action does. Production code rejects loopback http, so use this only for
    cases that fail before any request. Returns (code, stderr, outputs, files)."""
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "github-output"
        output.write_text("")
        full_env = {"PATH": os.environ.get("PATH", ""), "GITHUB_OUTPUT": str(output), "RUNNER_TEMP": directory, **env}
        result = subprocess.run(
            [sys.executable, "-I", str(CI_DIR / "run_action.py"), command],
            env=full_env, capture_output=True, text=True, check=False, timeout=60,
        )
        outputs, files = _read_outputs(output)
        return result.returncode, result.stderr, outputs, files
