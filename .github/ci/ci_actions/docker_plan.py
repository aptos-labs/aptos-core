"""Docker capability plan and required-check evaluation.

`.github/ci/docker-capabilities.json` is the only source of Docker IDs. This
module checks the manifest structure, builds the plan that
`docker-build-test.yaml` reads, and evaluates every required Docker check from
the job results. It checks structure only; it does not keep a second copy of
the manifest values.
"""

from __future__ import annotations

import json
import os
import re
import stat
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Mapping

from ci_actions.authorization import Authorization, authorize_labels
from ci_actions.github import (
    ActionError,
    GitHubClient,
    Transport,
    paginate,
    parse_positive_int,
    require_env,
    write_outputs,
)

MANIFEST_PATH = Path(__file__).resolve().parent.parent / "docker-capabilities.json"
MAX_MANIFEST_BYTES = 64 * 1024
MAX_PLAN_BYTES = 64 * 1024
MAX_NEEDS_BYTES = 256 * 1024
# GitHub lists at most 3,000 files for one pull request. A larger PR is never
# treated as documentation-only because the listing would be incomplete.
MAX_PR_FILES = 3000

# Fixed job IDs of docker-build-test.yaml that are not per-workload. The Ruby
# workflow tests assert that the workflow defines exactly these jobs.
AUTHORIZATION_JOB = "compute-authorization"
LOCAL_JOB = "pr-rust-images-local"
PUBLISH_JOB = "pr-publish-rust-images"
NEEDS_CONCLUSIONS = frozenset({"success", "failure", "cancelled", "skipped"})

_CAPABILITY_ID = re.compile(r"[a-z][a-z0-9_]*")
_NAME = re.compile(r"[a-z][a-z0-9-]*")
_LABEL = re.compile(r"CICD:[A-Za-z0-9][A-Za-z0-9-]{0,79}")
_FEATURES = re.compile(r"(?:[a-z0-9][a-z0-9-]*(?:,[a-z0-9][a-z0-9-]*)*)?")
_TITLE = re.compile(r"[A-Za-z0-9][A-Za-z0-9 .,:()-]{0,119}")
_LOGIN = re.compile(r"[^\s]+")


@dataclass(frozen=True)
class Marker:
    id: str
    comment_header: str
    title: str


@dataclass(frozen=True)
class Workload:
    id: str
    docs_sensitive: bool
    checks: tuple[str, ...]
    marker: Marker | None


@dataclass(frozen=True)
class Capability:
    id: str
    label: str
    local: tuple[str, ...]
    publish: tuple[str, ...]
    workloads: tuple[Workload, ...]


@dataclass(frozen=True)
class Manifest:
    image_check: str
    variants: tuple[Mapping[str, str], ...]
    capabilities: tuple[Capability, ...]

    def workloads(self) -> list[Workload]:
        return [workload for capability in self.capabilities for workload in capability.workloads]

    def markers(self) -> list[Marker]:
        return [workload.marker for workload in self.workloads() if workload.marker is not None]

    def workload_checks(self) -> list[str]:
        return list(dict.fromkeys(check for workload in self.workloads() for check in workload.checks))

    def checks(self) -> list[str]:
        """Every required Docker check name: the image check, then workload checks."""
        return [self.image_check, *self.workload_checks()]


def _object(value: Any, required: set[str], optional: set[str], where: str) -> dict:
    if not isinstance(value, dict):
        raise ActionError(f"{where} must be an object")
    unknown = sorted(set(value) - required - optional)
    if unknown:
        raise ActionError(f"{where} has unknown field {unknown[0]!r}")
    missing = sorted(required - set(value))
    if missing:
        raise ActionError(f"{where} is missing field {missing[0]!r}")
    return value


def _string(value: Any, pattern: re.Pattern[str], where: str) -> str:
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ActionError(f"{where} is invalid")
    return value


def _names(value: Any, allowed: Callable[[str], bool], where: str, *, non_empty: bool) -> tuple[str, ...]:
    if not isinstance(value, list) or (non_empty and not value):
        raise ActionError(f"{where} must be a {'non-empty ' if non_empty else ''}list")
    if any(not isinstance(item, str) or not allowed(item) for item in value):
        raise ActionError(f"{where} contains an unknown or invalid ID")
    _unique(value, where)
    return tuple(value)


def _unique(values: list[str], where: str) -> None:
    if len(set(values)) != len(values):
        raise ActionError(f"{where} contains duplicate values")


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict:
    keys = [key for key, _ in pairs]
    if len(set(keys)) != len(keys):
        raise ActionError("JSON object contains a duplicate key")
    return dict(pairs)


def _loads(text: str, what: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_reject_duplicate_keys)
    except json.JSONDecodeError as error:
        raise ActionError(f"{what} must be valid JSON: {error}") from None


def _parse_workload(value: Any, where: str) -> Workload:
    _object(value, {"id", "docs_sensitive", "checks"}, {"marker"}, where)
    if not isinstance(value["docs_sensitive"], bool):
        raise ActionError(f"{where}.docs_sensitive must be boolean")
    marker = None
    if "marker" in value:
        raw = _object(value["marker"], {"id", "comment_header", "title"}, set(), f"{where}.marker")
        marker = Marker(
            id=_string(raw["id"], _NAME, f"{where}.marker.id"),
            comment_header=_string(raw["comment_header"], _NAME, f"{where}.marker.comment_header"),
            title=_string(raw["title"], _TITLE, f"{where}.marker.title"),
        )
    return Workload(
        id=_string(value["id"], _NAME, f"{where}.id"),
        docs_sensitive=value["docs_sensitive"],
        checks=_names(value["checks"], _NAME.fullmatch, f"{where}.checks", non_empty=True),
        marker=marker,
    )


def parse_manifest(text: str) -> Manifest:
    if len(text.encode("utf-8")) > MAX_MANIFEST_BYTES:
        raise ActionError("manifest exceeds size limit")
    if "${{" in text:
        raise ActionError("manifest must not contain a dynamic expression")
    value = _object(_loads(text, "manifest"), {"version", "image_check", "variants", "capabilities"}, set(), "manifest")
    if type(value["version"]) is not int or value["version"] != 1:
        raise ActionError("manifest version must be 1")
    image_check = _string(value["image_check"], _NAME, "manifest.image_check")

    if not isinstance(value["variants"], list) or not value["variants"]:
        raise ActionError("manifest.variants must be a non-empty list")
    variants = []
    for index, raw in enumerate(value["variants"]):
        where = f"variant[{index}]"
        _object(raw, {"id", "profile", "features", "build_target"}, set(), where)
        variants.append({
            "id": _string(raw["id"], _NAME, f"{where}.id"),
            "profile": _string(raw["profile"], _NAME, f"{where}.profile"),
            "features": _string(raw["features"], _FEATURES, f"{where}.features"),
            "build_target": _string(raw["build_target"], _NAME, f"{where}.build_target"),
        })
    variant_ids = [variant["id"] for variant in variants]
    _unique(variant_ids, "variant IDs")
    known_variant = set(variant_ids).__contains__

    if not isinstance(value["capabilities"], list) or not value["capabilities"]:
        raise ActionError("manifest.capabilities must be a non-empty list")
    capabilities = []
    for index, raw in enumerate(value["capabilities"]):
        where = f"capability[{index}]"
        _object(raw, {"id", "label", "local", "publish", "workloads"}, set(), where)
        local = _names(raw["local"], known_variant, f"{where}.local", non_empty=True)
        publish = _names(raw["publish"], known_variant, f"{where}.publish", non_empty=False)
        if not set(publish) <= set(local):
            raise ActionError(f"{where} publication requires the same local variant")
        if not isinstance(raw["workloads"], list):
            raise ActionError(f"{where}.workloads must be a list")
        capabilities.append(Capability(
            id=_string(raw["id"], _CAPABILITY_ID, f"{where}.id"),
            label=_string(raw["label"], _LABEL, f"{where}.label"),
            local=local,
            publish=publish,
            workloads=tuple(
                _parse_workload(workload, f"{where}.workloads[{position}]")
                for position, workload in enumerate(raw["workloads"])
            ),
        ))

    manifest = Manifest(image_check=image_check, variants=tuple(variants), capabilities=tuple(capabilities))
    _unique([capability.id for capability in capabilities], "capability IDs")
    _unique([capability.label for capability in capabilities], "capability labels")
    _unique([marker.comment_header for marker in manifest.markers()], "marker comment headers")
    # Workload IDs are job IDs; marker IDs and check names are check-run names
    # in the same workflow run. No two of them may collide.
    _unique(
        [AUTHORIZATION_JOB, LOCAL_JOB, PUBLISH_JOB]
        + [workload.id for workload in manifest.workloads()]
        + [marker.id for marker in manifest.markers()]
        + manifest.checks(),
        "global job namespace",
    )
    return manifest


def load_manifest(path: Path = MANIFEST_PATH) -> Manifest:
    with open(path, "rb") as handle:
        metadata = os.fstat(handle.fileno())
        if not stat.S_ISREG(metadata.st_mode):
            raise ActionError("manifest must be a regular file")
        if metadata.st_size > MAX_MANIFEST_BYTES:
            raise ActionError("manifest exceeds size limit")
        data = handle.read(MAX_MANIFEST_BYTES + 1)
    if len(data) != metadata.st_size:
        raise ActionError("manifest changed while it was being read")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        raise ActionError("manifest must be UTF-8") from None
    return parse_manifest(text)


def _check_approval(value: Any, capability_id: str) -> None:
    if not isinstance(value, Authorization) or not isinstance(value.approved, bool):
        raise ActionError(f"approval {capability_id} is invalid")
    if value.approved:
        event_id = value.approval_event_id
        if not isinstance(value.approver, str) or not _LOGIN.fullmatch(value.approver):
            raise ActionError(f"approval {capability_id} has an invalid approver")
        if type(event_id) is not int or not 0 < event_id <= 2**53 - 1:
            raise ActionError(f"approval {capability_id} has an invalid event ID")
    elif value.approver is not None or value.approval_event_id is not None:
        raise ActionError(f"approval {capability_id} must clear denied metadata")


def build_plan(manifest: Manifest, approvals: Mapping[str, Authorization], docs_only: bool) -> dict:
    """Return the plan. Every `active`/`enabled` value in it is final."""
    if set(approvals) != {capability.id for capability in manifest.capabilities}:
        raise ActionError("approval IDs must exactly match manifest capabilities")
    if not isinstance(docs_only, bool):
        raise ActionError("docs_only must be boolean")
    local_ids: set[str] = set()
    publish_ids: set[str] = set()
    approval_values = {}
    workloads = {}
    markers = []
    for capability in manifest.capabilities:
        approval = approvals[capability.id]
        _check_approval(approval, capability.id)
        approval_values[capability.id] = {
            "approved": approval.approved,
            "approver": approval.approver,
            "approval_event_id": approval.approval_event_id,
        }
        if approval.approved:
            local_ids.update(capability.local)
            publish_ids.update(capability.publish)
        for workload in capability.workloads:
            active = approval.approved and not (workload.docs_sensitive and docs_only)
            workloads[workload.id] = active
            if active and workload.marker is not None:
                markers.append({"marker": workload.marker.id, "workload": workload.id})
    local = [dict(variant) for variant in manifest.variants if variant["id"] in local_ids]
    publish = [dict(variant) for variant in manifest.variants if variant["id"] in publish_ids]
    return {
        "docs_only": docs_only,
        "approvals": approval_values,
        "local": {"enabled": bool(local), "include": local},
        "publish": {"enabled": bool(publish), "include": publish},
        "workloads": workloads,
        "markers": {"enabled": bool(markers), "include": markers},
    }


def serialize_plan(plan: Mapping[str, Any]) -> str:
    value = json.dumps(plan, separators=(",", ":"), ensure_ascii=True)
    if len(value) > MAX_PLAN_BYTES:
        raise ActionError("plan exceeds size limit")
    if "${{" in value:
        raise ActionError("plan contains unsafe output data")
    return value


def parse_plan(manifest: Manifest, text: Any) -> dict:
    """Rebuild the plan from its own approvals and docs flag; require an exact match."""
    if not isinstance(text, str) or not text:
        raise ActionError("validated Docker capability plan is missing")
    if len(text.encode("utf-8")) > MAX_PLAN_BYTES or "${{" in text:
        raise ActionError("validated Docker capability plan is unsafe")
    try:
        supplied = _loads(text, "plan")
        approvals = {
            capability.id: Authorization(
                approved=supplied["approvals"][capability.id]["approved"],
                approver=supplied["approvals"][capability.id]["approver"],
                approval_event_id=supplied["approvals"][capability.id]["approval_event_id"],
            )
            for capability in manifest.capabilities
        }
        expected = build_plan(manifest, approvals, supplied["docs_only"])
    except (ActionError, KeyError, TypeError) as error:
        raise ActionError(f"validated Docker capability plan is inconsistent: {error}") from None
    if serialize_plan(expected) != text:
        raise ActionError("validated Docker capability plan does not match its approvals and manifest")
    return expected


def pull_request_is_docs_only(client: GitHubClient, pr_number: int, *, pull_request: Any = None) -> bool:
    """Port of the retired pr-target-determination rule: at least one changed
    file, and every changed filename ends with `.md`. A renamed file must
    also have a `previous_filename` that ends with `.md`."""
    if pull_request is None:
        pull_request = client.get_json(f"/pulls/{pr_number}")
    changed = pull_request.get("changed_files") if isinstance(pull_request, dict) else None
    if type(changed) is not int or changed < 0:
        raise ActionError("pull request changed_files is invalid")
    if changed == 0 or changed >= MAX_PR_FILES:
        return False
    files = paginate(client, f"/pulls/{pr_number}/files", items_key=None, max_pages=MAX_PR_FILES // 100)
    if len(files) != changed:
        raise ActionError("pull request file list does not match changed_files")
    names = [entry.get("filename") if isinstance(entry, dict) else None for entry in files]
    if any(not isinstance(name, str) or not name for name in names):
        raise ActionError("pull request file list contains an invalid filename")
    if not all(name.endswith(".md") for name in names):
        return False
    previous = [entry["previous_filename"] for entry in files if "previous_filename" in entry]
    return all(isinstance(name, str) and name.endswith(".md") for name in previous)


def compute_plan(
    manifest: Manifest,
    authorize_label: Callable[[str], Authorization],
    docs_only: Callable[[], bool],
) -> dict:
    approvals = {capability.id: authorize_label(capability.label) for capability in manifest.capabilities}
    needs_docs = any(
        approvals[capability.id].approved and workload.docs_sensitive
        for capability in manifest.capabilities
        for workload in capability.workloads
    )
    return build_plan(manifest, approvals, docs_only() if needs_docs else False)


def plan_main(transport: Transport | None = None) -> None:
    pr_number = parse_positive_int(require_env("INPUT_PR_NUMBER"), "pr_number")
    manifest = load_manifest()
    client = GitHubClient.from_env(user_agent="aptos-docker-capability-plan", transport=transport)
    pull_request = client.get_json(f"/pulls/{pr_number}")
    label_approvals = authorize_labels(
        client, pr_number, [capability.label for capability in manifest.capabilities], pull_request=pull_request,
    )
    plan = compute_plan(
        manifest,
        label_approvals.__getitem__,
        lambda: pull_request_is_docs_only(client, pr_number, pull_request=pull_request),
    )
    write_outputs({"plan": serialize_plan(plan)})


def _conclusion(needs: Mapping[str, Any], job: str) -> str:
    entry = needs[job]
    result = entry.get("result") if isinstance(entry, dict) else None
    if not isinstance(result, str) or result not in NEEDS_CONCLUSIONS:
        raise ActionError(f"job {job} has an invalid job conclusion")
    return result


def _expected(active: bool, result: str) -> bool:
    return result == ("success" if active else "skipped")


def evaluate_statuses(manifest: Manifest, needs: Any) -> dict[str, bool]:
    """Map every required check name to pass (True) or fail (False)."""
    workloads = manifest.workloads()
    jobs = {AUTHORIZATION_JOB, LOCAL_JOB, PUBLISH_JOB} | {workload.id for workload in workloads}
    if not isinstance(needs, dict) or set(needs) != jobs:
        raise ActionError("needs must list exactly the authorization, image, and manifest workload jobs")
    results = {job: _conclusion(needs, job) for job in sorted(jobs)}
    if results[AUTHORIZATION_JOB] != "success":
        return {check: False for check in manifest.checks()}

    outputs = needs[AUTHORIZATION_JOB].get("outputs")
    plan = parse_plan(manifest, outputs.get("plan") if isinstance(outputs, dict) else None)
    statuses = {
        manifest.image_check: _expected(plan["local"]["enabled"], results[LOCAL_JOB])
        and _expected(plan["publish"]["enabled"], results[PUBLISH_JOB])
    }
    for check in manifest.workload_checks():
        statuses[check] = all(
            _expected(plan["workloads"][workload.id], results[workload.id])
            for workload in workloads
            if check in workload.checks
        )
    return statuses


def status_main() -> None:
    # toJSON(needs) is multi-line, so require_env (which rejects newlines) does not apply.
    raw = os.environ.get("INPUT_NEEDS", "")
    if not raw:
        raise ActionError("Missing required environment variable: INPUT_NEEDS")
    if len(raw.encode("utf-8")) > MAX_NEEDS_BYTES:
        raise ActionError("needs exceeds size limit")
    statuses = evaluate_statuses(load_manifest(), _loads(raw, "needs"))
    write_outputs({"statuses": json.dumps(statuses, separators=(",", ":"))})
