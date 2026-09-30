"""GitHub REST client, input parsing and action output helpers for trusted actions."""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any, Callable, Mapping, Optional

API_VERSION = "2022-11-28"
MAX_JSON_BYTES = 16 * 1024 * 1024
MAX_SAFE_INTEGER = 2**53 - 1
REQUEST_TIMEOUT_SECONDS = 60
LOOPBACK_HOSTS = frozenset({"127.0.0.1", "::1", "localhost"})
_POSITIVE_INT = re.compile(r"[1-9][0-9]*")
_REPOSITORY = re.compile(r"([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)")
_OUTPUT_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_-]*")
# RFC 3986 unreserved characters only: no "%", so a percent-encoded dot segment
# (for example "%2e%2e") cannot pass as an ordinary segment.
_API_PATH = re.compile(r"(?:/[A-Za-z0-9_.~-]+)+")
_UTC_TIMESTAMP = re.compile(
    r"([0-9]{4})-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})"
    r"(?:\.([0-9]{1,3}))?(Z|([+-])([0-9]{2}):([0-9]{2}))"
)
_LINK_RELATION = re.compile(r'rel\s*=\s*"([^"]*)"')


class ActionError(Exception):
    """A fail-closed condition. `status` is the HTTP status when one caused it."""

    def __init__(self, message: str, *, status: Optional[int] = None) -> None:
        super().__init__(message)
        self.status = status


@dataclass(frozen=True)
class HttpResponse:
    status: int
    headers: Mapping[str, str]  # lower-cased keys
    body: bytes


Transport = Callable[[urllib.request.Request, int, bool], HttpResponse]


def require_env(name: str) -> str:
    value = os.environ.get(name, "")
    if not value or "\r" in value or "\n" in value:
        raise ActionError(f"missing or invalid required environment variable: {name}")
    return value


def parse_positive_int(value: str, name: str) -> int:
    if not isinstance(value, str) or _POSITIVE_INT.fullmatch(value) is None:
        raise ActionError(f"{name} must be a positive integer")
    parsed = int(value)
    if parsed > MAX_SAFE_INTEGER:
        raise ActionError(f"{name} must be a safe positive integer")
    return parsed


def parse_repository(value: str, name: str = "GITHUB_REPOSITORY") -> tuple[str, str]:
    match = _REPOSITORY.fullmatch(value) if isinstance(value, str) else None
    if match is None or match.group(1) in (".", "..") or match.group(2) in (".", ".."):
        raise ActionError(f"{name} must have the form owner/repository")
    return match.group(1), match.group(2)


def parse_api_base(value: str, *, allow_loopback_http: bool = False) -> str:
    """Return the API base URL without a trailing slash. `allow_loopback_http`
    lets local-server tests use http://127.0.0.1."""
    if any(ord(character) <= 0x20 or ord(character) == 0x7F for character in value):
        raise ActionError("GITHUB_API_URL must not contain whitespace or control characters")
    try:
        parts = urllib.parse.urlsplit(value)
        parts.port  # noqa: B018 - raises ValueError for an invalid port
    except ValueError as error:
        raise ActionError("GITHUB_API_URL must be a valid URL") from error
    if not parts.hostname:
        raise ActionError("GITHUB_API_URL must be a valid URL")
    loopback_http = (
        allow_loopback_http and parts.scheme == "http" and parts.hostname in LOOPBACK_HOSTS
    )
    if parts.scheme != "https" and not loopback_http:
        raise ActionError("GITHUB_API_URL must use HTTPS")
    if parts.username is not None or parts.password is not None or parts.query or parts.fragment or "?" in value or "#" in value:
        raise ActionError("GITHUB_API_URL must not contain credentials, a query, or a fragment")
    return urllib.parse.urlunsplit((parts.scheme, parts.netloc, parts.path.rstrip("/"), "", ""))


def parse_utc_timestamp(value: Any) -> Optional[int]:
    """Parse a GitHub ISO 8601 timestamp to epoch milliseconds, or None if invalid."""
    match = _UTC_TIMESTAMP.fullmatch(value) if isinstance(value, str) else None
    if match is None:
        return None
    year, month, day, hour, minute, second = (int(part) for part in match.group(1, 2, 3, 4, 5, 6))
    milliseconds = int((match.group(7) or "").ljust(3, "0"))
    offset = timedelta(0)
    if match.group(8) != "Z":
        offset_hours, offset_minutes = int(match.group(10)), int(match.group(11))
        if offset_hours > 23 or offset_minutes > 59:
            return None
        offset = timedelta(hours=offset_hours, minutes=offset_minutes)
        if match.group(9) == "-":
            offset = -offset
    try:
        instant = datetime(year, month, day, hour, minute, second, milliseconds * 1000, tzinfo=timezone(offset))
    except ValueError:
        return None
    return round(instant.timestamp() * 1000)


class _RedirectPolicy(urllib.request.HTTPRedirectHandler):
    """Refuses every redirect unless the call allows one. An allowed redirect
    must target HTTPS. urllib never copies unredirected headers, so
    Authorization does not reach the target."""

    def __init__(self, follow: bool) -> None:
        self._follow = follow

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: D102
        target = urllib.parse.urlsplit(newurl)
        # Plain http is followed only from a loopback-http API base, which only tests use.
        allowed_scheme = target.scheme == "https" or (
            target.scheme == "http"
            and target.hostname in LOOPBACK_HOSTS
            and urllib.parse.urlsplit(req.full_url).scheme == "http"
        )
        if not self._follow or not allowed_scheme:
            return None
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def urllib_transport(request: urllib.request.Request, max_bytes: int, follow_redirect: bool) -> HttpResponse:
    # ProxyHandler({}) ignores *_PROXY variables, as Node's fetch did.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), _RedirectPolicy(follow_redirect))
    try:
        with opener.open(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
            headers = {key.lower(): value for key, value in response.headers.items()}
            return HttpResponse(response.status, headers, response.read(max_bytes + 1))
    except urllib.error.HTTPError as error:
        headers = {key.lower(): value for key, value in error.headers.items()}
        error.close()
        return HttpResponse(error.code, headers, b"")
    except (urllib.error.URLError, OSError) as error:
        reason = getattr(error, "reason", error)
        raise ActionError(f"GitHub API request failed: {reason}") from error


def _decode_json(body: bytes) -> Any:
    def reject_constant(value: str) -> Any:
        raise ValueError(f"non-finite JSON number {value}")

    try:
        return json.loads(body.decode("utf-8"), parse_constant=reject_constant)
    except (UnicodeDecodeError, ValueError) as error:
        raise ActionError("GitHub API returned malformed JSON") from error


class GitHubClient:
    def __init__(
        self,
        *,
        api_base: str,
        token: str,
        repository: str,
        user_agent: str,
        transport: Optional[Transport] = None,
        allow_loopback_http: bool = False,
    ) -> None:
        owner, name = parse_repository(repository, "repository")
        if not token or "\r" in token or "\n" in token:
            raise ActionError("GitHub token is missing or invalid")
        self.repository = f"{owner}/{name}"
        self._repository_url = (
            f"{parse_api_base(api_base, allow_loopback_http=allow_loopback_http)}"
            f"/repos/{urllib.parse.quote(owner, safe='')}/{urllib.parse.quote(name, safe='')}"
        )
        self._token = token
        self._user_agent = user_agent
        self._transport = transport or urllib_transport

    @classmethod
    def from_env(
        cls,
        *,
        user_agent: str,
        repository: Optional[str] = None,
        token_env: str = "GH_TOKEN",
        transport: Optional[Transport] = None,
        allow_loopback_http: bool = False,
    ) -> "GitHubClient":
        """Production callers never pass allow_loopback_http; local-server tests do."""
        return cls(
            api_base=require_env("GITHUB_API_URL"),
            token=require_env(token_env),
            repository=repository if repository is not None else require_env("GITHUB_REPOSITORY"),
            user_agent=user_agent,
            transport=transport,
            allow_loopback_http=allow_loopback_http,
        )

    def get_json(self, path: str, params: Optional[Mapping[str, Any]] = None) -> Any:
        return _decode_json(self._request("GET", path, params=params, max_bytes=MAX_JSON_BYTES).body)

    def get_bytes(self, path: str, *, max_bytes: int, follow_redirect: bool = False) -> bytes:
        return self._request("GET", path, max_bytes=max_bytes, follow_redirect=follow_redirect).body

    def post_json(self, path: str, body: Mapping[str, Any]) -> None:
        payload = json.dumps(body, allow_nan=False, separators=(",", ":")).encode("utf-8")
        self._request("POST", path, data=payload, max_bytes=MAX_JSON_BYTES)

    def _request(
        self,
        method: str,
        path: str,
        *,
        params: Optional[Mapping[str, Any]] = None,
        data: Optional[bytes] = None,
        max_bytes: int,
        follow_redirect: bool = False,
    ) -> HttpResponse:
        if _API_PATH.fullmatch(path) is None or any(segment in (".", "..") for segment in path.split("/")):
            raise ActionError(f"invalid repository API path: {path!r}")
        url = self._repository_url + path
        if params:
            url += "?" + urllib.parse.urlencode([(key, str(value)) for key, value in params.items()])
        request = urllib.request.Request(url, data=data, method=method)
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", API_VERSION)
        request.add_header("User-Agent", self._user_agent)
        if data is not None:
            request.add_header("Content-Type", "application/json")
        request.add_unredirected_header("Authorization", f"Bearer {self._token}")
        response = self._transport(request, max_bytes, follow_redirect)
        if not 200 <= response.status < 300:
            raise ActionError(f"GitHub API request failed with HTTP {response.status}", status=response.status)
        declared = response.headers.get("content-length")
        if declared is not None and (not declared.isdigit() or not declared.isascii() or int(declared) > max_bytes):
            raise ActionError("GitHub API response declared size exceeds the allowed limit")
        if len(response.body) > max_bytes:
            raise ActionError("GitHub API response body exceeded the allowed limit")
        return response


def _has_next_link(link: Optional[str]) -> bool:
    return link is not None and any("next" in relation.split() for relation in _LINK_RELATION.findall(link))


def _is_count(value: Any) -> bool:
    return type(value) is int and 0 <= value <= MAX_SAFE_INTEGER


def paginate(
    client: GitHubClient,
    path: str,
    *,
    items_key: Optional[str],
    max_pages: int,
    params: Optional[Mapping[str, Any]] = None,
    total_count_key: Optional[str] = None,
    per_page: int = 100,
) -> list[Any]:
    """Read every page of a list endpoint. The next request is always built from
    `path` with page=N+1; a Link URL from the server is never followed. Reading
    stops only on a short page without a rel="next" Link, because GitHub may
    return a short page before the end of some lists."""
    if max_pages < 1 or not 1 <= per_page <= 100:
        raise ActionError("pagination limits are invalid")
    items: list[Any] = []
    expected_total: Optional[int] = None
    for page in range(1, max_pages + 1):
        response = client._request(
            "GET", path, params={**(params or {}), "per_page": per_page, "page": page}, max_bytes=MAX_JSON_BYTES
        )
        data = _decode_json(response.body)
        if items_key is None:
            page_items = data
        else:
            page_items = data.get(items_key) if isinstance(data, dict) else None
        if not isinstance(page_items, list) or len(page_items) > per_page:
            raise ActionError(f"GitHub API returned a malformed list page for {path}")
        if total_count_key is not None:
            total = data.get(total_count_key) if isinstance(data, dict) else None
            if not _is_count(total):
                raise ActionError(f"GitHub API returned a malformed total count for {path}")
            if expected_total is None:
                expected_total = total
            elif total != expected_total:
                raise ActionError(f"{path} changed during pagination")
        items.extend(page_items)
        if len(page_items) < per_page and not _has_next_link(response.headers.get("link")):
            if expected_total is not None and len(items) != expected_total:
                raise ActionError(f"{path} item count does not match {total_count_key}")
            return items
    raise ActionError(f"{path} exceeded the pagination limit of {max_pages} pages")


def write_outputs(values: Mapping[str, str]) -> None:
    lines = []
    for name, value in values.items():
        if _OUTPUT_NAME.fullmatch(name) is None or not isinstance(value, str) or "\r" in value or "\n" in value:
            raise ActionError(f"refusing to write an unsafe action output: {name}")
        lines.append(f"{name}={value}\n")
    with open(require_env("GITHUB_OUTPUT"), "a", encoding="utf-8") as output:
        output.write("".join(lines))


def run_action(label: str, main: Callable[[], None]) -> int:
    try:
        main()
    except Exception as error:  # noqa: BLE001 - every failure must fail closed
        print(f"{label} failed closed: {error}", file=sys.stderr)
        return 1
    return 0
