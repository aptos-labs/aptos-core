"""compute-authorized: approve only when a current write/admin collaborator
applied the required label in its latest lifecycle event."""

from __future__ import annotations

import json
import os
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


def _matching_event(value: Any, required_labels: set[str]) -> Optional[tuple[str, _LabelEvent]]:
    if not isinstance(value, dict) or not isinstance(value.get("event"), str):
        raise ActionError("Malformed timeline response: event is missing")
    if value["event"] not in LIFECYCLE_EVENTS:
        return None
    label = value.get("label")
    if not isinstance(label, dict) or not isinstance(label.get("name"), str):
        raise ActionError("Malformed timeline response: lifecycle label is missing")
    if label["name"] not in required_labels:
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
    return label["name"], _LabelEvent(event_id, value["event"], actor["login"], created_at)


def _latest_matching_events(timeline: list[Any], required_labels: set[str]) -> dict[str, _LabelEvent]:
    latest: dict[str, _LabelEvent] = {}
    for value in timeline:
        match = _matching_event(value, required_labels)
        if match is None:
            continue
        label, candidate = match
        previous = latest.get(label)
        if previous is None or (candidate.created_at, candidate.id) > (previous.created_at, previous.id):
            latest[label] = candidate
    return latest


def _permission(response: Any) -> str:
    if not isinstance(response, dict) or not isinstance(response.get("permission"), str):
        raise ActionError("Malformed collaborator permission response")
    if response["permission"] not in KNOWN_PERMISSIONS:
        raise ActionError(f"Unknown collaborator permission: {response['permission']!r}")
    return response["permission"]


def authorize(client: GitHubClient, pr_number: int, required_label: str) -> Authorization:
    return authorize_labels(client, pr_number, [required_label])[required_label]


def authorize_labels(
    client: GitHubClient, pr_number: int, required_labels: list[str], *, pull_request: Any = None,
) -> dict[str, Authorization]:
    if not isinstance(required_labels, list) or not required_labels:
        raise ActionError("required_labels must be a non-empty array")
    for label in required_labels:
        if not isinstance(label, str):
            raise ActionError("required_labels must contain strings")
        parse_required_label(label)
    if len(set(required_labels)) != len(required_labels):
        raise ActionError("required_labels contains duplicate labels")
    if pull_request is None:
        pull_request = client.get_json(f"/pulls/{pr_number}")
    present = set(required_labels) & _label_names(pull_request)
    results = {label: DENIED for label in required_labels}
    if not present:
        return results
    timeline = paginate(client, f"/issues/{pr_number}/timeline", items_key=None, max_pages=MAX_TIMELINE_PAGES)
    latest = _latest_matching_events(timeline, present)
    permissions: dict[str, str] = {}
    for label in required_labels:
        if label not in present:
            continue
        event = latest.get(label)
        if event is None:
            raise ActionError(f"No matching label lifecycle event found for {label!r}")
        if event.event != "labeled" or _GITHUB_LOGIN.fullmatch(event.actor) is None:
            continue
        if event.actor not in permissions:
            permissions[event.actor] = _permission(client.get_json(f"/collaborators/{event.actor}/permission"))
        if permissions[event.actor] in ALLOWED_PERMISSIONS:
            results[label] = Authorization(True, event.actor, event.id)
    return results


def _outputs(result: Authorization) -> dict[str, str]:
    return {
        "approved": "true" if result.approved else "false",
        "approver": result.approver or "",
        "approval_event_id": "" if result.approval_event_id is None else str(result.approval_event_id),
    }


def _batch_outputs(results: dict[str, Authorization]) -> dict[str, str]:
    return {
        "approved": "true" if any(result.approved for result in results.values()) else "false",
        "approvals": json.dumps({label: _outputs(result) for label, result in results.items()}, separators=(",", ":")),
    }


def _parse_required_labels(value: str) -> list[str]:
    try:
        labels = json.loads(value)
    except json.JSONDecodeError:
        raise ActionError("required_labels must be a JSON array") from None
    if not isinstance(labels, list) or not labels:
        raise ActionError("required_labels must be a non-empty JSON array")
    if any(not isinstance(label, str) for label in labels):
        raise ActionError("required_labels must contain strings")
    return labels


def main() -> None:
    batch = bool(os.environ.get("INPUT_REQUIRED_LABELS"))
    try:
        pr_number = parse_positive_int(require_env("INPUT_PR_NUMBER"), "pr_number")
        if batch and os.environ.get("INPUT_REQUIRED_LABEL"):
            raise ActionError("required_label and required_labels are mutually exclusive")
        if batch:
            labels = _parse_required_labels(require_env("INPUT_REQUIRED_LABELS"))
        else:
            required_label = parse_required_label(require_env("INPUT_REQUIRED_LABEL"))
        client = GitHubClient.from_env(user_agent=USER_AGENT)
        if batch:
            write_outputs(_batch_outputs(authorize_labels(client, pr_number, labels)))
        else:
            write_outputs(_outputs(authorize(client, pr_number, required_label)))
    except Exception:
        try:
            write_outputs(_batch_outputs({}) if batch else _outputs(DENIED))
        except ActionError as output_error:
            print(f"failed to write fail-closed outputs: {output_error}", file=sys.stderr)
        raise
