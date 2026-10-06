"""Tests for ci_actions.docker_plan (docker-capability-plan and docker-status-plan)."""

from __future__ import annotations

import contextlib
import copy
import io
import itertools
import json
import unittest
from unittest import mock

from hypothesis import given, strategies as st

from tests.property_support import configure_profiles
from tests.planning_support import expected_plan, expected_statuses, manifest_graphs

configure_profiles()

from ci_actions import docker_plan
from ci_actions.authorization import Authorization
from ci_actions.docker_plan import (
    AUTHORIZATION_JOB,
    LOCAL_JOB,
    MAX_MANIFEST_BYTES,
    MAX_NEEDS_BYTES,
    MAX_PLAN_BYTES,
    PUBLISH_JOB,
    build_plan,
    evaluate_statuses,
    load_manifest,
    parse_manifest,
    parse_plan,
    pull_request_is_docs_only,
    serialize_plan,
)
from ci_actions.github import ActionError
from tests.helpers import REPO_PATH, RouteTransport, action_env, read_outputs, route_client

MANIFEST = load_manifest()
RAW = json.loads(docker_plan.MANIFEST_PATH.read_text(encoding="utf-8"))
DENIED = Authorization(approved=False, approver=None, approval_event_id=None)


def granted(index: int) -> Authorization:
    return Authorization(approved=True, approver=f"maintainer-{index}", approval_event_id=10_000 + index)


def approvals(approved_ids=()) -> dict[str, Authorization]:
    return {
        capability.id: granted(index) if capability.id in approved_ids else DENIED
        for index, capability in enumerate(MANIFEST.capabilities)
    }


def mutated(mutator) -> str:
    value = copy.deepcopy(RAW)
    mutator(value)
    return json.dumps(value)


def capability(value: dict, capability_id: str) -> dict:
    return next(item for item in value["capabilities"] if item["id"] == capability_id)


PULL = f"{REPO_PATH}/pulls/42"
FILES_PAGE = f"{REPO_PATH}/pulls/42/files?per_page=100&page="


def docs_routes(filenames, changed=None) -> dict:
    files = [{"filename": name} for name in filenames]
    routes = {PULL: {"number": 42, "changed_files": len(files) if changed is None else changed}}
    for page in range(1, len(files) // 100 + 2):
        routes[f"{FILES_PAGE}{page}"] = files[(page - 1) * 100:page * 100]
    return routes


class ManifestTests(unittest.TestCase):
    def test_workload_image_requirements_are_required_and_validated(self):
        def first_workload(value):
            return capability(value, "run_e2e")["workloads"][0]

        cases = [
            (lambda v: first_workload(v).pop("image_variant"), "missing field 'image_variant'"),
            (lambda v: first_workload(v).pop("additional_testing_images"), "missing field 'additional_testing_images'"),
            (lambda v: first_workload(v).update(image_variant="unknown"), "image_variant.*unknown"),
            (lambda v: first_workload(v).update(additional_testing_images="true"), "additional_testing_images must be boolean"),
        ]
        for mutator, error in cases:
            with self.subTest(error=error), self.assertRaisesRegex(ActionError, error):
                parse_manifest(mutated(mutator))

    def test_unknown_fields_are_rejected_at_every_level(self):
        cases = [
            lambda v: v.update(unknown=True),
            lambda v: v["variants"][0].update(unknown=True),
            lambda v: v["capabilities"][0].update(unknown=True),
            lambda v: capability(v, "run_e2e")["workloads"][0].update(unknown=True),
            lambda v: capability(v, "run_e2e")["workloads"][1]["marker"].update(unknown=True),
            lambda v: v["variants"][0].update(local_job="pr-rust-images-local"),
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "unknown field"):
                parse_manifest(mutated(mutator))

    def test_duplicate_and_unknown_references_are_rejected(self):
        cases = [
            lambda v: v["variants"][1].update(id="release"),
            lambda v: v["capabilities"][1].update(id="build_images"),
            lambda v: v["capabilities"][1].update(label=v["capabilities"][0]["label"]),
            lambda v: v["capabilities"][0]["local"].append("release"),
            lambda v: v["capabilities"][0].update(local=["unknown"]),
            lambda v: v["capabilities"][0].update(local=[], publish=[]),
            lambda v: capability(v, "run_e2e")["workloads"][0]["checks"].append("cli-e2e-tests"),
            lambda v: capability(v, "run_e2e")["workloads"][0].update(checks=[]),
            lambda v: capability(v, "run_multiregion")["workloads"][0]["marker"].update(comment_header="forge-e2e"),
            lambda v: capability(v, "build_failpoints").update(local=["release"], publish=["failpoints"]),
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "duplicate|unknown|list|subset"):
                parse_manifest(mutated(mutator))

    def test_global_job_namespace_rejects_collisions(self):
        def workload(value):
            return capability(value, "run_e2e")["workloads"][1]

        cases = [
            lambda v: workload(v).update(id="pr-rust-images-local"),
            lambda v: workload(v).update(id="pr-publish-rust-images"),
            lambda v: workload(v).update(id="compute-authorization"),
            lambda v: workload(v).update(id="forge-report-source-compat"),
            lambda v: workload(v).update(id="forge-compat-test"),
            lambda v: workload(v).update(id="rust-images"),
            lambda v: workload(v)["marker"].update(id="pr-forge-compat"),
            lambda v: workload(v).update(checks=["rust-images"]),
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "global job namespace"):
                parse_manifest(mutated(mutator))

    def test_fields_must_match_safe_patterns(self):
        cases = [
            lambda v: v["variants"][0].update(profile="Release!"),
            lambda v: v["variants"][0].update(features="a b"),
            lambda v: v["variants"][0].update(build_target="../all"),
            lambda v: v["capabilities"][0].update(label="build-images"),
            lambda v: v["capabilities"][0].update(id="Build-Images"),
            lambda v: capability(v, "run_e2e")["workloads"][1]["marker"].update(title="`inject`"),
            lambda v: capability(v, "run_e2e")["workloads"][0].update(docs_sensitive="true"),
            lambda v: v.update(version=True),
            lambda v: v.update(image_check="Rust Images"),
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "invalid|boolean|version"):
                parse_manifest(mutated(mutator))

    def test_unsafe_malformed_and_oversized_text_is_rejected(self):
        with self.assertRaisesRegex(ActionError, "dynamic expression"):
            parse_manifest(mutated(lambda v: v["capabilities"][0].update(label="${{ github.event.pull_request.title }}")))
        with self.assertRaisesRegex(ActionError, "duplicate key"):
            parse_manifest('{"version": 1, "version": 1}')
        with self.assertRaisesRegex(ActionError, "manifest must be valid JSON"):
            parse_manifest("{bad")
        with self.assertRaisesRegex(ActionError, "size limit"):
            parse_manifest(" " * (MAX_MANIFEST_BYTES + 1))


class PlanTests(unittest.TestCase):
    def test_every_approval_subset_selects_exact_variants_and_workloads(self):
        ids = [capability.id for capability in MANIFEST.capabilities]
        for mask in range(1 << len(ids)):
            approved = approvals({ids[i] for i in range(len(ids)) if mask & (1 << i)})
            for docs_only, protected in itertools.product((False, True), (False, True)):
                where = f"{mask}:{docs_only}:{protected}"
                plan = build_plan(MANIFEST, approved, docs_only, protected_runners=protected)
                self.assertEqual(expected_plan(RAW, approved, docs_only, protected), plan, where)
                self.assertEqual(plan, parse_plan(MANIFEST, serialize_plan(plan)), where)

    def test_hand_computed_plans_for_failpoints_and_e2e(self):
        # Independent of planning_support.expected_plan, which mirrors build_plan.
        e2e = ["pr-node-cli-faucet-tests", "pr-forge-e2e", "pr-forge-compat"]

        def summary(docs_only, protected):
            plan = build_plan(MANIFEST, approvals({"build_failpoints", "run_e2e"}), docs_only,
                              protected_runners=protected)
            return ([v["id"] for v in plan["local"]["include"]],
                    [(v["id"], v["additional_testing_images"]) for v in plan["publish"]["include"]],
                    [workload for workload, active in plan["workloads"].items() if active], plan["blocked"])

        self.assertEqual(([], [("release", True), ("failpoints", False)], e2e, []), summary(False, True))
        self.assertEqual((["release"], [("failpoints", False)], [], []), summary(True, True))
        self.assertEqual((["release", "failpoints"], [], [], e2e), summary(False, False))
        self.assertEqual((["release", "failpoints"], [], [], []), summary(True, False))

    def test_serialize_plan_enforces_the_size_limit(self):
        padding = MAX_PLAN_BYTES - len(serialize_plan({"x": ""}))
        self.assertEqual(MAX_PLAN_BYTES, len(serialize_plan({"x": "a" * padding})))
        with self.assertRaisesRegex(ActionError, "plan exceeds size limit"):
            serialize_plan({"x": "a" * (padding + 1)})

    def test_build_plan_requires_exact_valid_approvals(self):
        missing = approvals()
        del missing["run_e2e"]
        with self.assertRaisesRegex(ActionError, "approval IDs"):
            build_plan(MANIFEST, missing, False, protected_runners=True)
        with self.assertRaisesRegex(ActionError, "approval IDs"):
            build_plan(MANIFEST, {**approvals(), "surprise": DENIED}, False, protected_runners=True)
        for bad in [
            Authorization(approved="true", approver="maintainer", approval_event_id=1),
            Authorization(approved=True, approver="", approval_event_id=1),
            Authorization(approved=True, approver="maintainer\nunsafe", approval_event_id=1),
            Authorization(approved=True, approver="bad login", approval_event_id=1),
            Authorization(approved=True, approver="maintainer", approval_event_id=0),
            Authorization(approved=True, approver="maintainer", approval_event_id=True),
            Authorization(approved=True, approver="maintainer", approval_event_id=2**53),
            Authorization(approved=False, approver="maintainer", approval_event_id=None),
            Authorization(approved=False, approver=None, approval_event_id=1),
        ]:
            with self.subTest(bad=bad), self.assertRaisesRegex(ActionError, "approval"):
                build_plan(MANIFEST, {**approvals(), "build_images": bad}, False, protected_runners=True)
        with self.assertRaisesRegex(ActionError, "docs_only"):
            build_plan(MANIFEST, approvals(), "false", protected_runners=True)
        for bad in ("true", 1, None):
            with self.subTest(protected_runners=bad), self.assertRaisesRegex(ActionError, "protected_runners"):
                build_plan(MANIFEST, approvals(), False, protected_runners=bad)

    def test_tampered_plans_are_rejected(self):
        approved = approvals({"build_failpoints", "run_e2e"})
        enabled = json.loads(serialize_plan(build_plan(MANIFEST, approved, False, protected_runners=True)))
        disabled = json.loads(serialize_plan(build_plan(MANIFEST, approved, False, protected_runners=False)))
        tampered = [
            (enabled, lambda p: p["publish"].update(
                include=[v for v in p["publish"]["include"] if v["id"] != "failpoints"])),
            (enabled, lambda p: p["workloads"].update({"pr-forge-e2e": False})),
            (enabled, lambda p: p.update(docs_only="yes")),
            (enabled, lambda p: p["approvals"].pop("run_e2e")),
            (enabled, lambda p: p.update(extra=1)),
            (enabled, lambda p: p.update(protected_runners=False)),
            (enabled, lambda p: p.pop("protected_runners")),
            (enabled, lambda p: p.update(blocked=["pr-forge-e2e"])),
            (disabled, lambda p: p.update(protected_runners=True)),
            (disabled, lambda p: p.update(blocked=[])),
            (disabled, lambda p: p.pop("blocked")),
            (disabled, lambda p: p["workloads"].update({"pr-forge-e2e": True})),
        ]
        for plan, mutator in tampered:
            value = copy.deepcopy(plan)
            mutator(value)
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "validated Docker capability plan"):
                parse_plan(MANIFEST, json.dumps(value, separators=(",", ":")))
        text = serialize_plan(enabled)
        for bad in ["", None, text.replace("}", "} ", 1), text + "${{"]:
            with self.subTest(bad=bad), self.assertRaisesRegex(ActionError, "validated Docker capability plan"):
                parse_plan(MANIFEST, bad)


class DocsOnlyTests(unittest.TestCase):
    def docs_only(self, routes):
        client, transport = route_client(routes)
        return pull_request_is_docs_only(client, 42), transport

    def test_pull_requests_beyond_the_listing_cap_are_not_docs_only(self):
        result, transport = self.docs_only(docs_routes(["README.md"], changed=3000))
        self.assertFalse(result)
        self.assertEqual([PULL], transport.urls())

    def test_incomplete_or_malformed_listing_fails_closed(self):
        with self.assertRaisesRegex(ActionError, "does not match changed_files"):
            self.docs_only(docs_routes(["README.md"], changed=2))
        with self.assertRaisesRegex(ActionError, "changed_files is invalid"):
            self.docs_only(docs_routes(["README.md"], changed="1"))
        with self.assertRaisesRegex(ActionError, "invalid filename"):
            self.docs_only({PULL: {"changed_files": 1}, f"{FILES_PAGE}1": [{"filename": 7}]})


def labeled_routes(labels, filenames) -> dict:
    routes = docs_routes(filenames)
    routes[PULL]["labels"] = [{"name": label} for label in labels]
    routes[f"{REPO_PATH}/issues/42/timeline?per_page=100&page=1"] = [
        {"id": index, "event": "labeled", "label": {"name": label},
         "actor": {"login": "trusted-maintainer"}, "created_at": "2026-09-17T10:03:00Z"}
        for index, label in enumerate(labels, 1)
    ]
    routes[f"{REPO_PATH}/collaborators/trusted-maintainer/permission"] = {"permission": "write"}
    return routes


class PlanMainTests(unittest.TestCase):
    def test_plan_main_authorizes_each_manifest_label_and_writes_plan(self):
        transport = RouteTransport(labeled_routes(("CICD:build-failpoints-images", "CICD:run-e2e-tests"), ["README.md"]))
        with action_env(INPUT_PR_NUMBER="42", INPUT_PROTECTED_RUNNERS_ENABLED="true") as output:
            docker_plan.plan_main(transport=transport)
            outputs = read_outputs(output)
        permission = f"{REPO_PATH}/collaborators/trusted-maintainer/permission"
        self.assertEqual(1, transport.urls().count(PULL))
        self.assertEqual(1, transport.urls().count(permission))
        self.assertEqual(4, len(transport.urls()))
        self.assertEqual(["plan"], list(outputs))
        plan = parse_plan(MANIFEST, outputs["plan"])
        self.assertTrue(plan["docs_only"])
        self.assertEqual({"build_failpoints", "run_e2e"},
                         {cid for cid, approval in plan["approvals"].items() if approval["approved"]})

    def test_plan_main_blocks_requested_workloads_without_protected_runners(self):
        transport = RouteTransport(labeled_routes(("CICD:run-e2e-tests",), ["src/lib.rs"]))
        with action_env(INPUT_PR_NUMBER="42", INPUT_PROTECTED_RUNNERS_ENABLED="false") as output:
            docker_plan.plan_main(transport=transport)
            plan = parse_plan(MANIFEST, read_outputs(output)["plan"])
        self.assertFalse(plan["protected_runners"])
        self.assertEqual(["pr-node-cli-faucet-tests", "pr-forge-e2e", "pr-forge-compat"], plan["blocked"])

    def test_plan_main_skips_the_file_listing_without_a_docs_sensitive_approval(self):
        transport = RouteTransport({PULL: {"labels": [], "changed_files": 1}})
        with action_env(INPUT_PR_NUMBER="42", INPUT_PROTECTED_RUNNERS_ENABLED="true") as output:
            docker_plan.plan_main(transport=transport)
            self.assertFalse(json.loads(read_outputs(output)["plan"])["docs_only"])
        self.assertEqual([PULL], transport.urls())

    def test_plan_main_fails_closed_before_api_access_for_invalid_input(self):
        cases = [{"INPUT_PR_NUMBER": "0", "INPUT_PROTECTED_RUNNERS_ENABLED": "true"},
                 {"INPUT_PR_NUMBER": "42"}]
        cases += [{"INPUT_PR_NUMBER": "42", "INPUT_PROTECTED_RUNNERS_ENABLED": flag}
                  for flag in ("", "True", "TRUE", "1", "yes", " true", "true\n")]
        for env in cases:
            transport = RouteTransport({})
            with self.subTest(env=env), action_env(**env) as output:
                with self.assertRaises(ActionError):
                    docker_plan.plan_main(transport=transport)
                self.assertEqual("", output.read_text())
            self.assertEqual([], transport.requests)


def needs_for(approved=(), docs_only=False, protected_runners=True, **overrides) -> dict:
    plan = build_plan(MANIFEST, approvals(approved), docs_only, protected_runners=protected_runners)
    needs = {
        AUTHORIZATION_JOB: {"result": "success", "outputs": {"plan": serialize_plan(plan)}},
        LOCAL_JOB: {"result": "success" if plan["local"]["enabled"] else "skipped", "outputs": {}},
        PUBLISH_JOB: {"result": "success" if plan["publish"]["enabled"] else "skipped", "outputs": {}},
    }
    for job, active in plan["workloads"].items():
        needs[job] = {"result": "success" if active else "skipped", "outputs": {}}
    for job, result in overrides.items():
        needs[job.replace("_", "-")]["result"] = result
    return needs


def all_checks(value: bool, **overrides: bool) -> dict[str, bool]:
    statuses = {check: value for check in MANIFEST.checks()}
    statuses.update({check.replace("_", "-"): result for check, result in overrides.items()})
    return statuses


class StatusTests(unittest.TestCase):
    def test_enabled_workloads_fail_when_protected_publication_does_not_succeed(self):
        for result in ("failure", "cancelled", "skipped"):
            with self.subTest(result=result):
                needs = needs_for({"run_e2e"}, pr_publish_rust_images=result)
                self.assertEqual(
                    all_checks(True, rust_images=False, node_api_compatibility_tests=False,
                               cli_e2e_tests=False, faucet_tests_main=False,
                               forge_e2e_test=False, forge_compat_test=False),
                    evaluate_statuses(MANIFEST, needs),
                )

    def test_inactive_workload_that_ran_fails_its_checks(self):
        needs = needs_for({"run_e2e"}, docs_only=True)
        self.assertEqual(all_checks(True), evaluate_statuses(MANIFEST, needs))
        needs = needs_for({"run_e2e"}, docs_only=True, pr_forge_e2e="failure")
        self.assertFalse(evaluate_statuses(MANIFEST, needs)["forge-e2e-test"])
        needs = needs_for((), pr_forge_multiregion="success")
        self.assertFalse(evaluate_statuses(MANIFEST, needs)["forge-multiregion-test"])

    def test_authorization_failure_fails_every_check_without_reading_a_plan(self):
        for result in ("failure", "cancelled", "skipped"):
            needs = needs_for()
            needs[AUTHORIZATION_JOB] = {"result": result, "outputs": {}}
            self.assertEqual(all_checks(False), evaluate_statuses(MANIFEST, needs))

    def test_tampered_plan_fails_closed(self):
        needs = needs_for({"build_failpoints"})
        plan = json.loads(needs[AUTHORIZATION_JOB]["outputs"]["plan"])
        plan["local"]["include"] = [v for v in plan["local"]["include"] if v["id"] != "release"]
        needs[AUTHORIZATION_JOB]["outputs"]["plan"] = json.dumps(plan, separators=(",", ":"))
        with self.assertRaisesRegex(ActionError, "validated Docker capability plan"):
            evaluate_statuses(MANIFEST, needs)

    def test_unknown_job_conclusions_fail_closed(self):
        with self.assertRaisesRegex(ActionError, "job conclusion"):
            evaluate_statuses(MANIFEST, needs_for({"run_framework_upgrade"}, pr_forge_framework_upgrade="neutral"))

    def test_needs_must_list_exactly_the_manifest_jobs(self):
        extra = needs_for()
        extra["pr-forge-renamed"] = {"result": "skipped", "outputs": {}}
        missing = needs_for()
        del missing["pr-forge-e2e"]
        for needs in (extra, missing, [], None):
            with self.subTest(needs=needs), self.assertRaisesRegex(ActionError, "needs must list exactly"):
                evaluate_statuses(MANIFEST, needs)

    def test_status_main_reads_pretty_printed_needs_and_writes_one_map(self):
        needs = json.dumps(needs_for({"run_multiregion"}), indent=2)
        with action_env(INPUT_NEEDS=needs) as output:
            docker_plan.status_main()
            outputs = read_outputs(output)
        self.assertEqual(["statuses"], list(outputs))
        self.assertEqual(all_checks(True), json.loads(outputs["statuses"]))

    def test_disabled_runners_fail_every_check_of_a_requested_workload(self):
        needs = needs_for({"run_e2e", "run_multiregion"}, protected_runners=False)
        self.assertEqual(
            all_checks(True, node_api_compatibility_tests=False, cli_e2e_tests=False, faucet_tests_main=False,
                       forge_e2e_test=False, forge_compat_test=False, forge_multiregion_test=False),
            evaluate_statuses(MANIFEST, needs),
        )
        # A blocked workload's job must have been skipped; a run that somehow
        # succeeded still cannot turn its checks green.
        ran = needs_for({"run_multiregion"}, protected_runners=False, pr_forge_multiregion="success")
        self.assertFalse(evaluate_statuses(MANIFEST, ran)["forge-multiregion-test"])

    def test_flag_change_mid_run_fails_requested_workloads_closed(self):
        # The plan saw protected runners, but the inner rollout gate then skipped
        # publication. This assumes GitHub reports such a call as skipped (unverified).
        needs = needs_for({"run_e2e"}, pr_publish_rust_images="skipped",
                          pr_node_cli_faucet_tests="skipped", pr_forge_e2e="skipped", pr_forge_compat="skipped")
        self.assertEqual(
            all_checks(True, rust_images=False, node_api_compatibility_tests=False, cli_e2e_tests=False,
                       faucet_tests_main=False, forge_e2e_test=False, forge_compat_test=False),
            evaluate_statuses(MANIFEST, needs),
        )

    def test_status_main_annotates_each_blocked_workload(self):
        for protected, blocked in ((False, ["pr-node-cli-faucet-tests", "pr-forge-e2e", "pr-forge-compat"]),
                                   (True, [])):
            stdout = io.StringIO()
            needs = json.dumps(needs_for({"run_e2e"}, protected_runners=protected))
            with self.subTest(protected=protected), action_env(INPUT_NEEDS=needs) as output, \
                    contextlib.redirect_stdout(stdout):
                docker_plan.status_main()
                self.assertIs(protected, json.loads(read_outputs(output)["statuses"])["forge-e2e-test"])
            errors = [line for line in stdout.getvalue().splitlines() if line.startswith("::error")]
            self.assertEqual(len(blocked), len(errors))
            for workload, line in zip(blocked, errors):
                self.assertIn(workload, line)

    def test_status_main_rejects_needs_over_the_size_limit(self):
        needs = json.dumps(needs_for())
        for size in (MAX_NEEDS_BYTES, MAX_NEEDS_BYTES + 1):
            with self.subTest(size=size), action_env(INPUT_NEEDS=needs.ljust(size)) as output:
                if size <= MAX_NEEDS_BYTES:
                    docker_plan.status_main()
                    self.assertEqual(["statuses"], list(read_outputs(output)))
                else:
                    with self.assertRaisesRegex(ActionError, "needs exceeds size limit"):
                        docker_plan.status_main()
                    self.assertEqual("", output.read_text())


def graph_auth(approved: dict[str, bool]) -> dict[str, Authorization]:
    return {cid: granted(i) if flag else DENIED for i, (cid, flag) in enumerate(approved.items())}


class PlanningPropertyTests(unittest.TestCase):
    @given(manifest_graphs(), st.booleans())
    def test_generated_graph_has_exact_plan_and_canonical_serialization(self, graph, protected):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = graph_auth(approved)
        expected = expected_plan(raw, auth, docs_only, protected)
        actual = build_plan(manifest, auth, docs_only, protected_runners=protected)
        self.assertEqual(expected, actual)
        canonical = json.dumps(expected, separators=(",", ":"), ensure_ascii=True)
        self.assertEqual(canonical, serialize_plan(actual))
        self.assertEqual(expected, parse_plan(manifest, canonical))
        # Whitespace and key order are part of the handoff contract.
        for altered in (json.dumps(expected, indent=2), json.dumps(expected, sort_keys=True)):
            if altered != canonical:
                with self.assertRaises(ActionError):
                    parse_plan(manifest, altered)
        local = {v["id"] for v in actual["local"]["include"]}
        publish = {v["id"] for v in actual["publish"]["include"]}
        requested_local = {v for c in raw["capabilities"] if approved[c["id"]] for v in c["local"]}
        if protected:
            self.assertEqual(set(), local & publish)
            approved_publish = {v for c in raw["capabilities"] if approved[c["id"]] for v in c["publish"]}
            workload_variants = {w["image_variant"] for c in raw["capabilities"] if approved[c["id"]]
                                 for w in c["workloads"] if not (w["docs_sensitive"] and docs_only)}
            self.assertEqual(requested_local | approved_publish | workload_variants, local | publish)
            self.assertEqual([], actual["blocked"])
        else:
            self.assertEqual(requested_local, local)
            self.assertEqual(set(), publish)
            self.assertFalse(any(actual["workloads"].values()))
            self.assertFalse(actual["markers"]["enabled"])

    @given(manifest_graphs())
    def test_generated_manifest_mutations_fail_closed(self, graph):
        raw = graph[0]
        parse_manifest(json.dumps(raw))
        mutations = []
        extra = copy.deepcopy(raw)
        extra["unknown"] = True
        mutations.append(extra)
        for key in ("variants", "capabilities"):
            duplicate = copy.deepcopy(raw)
            duplicate[key].append(copy.deepcopy(duplicate[key][0]))
            mutations.append(duplicate)
        for change in ({"local": []}, {"local": ["unknown-variant"]}, {"publish": ["unknown-variant"]},
                       {"label": "${{ injected }}"}):
            candidate = copy.deepcopy(raw)
            candidate["capabilities"][0].update(change)
            mutations.append(candidate)
        duplicate_local = copy.deepcopy(raw)
        duplicate_local["capabilities"][0]["local"].append(duplicate_local["capabilities"][0]["local"][0])
        mutations.append(duplicate_local)
        if len(raw["variants"]) > 1:
            outside_local = copy.deepcopy(raw)
            outside_local["capabilities"][0].update(local=[raw["variants"][0]["id"]],
                                                    publish=[raw["variants"][1]["id"]])
            mutations.append(outside_local)
        for candidate in mutations:
            with self.assertRaises(ActionError):
                parse_manifest(json.dumps(candidate))

    @given(manifest_graphs(), st.booleans())
    def test_generated_graph_queries_docs_only_for_sensitive_approved_workload(self, graph, protected):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = graph_auth(approved)
        by_label = {c["label"]: auth[c["id"]] for c in raw["capabilities"]}
        needs_docs = any(approved[c["id"]] and w["docs_sensitive"]
                         for c in raw["capabilities"] for w in c["workloads"])
        with mock.patch.object(docker_plan, "pull_request_is_docs_only") as unrelated:
            docs = mock.Mock(return_value=docs_only)
            authorize = mock.Mock(side_effect=by_label.__getitem__)
            plan = docker_plan.compute_plan(manifest, authorize, docs, protected_runners=protected)
        self.assertEqual(expected_plan(raw, auth, docs_only if needs_docs else False, protected), plan)
        self.assertEqual([mock.call(c["label"]) for c in raw["capabilities"]], authorize.call_args_list)
        self.assertEqual(int(needs_docs), docs.call_count)
        unrelated.assert_not_called()

    @given(manifest_graphs(), st.booleans(),
           st.lists(st.sampled_from(["success", "failure", "cancelled", "skipped"]), min_size=13, max_size=13))
    def test_generated_statuses_aggregate_shared_checks_and_publication(self, graph, protected, conclusions):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = graph_auth(approved)
        plan = expected_plan(raw, auth, docs_only, protected)
        jobs = [AUTHORIZATION_JOB, LOCAL_JOB, PUBLISH_JOB, *plan["workloads"]]
        results = dict(zip(jobs, conclusions))
        def verify(current):
            needs = {job: {"result": result, "outputs": {}} for job, result in current.items()}
            needs[AUTHORIZATION_JOB]["outputs"] = {"plan": serialize_plan(plan)}
            self.assertEqual(expected_statuses(raw, plan, current), evaluate_statuses(manifest, needs))
        verify(results)
        matching = {AUTHORIZATION_JOB: "success",
                    LOCAL_JOB: "success" if plan["local"]["enabled"] else "skipped",
                    PUBLISH_JOB: "success" if plan["publish"]["enabled"] else "skipped",
                    **{job: "success" if enabled else "skipped" for job, enabled in plan["workloads"].items()}}
        for job in jobs:
            for conclusion in ("success", "failure", "cancelled", "skipped"):
                verify({**matching, job: conclusion})

    @given(st.sampled_from([0, 1, 99, 100, 101, 199, 200, 201, 299]),
           st.sampled_from([".md", ".rs", ".MD"]),
           st.sampled_from([None, "old.md", "old.rs", "old.MD", 7]), st.integers(0, 298))
    def test_docs_only_checks_complete_rename_paths_and_pagination(self, count, suffix, previous, index):
        files = [{"filename": f"docs/file-{i}.md"} for i in range(count)]
        if files:
            entry = files[index % count]
            entry["filename"] = "docs/changed" + suffix
            if previous is not None:
                entry["previous_filename"] = previous
        routes = {PULL: {"changed_files": count}}
        for page in range(1, count // 100 + 2):
            routes[f"{FILES_PAGE}{page}"] = files[(page - 1) * 100:page * 100]
        client, transport = route_client(routes)
        expected = count > 0 and suffix == ".md" and (previous is None or previous == "old.md")
        self.assertEqual(expected, pull_request_is_docs_only(client, 42))
        pages = list(range(1, count // 100 + 2)) if count else []
        self.assertEqual([PULL, *[f"{FILES_PAGE}{page}" for page in pages]], transport.urls())
        if count:
            for claimed in (max(1, count - 1), count + 1):
                if claimed != count:
                    mismatched = copy.deepcopy(routes)
                    mismatched[PULL]["changed_files"] = claimed
                    client, _ = route_client(mismatched)
                    with self.assertRaises(ActionError):
                        pull_request_is_docs_only(client, 42)


if __name__ == "__main__":
    unittest.main()
