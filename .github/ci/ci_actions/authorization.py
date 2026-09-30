"""compute-authorized: approve only when a current write/admin collaborator
applied the required label in its latest lifecycle event."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from typing import Any, Optional

from ci_actions.github import (
    MAX_SAFE_INTEGER, ActionError, GitHubClient, paginate, parse_positive_int, parse_utc_timestamp,
    require_env, write_outputs,
)

# GitHub's permission field uses legacy base roles: maintain maps to write and
# triage maps to read. role_name carries the specific role and is not used.
ALLOWED_PERMISSIONS = frozenset({"write", "admin"})
KNOWN_PERMISSIONS = frozenset({"none", "read"}) | ALLOWED_PERMISSIONS
LIFECYCLE_EVENTS = frozenset({"labeled", "unlabeled"})
MAX_TIMELINE_PAGES = 1000
USER_AGENT = "aptos-compute-authorization-action"
# GitHub user login format: letters, digits and hyphens only, max 39 characters.
# A bot actor such as "github-actions[bot]" never matches, so it is denied
# before the login reaches the collaborator-permission request path.
_GITHUB_LOGIN = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})")


@dataclass(frozen=True)
class Authorization:
    approved: bool
    approver: Optional[str]
    approval_event_id: Optional[int]


DENIED = Authorization(False, None, None)


@dataclass(frozen=True)
class _LabelEvent:
    id: int
    event: str
    actor: str
    created_at: int


def parse_required_label(value: str) -> str:
    if not value.strip():
        raise ActionError("required_label must not be empty")
    return value


def _valid_login(value: Any) -> bool:
    return isinstance(value, str) and value and value.strip() == value and "\r" not in value and "\n" not in value


def _label_names(pull_request: Any) -> set[str]:
    labels = pull_request.get("labels") if isinstance(pull_request, dict) else None
    if not isinstance(labels, list):
        raise ActionError("Malformed pull request response: labels must be an array")
    names = set()
    for label in labels:
        if not isinstance(label, dict) or not isinstance(label.get("name"), str):
            raise ActionError("Malformed pull request response: label name is missing")
        names.add(label["name"])
    return names


def _matching_event(value: Any, required_label: str) -> Optional[_LabelEvent]:
    if not isinstance(value, dict) or not isinstance(value.get("event"), str):
        raise ActionError("Malformed timeline response: event is missing")
    if value["event"] not in LIFECYCLE_EVENTS:
        return None
    label = value.get("label")
    if not isinstance(label, dict) or not isinstance(label.get("name"), str):
        raise ActionError("Malformed timeline response: lifecycle label is missing")
    if label["name"] != required_label:
        return None
    event_id = value.get("id")
    if type(event_id) is not int or not 0 < event_id <= MAX_SAFE_INTEGER:
        raise ActionError("Malformed timeline response: lifecycle event ID is invalid")
    created_at = parse_utc_timestamp(value.get("created_at"))
    if created_at is None:
        raise ActionError("Malformed timeline response: lifecycle event timestamp is invalid")
    actor = value.get("actor")
    if not isinstance(actor, dict) or not _valid_login(actor.get("login")):
        raise ActionError("Matching label lifecycle event has no valid actor")
    return _LabelEvent(event_id, value["event"], actor["login"], created_at)


def _latest_matching_event(timeline: list[Any], required_label: str) -> _LabelEvent:
    latest: Optional[_LabelEvent] = None
    for value in timeline:
        candidate = _matching_event(value, required_label)
        if candidate is not None and (
            latest is None or (candidate.created_at, candidate.id) > (latest.created_at, latest.id)
        ):
            latest = candidate
    if latest is None:
        raise ActionError(f"No matching label lifecycle event found for {required_label!r}")
    return latest


def _permission(response: Any) -> str:
    if not isinstance(response, dict) or not isinstance(response.get("permission"), str):
        raise ActionError("Malformed collaborator permission response")
    if response["permission"] not in KNOWN_PERMISSIONS:
        raise ActionError(f"Unknown collaborator permission: {response['permission']!r}")
    return response["permission"]


def authorize(client: GitHubClient, pr_number: int, required_label: str) -> Authorization:
    parse_required_label(required_label)
    if required_label not in _label_names(client.get_json(f"/pulls/{pr_number}")):
        return DENIED
    timeline = paginate(client, f"/issues/{pr_number}/timeline", items_key=None, max_pages=MAX_TIMELINE_PAGES)
    latest = _latest_matching_event(timeline, required_label)
    if latest.event != "labeled":
        return DENIED
    if _GITHUB_LOGIN.fullmatch(latest.actor) is None:
        return DENIED
    if _permission(client.get_json(f"/collaborators/{latest.actor}/permission")) not in ALLOWED_PERMISSIONS:
        return DENIED
    return Authorization(True, latest.actor, latest.id)


def _outputs(result: Authorization) -> dict[str, str]:
    return {
        "approved": "true" if result.approved else "false",
        "approver": result.approver or "",
        "approval_event_id": "" if result.approval_event_id is None else str(result.approval_event_id),
    }


def main() -> None:
    try:
        pr_number = parse_positive_int(require_env("INPUT_PR_NUMBER"), "pr_number")
        required_label = parse_required_label(require_env("INPUT_REQUIRED_LABEL"))
        client = GitHubClient.from_env(user_agent=USER_AGENT)
        write_outputs(_outputs(authorize(client, pr_number, required_label)))
    except Exception:
        try:
            write_outputs(_outputs(DENIED))
        except ActionError as output_error:
            print(f"failed to write fail-closed outputs: {output_error}", file=sys.stderr)
        raise
