"""Tests for ci_actions.docker_plan (docker-capability-plan and docker-status-plan)."""

from __future__ import annotations

import copy
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
    def test_repository_manifest_is_structurally_valid(self):
        self.assertEqual("rust-images", MANIFEST.checks()[0])
        self.assertEqual(len(MANIFEST.checks()), len(set(MANIFEST.checks())))

    def test_repository_workloads_declare_their_image_requirements(self):
        self.assertEqual({
            "pr-node-cli-faucet-tests": ("release", True),
            "pr-forge-e2e": ("release", False),
            "pr-forge-compat": ("release", False),
            "pr-forge-performance": ("performance", False),
            "pr-forge-framework-upgrade": ("release", False),
            "pr-forge-consensus-only": ("consensus", False),
            "pr-forge-multiregion": ("release", False),
        }, {
            workload.id: (workload.image_variant, workload.additional_testing_images)
            for workload in MANIFEST.workloads()
        })

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
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "unknown field"):
                parse_manifest(mutated(mutator))

    def test_removed_job_fields_are_rejected(self):
        with self.assertRaisesRegex(ActionError, "unknown field 'local_job'"):
            parse_manifest(mutated(lambda v: v["variants"][0].update(local_job="pr-rust-images-local")))

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
        ]
        for mutator in cases:
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "duplicate|unknown|list"):
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

    def test_publication_requires_the_same_local_variant(self):
        with self.assertRaisesRegex(ActionError, "publication requires the same local variant"):
            parse_manifest(mutated(lambda v: capability(v, "build_failpoints").update(
                local=["release"], publish=["failpoints"])))

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

    def test_dynamic_expression_duplicate_key_and_oversized_input_are_rejected(self):
        with self.assertRaisesRegex(ActionError, "dynamic expression"):
            parse_manifest(mutated(lambda v: v["capabilities"][0].update(label="${{ github.event.pull_request.title }}")))
        with self.assertRaisesRegex(ActionError, "duplicate key"):
            parse_manifest('{"version": 1, "version": 1}')
        with self.assertRaisesRegex(ActionError, "size limit"):
            parse_manifest(" " * (MAX_MANIFEST_BYTES + 1))


class PlanTests(unittest.TestCase):
    def test_every_approval_subset_selects_exact_variants_and_workloads(self):
        ids = [capability.id for capability in MANIFEST.capabilities]
        order = [variant["id"] for variant in MANIFEST.variants]
        for mask in range(1 << len(ids)):
            approved = {ids[i] for i in range(len(ids)) if mask & (1 << i)}
            for docs_only in (False, True):
                plan = build_plan(MANIFEST, approvals(approved), docs_only)
                local = {v for c in MANIFEST.capabilities if c.id in approved for v in c.local}
                publish = {v for c in MANIFEST.capabilities if c.id in approved for v in c.publish}
                publish.update(
                    w.image_variant for c in MANIFEST.capabilities if c.id in approved
                    for w in c.workloads if not (w.docs_sensitive and docs_only)
                )
                self.assertEqual([i for i in order if i in local], [v["id"] for v in plan["local"]["include"]])
                self.assertEqual([i for i in order if i in publish], [v["id"] for v in plan["publish"]["include"]])
                for variant in plan["publish"]["include"]:
                    self.assertIs(
                        any(
                            w.image_variant == variant["id"] and w.additional_testing_images
                            for c in MANIFEST.capabilities if c.id in approved
                            for w in c.workloads if not (w.docs_sensitive and docs_only)
                        ),
                        variant["additional_testing_images"],
                    )
                self.assertEqual(bool(local), plan["local"]["enabled"])
                self.assertEqual(bool(publish), plan["publish"]["enabled"])
                expected_markers = []
                for c in MANIFEST.capabilities:
                    for w in c.workloads:
                        active = c.id in approved and not (w.docs_sensitive and docs_only)
                        self.assertIs(active, plan["workloads"][w.id], f"{mask}:{docs_only}:{w.id}")
                        if active and w.marker:
                            expected_markers.append({"marker": w.marker.id, "workload": w.id})
                self.assertEqual(expected_markers, plan["markers"]["include"])
                self.assertEqual(bool(expected_markers), plan["markers"]["enabled"])
                self.assertEqual(plan, parse_plan(MANIFEST, serialize_plan(plan)))

    def test_plan_shape_uses_manifest_ids_directly(self):
        plan = build_plan(MANIFEST, approvals({c.id for c in MANIFEST.capabilities}), False)
        self.assertEqual(["docs_only", "approvals", "local", "publish", "workloads", "markers"], list(plan))
        self.assertEqual([w.id for w in MANIFEST.workloads()], list(plan["workloads"]))
        self.assertEqual([m.id for m in MANIFEST.markers()], [m["marker"] for m in plan["markers"]["include"]])
        self.assertEqual({"id", "profile", "features", "build_target"}, set(plan["local"]["include"][0]))
        self.assertNotIn("\n", serialize_plan(plan))

    def test_workload_publication_deduplicates_variants_and_combines_testing_images(self):
        cases = [
            ({"run_e2e"}, False, [("release", True)]),
            ({"run_framework_upgrade"}, False, [("release", False)]),
            ({"run_forge_performance"}, False, [("performance", False)]),
            ({"run_consensus"}, False, [("consensus", False)]),
            ({"run_e2e", "run_framework_upgrade", "build_images"}, False, [("release", True)]),
            ({"run_e2e", "build_failpoints"}, False, [("release", True), ("failpoints", False)]),
            ({"run_e2e"}, True, []),
            ({"run_e2e", "build_images"}, True, [("release", False)]),
        ]
        for approved, docs_only, expected in cases:
            with self.subTest(approved=approved, docs_only=docs_only):
                plan = build_plan(MANIFEST, approvals(approved), docs_only)
                self.assertEqual(expected, [
                    (variant["id"], variant["additional_testing_images"])
                    for variant in plan["publish"]["include"]
                ])
                self.assertEqual(bool(expected), plan["publish"]["enabled"])
                if approved == {"run_e2e"} and docs_only:
                    self.assertEqual(["release"], [variant["id"] for variant in plan["local"]["include"]])

    def test_manifest_relationship_mutations_change_activation(self):
        manifest = parse_manifest(mutated(lambda v: capability(v, "build_failpoints").update(publish=[])))
        plan = build_plan(manifest, {c.id: granted(0) if c.id == "build_failpoints" else DENIED
                                     for c in manifest.capabilities}, False)
        self.assertEqual(["release", "failpoints"], [v["id"] for v in plan["local"]["include"]])
        self.assertFalse(plan["publish"]["enabled"])

        def move(v):
            capability(v, "run_multiregion")["workloads"].append(capability(v, "run_framework_upgrade")["workloads"].pop())
        manifest = parse_manifest(mutated(move))
        def only(cid):
            return {c.id: granted(0) if c.id == cid else DENIED for c in manifest.capabilities}

        self.assertFalse(build_plan(manifest, only("run_framework_upgrade"), False)["workloads"]["pr-forge-framework-upgrade"])
        self.assertTrue(build_plan(manifest, only("run_multiregion"), False)["workloads"]["pr-forge-framework-upgrade"])

    def test_build_plan_requires_exact_valid_approvals(self):
        missing = approvals()
        del missing["run_e2e"]
        with self.assertRaisesRegex(ActionError, "approval IDs"):
            build_plan(MANIFEST, missing, False)
        with self.assertRaisesRegex(ActionError, "approval IDs"):
            build_plan(MANIFEST, {**approvals(), "surprise": DENIED}, False)
        for bad in [
            Authorization(approved="true", approver="maintainer", approval_event_id=1),
            Authorization(approved=True, approver="", approval_event_id=1),
            Authorization(approved=True, approver="maintainer\nunsafe", approval_event_id=1),
            Authorization(approved=True, approver="maintainer", approval_event_id=0),
            Authorization(approved=True, approver="maintainer", approval_event_id=True),
            Authorization(approved=True, approver="maintainer", approval_event_id=2**53),
            Authorization(approved=False, approver="maintainer", approval_event_id=None),
            Authorization(approved=False, approver=None, approval_event_id=1),
        ]:
            with self.subTest(bad=bad), self.assertRaisesRegex(ActionError, "approval"):
                build_plan(MANIFEST, {**approvals(), "build_images": bad}, False)
        with self.assertRaisesRegex(ActionError, "docs_only"):
            build_plan(MANIFEST, approvals(), "false")

    def test_tampered_plans_are_rejected(self):
        text = serialize_plan(build_plan(MANIFEST, approvals({"build_failpoints", "run_e2e"}), False))
        plan = json.loads(text)
        tampered = [
            lambda p: p["local"].update(include=[v for v in p["local"]["include"] if v["id"] != "failpoints"]),
            lambda p: p["workloads"].update({"pr-forge-e2e": False}),
            lambda p: p.update(docs_only="yes"),
            lambda p: p["approvals"].pop("run_e2e"),
            lambda p: p.update(extra=1),
        ]
        for mutator in tampered:
            value = copy.deepcopy(plan)
            mutator(value)
            with self.subTest(mutator=mutator), self.assertRaisesRegex(ActionError, "validated Docker capability plan"):
                parse_plan(MANIFEST, json.dumps(value, separators=(",", ":")))
        for bad in ["", None, text.replace("}", "} ", 1), text + "${{"]:
            with self.subTest(bad=bad), self.assertRaisesRegex(ActionError, "validated Docker capability plan"):
                parse_plan(MANIFEST, bad)


class DocsOnlyTests(unittest.TestCase):
    def docs_only(self, routes):
        client, transport = route_client(routes)
        return pull_request_is_docs_only(client, 42), transport

    def test_docs_only_requires_at_least_one_file_and_only_markdown(self):
        self.assertTrue(self.docs_only(docs_routes(["README.md", "docs/a.md"]))[0])
        self.assertFalse(self.docs_only(docs_routes(["README.md", "src/lib.rs"]))[0])
        self.assertFalse(self.docs_only(docs_routes(["README.MD"]))[0])
        result, transport = self.docs_only(docs_routes([]))
        self.assertFalse(result)
        self.assertEqual([PULL], transport.urls())

    def test_docs_only_requires_renames_to_come_from_markdown(self):
        def renamed(previous):
            return {PULL: {"changed_files": 1}, f"{FILES_PAGE}1": [{"filename": "x.md", "previous_filename": previous}]}

        self.assertTrue(self.docs_only(renamed("docs/old.md"))[0])
        self.assertFalse(self.docs_only(renamed("src/x.rs"))[0])
        self.assertFalse(self.docs_only(renamed(None))[0])

    def test_docs_only_reads_every_page(self):
        result, transport = self.docs_only(docs_routes([f"docs/{i}.md" for i in range(250)] + ["src/main.rs"]))
        self.assertFalse(result)
        self.assertEqual([f"{FILES_PAGE}{page}" for page in (1, 2, 3)], transport.urls()[1:])

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


class PlanMainTests(unittest.TestCase):
    def test_plan_main_authorizes_each_manifest_label_and_writes_plan(self):
        labels = ("CICD:build-failpoints-images", "CICD:run-e2e-tests")
        routes = docs_routes(["README.md"])
        routes[PULL]["labels"] = [{"name": label} for label in labels]
        routes[f"{REPO_PATH}/issues/42/timeline?per_page=100&page=1"] = [
            {"id": index, "event": "labeled", "label": {"name": label},
             "actor": {"login": "trusted-maintainer"}, "created_at": "2026-09-17T10:03:00Z"}
            for index, label in enumerate(labels, 1)
        ]
        permission = f"{REPO_PATH}/collaborators/trusted-maintainer/permission"
        routes[permission] = {"permission": "write"}
        transport = RouteTransport(routes)
        with action_env(INPUT_PR_NUMBER="42") as output:
            docker_plan.plan_main(transport=transport)
            outputs = read_outputs(output)
        self.assertEqual(1, transport.urls().count(PULL))
        self.assertEqual(1, transport.urls().count(permission))
        self.assertEqual(4, len(transport.urls()))
        self.assertEqual(["plan"], list(outputs))
        plan = parse_plan(MANIFEST, outputs["plan"])
        self.assertTrue(plan["docs_only"])
        self.assertEqual(["release", "failpoints"], [v["id"] for v in plan["local"]["include"]])
        self.assertEqual(["failpoints"], [v["id"] for v in plan["publish"]["include"]])
        self.assertFalse(plan["workloads"]["pr-forge-e2e"])
        self.assertFalse(plan["markers"]["enabled"])

    def test_plan_main_skips_the_file_listing_without_a_docs_sensitive_approval(self):
        transport = RouteTransport({PULL: {"labels": [], "changed_files": 1}})
        with action_env(INPUT_PR_NUMBER="42") as output:
            docker_plan.plan_main(transport=transport)
            self.assertFalse(json.loads(read_outputs(output)["plan"])["docs_only"])
        self.assertEqual([PULL], transport.urls())

    def test_plan_main_fails_closed_before_api_access_for_invalid_input(self):
        transport = RouteTransport({})
        with action_env(INPUT_PR_NUMBER="0") as output:
            with self.assertRaises(ActionError):
                docker_plan.plan_main(transport=transport)
            self.assertEqual("", output.read_text())
        self.assertEqual([], transport.requests)


def needs_for(approved=(), docs_only=False, **overrides) -> dict:
    plan = build_plan(MANIFEST, approvals(approved), docs_only)
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
    def test_no_approvals_passes_when_everything_is_skipped(self):
        self.assertEqual(all_checks(True), evaluate_statuses(MANIFEST, needs_for()))

    def test_image_matrix_failure_fails_only_the_image_check(self):
        needs = needs_for({"build_images"}, pr_rust_images_local="failure")
        self.assertEqual(all_checks(True, rust_images=False), evaluate_statuses(MANIFEST, needs))
        needs = needs_for({"build_images"}, pr_publish_rust_images="skipped")
        self.assertEqual(all_checks(True, rust_images=False), evaluate_statuses(MANIFEST, needs))

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

    def test_workload_failure_fails_every_dependent_check(self):
        needs = needs_for({"run_e2e", "run_forge_performance"},
                          pr_node_cli_faucet_tests="failure", pr_forge_performance="cancelled")
        self.assertEqual(
            all_checks(True, node_api_compatibility_tests=False, cli_e2e_tests=False,
                       faucet_tests_main=False, forge_e2e_test=False),
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
        plan["local"]["include"] = [v for v in plan["local"]["include"] if v["id"] != "failpoints"]
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

    def test_every_subset_of_approvals_passes_when_jobs_match_the_plan(self):
        ids = [c.id for c in MANIFEST.capabilities]
        for count in range(len(ids) + 1):
            for approved in itertools.combinations(ids, count):
                self.assertEqual(all_checks(True), evaluate_statuses(MANIFEST, needs_for(set(approved))))


class PlanningPropertyTests(unittest.TestCase):
    @given(manifest_graphs())
    def test_generated_graph_has_exact_plan_and_canonical_serialization(self, graph):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = {cid: granted(i) if flag else DENIED for i, (cid, flag) in enumerate(approved.items())}
        expected = expected_plan(raw, auth, docs_only)
        actual = build_plan(manifest, auth, docs_only)
        self.assertEqual(expected, actual)
        canonical = json.dumps(expected, separators=(",", ":"), ensure_ascii=True)
        self.assertEqual(canonical, serialize_plan(actual))
        self.assertEqual(expected, parse_plan(manifest, canonical))
        # Whitespace and key order are part of the handoff contract.
        for altered in (json.dumps(expected, indent=2), json.dumps(expected, sort_keys=True)):
            if altered != canonical:
                with self.assertRaises(ActionError):
                    parse_plan(manifest, altered)

    @given(manifest_graphs())
    def test_generated_manifest_and_approval_contracts_fail_closed(self, graph):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = {cid: granted(i) if flag else DENIED for i, (cid, flag) in enumerate(approved.items())}
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
        cid = next(iter(auth))
        for invalid in (Authorization("true", "maintainer", 1), Authorization(True, "", 1),
                        Authorization(True, "bad login", 1), Authorization(True, "maintainer", True),
                        Authorization(True, "maintainer", 0), Authorization(True, "maintainer", 2**53),
                        Authorization(False, "maintainer", None), Authorization(False, None, 1)):
            with self.assertRaises(ActionError):
                build_plan(manifest, {**auth, cid: invalid}, docs_only)
        for invalid in ({k: v for k, v in auth.items() if k != cid}, {**auth, "unknown": DENIED}):
            with self.assertRaises(ActionError):
                build_plan(manifest, invalid, docs_only)
        for invalid in ("true", 1, None):
            with self.assertRaises(ActionError):
                build_plan(manifest, auth, invalid)

    @given(manifest_graphs())
    def test_generated_graph_queries_docs_only_for_sensitive_approved_workload(self, graph):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = {cid: granted(i) if flag else DENIED for i, (cid, flag) in enumerate(approved.items())}
        by_label = {c["label"]: auth[c["id"]] for c in raw["capabilities"]}
        needs_docs = any(approved[c["id"]] and w["docs_sensitive"]
                         for c in raw["capabilities"] for w in c["workloads"])
        with mock.patch.object(docker_plan, "pull_request_is_docs_only") as unrelated:
            docs = mock.Mock(return_value=docs_only)
            authorize = mock.Mock(side_effect=by_label.__getitem__)
            plan = docker_plan.compute_plan(manifest, authorize, docs)
        self.assertEqual(expected_plan(raw, auth, docs_only if needs_docs else False), plan)
        self.assertEqual([mock.call(c["label"]) for c in raw["capabilities"]], authorize.call_args_list)
        self.assertEqual(int(needs_docs), docs.call_count)
        unrelated.assert_not_called()

    @given(manifest_graphs(), st.lists(st.sampled_from(["success", "failure", "cancelled", "skipped"]),
                                      min_size=13, max_size=13))
    def test_generated_statuses_aggregate_shared_checks_and_publication(self, graph, conclusions):
        raw, approved, docs_only = graph
        manifest = parse_manifest(json.dumps(raw))
        auth = {cid: granted(i) if flag else DENIED for i, (cid, flag) in enumerate(approved.items())}
        plan = expected_plan(raw, auth, docs_only)
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
