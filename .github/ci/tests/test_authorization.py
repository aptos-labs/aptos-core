import json
from datetime import datetime, timedelta
import unittest
from unittest import mock

from hypothesis import given, strategies as st

from tests.property_support import SAFE_COMPONENT, configure_profiles

configure_profiles()

from ci_actions import authorization
from ci_actions.authorization import DENIED, Authorization, authorize, main
from ci_actions.github import ActionError, HttpResponse
from tests.helpers import REPO_PATH, action_env, json_response, json_route, local_server, read_outputs, route_client, run_command, run_main

PULL = f"{REPO_PATH}/pulls/42"
TIMELINE = f"{REPO_PATH}/issues/42/timeline?per_page=100&page=1"
TIMELINE_2 = f"{REPO_PATH}/issues/42/timeline?per_page=100&page=2"
PERMISSION = f"{REPO_PATH}/collaborators/trusted-maintainer/permission"


def label(name):
    return {"id": 1, "name": name, "color": "1d76db", "default": False}


def event(event_id, *, login="trusted-maintainer", event="labeled", name="safe-to-test",
          created_at="2026-09-17T10:03:00Z"):
    return {
        "id": event_id, "event": event, "created_at": created_at, "label": label(name),
        "actor": None if login is None else {"login": login, "id": 1, "type": "User"},
    }


def routes(*, permission="write", latest=None, labels=("safe-to-test",)):
    return {
        PULL: {"number": 42, "state": "open", "labels": [label(name) for name in labels]},
        TIMELINE: [latest or event(303)],
        PERMISSION: {"permission": permission, "role_name": permission},
    }


def approved(login, event_id):
    return Authorization(True, login, event_id)


class AuthorizeTests(unittest.TestCase):
    def authorize(self, route_table):
        client, transport = route_client(route_table)
        return authorize(client, 42, "safe-to-test"), transport

    def test_approves_from_the_latest_matching_event_across_timeline_pages(self):
        table = routes()
        next_link = {"link": f'<https://api.github.test{TIMELINE_2}>; rel="next"'}
        table[TIMELINE] = json_response(
            [event(101, login="old-maintainer", created_at="2026-09-17T10:01:00Z"),
             event(102, name="unrelated-label", login="unrelated-user", created_at="2026-09-17T10:04:00Z")],
            headers=next_link,
        )
        table[TIMELINE_2] = [event(103)]
        result, transport = self.authorize(table)
        self.assertEqual(result, approved("trusted-maintainer", 103))
        self.assertEqual(len(transport.requests), 4)
        self.assertTrue(all(request["authorization"] == "Bearer test-token" for request in transport.requests))

    def test_denies_when_the_label_is_not_present(self):
        result, transport = self.authorize(routes(labels=("different-label",)))
        self.assertEqual(result, DENIED)
        self.assertEqual(len(transport.requests), 1)

    def test_denies_when_the_latest_matching_event_removed_the_label(self):
        table = routes()
        table[TIMELINE] = [event(201, created_at="2026-09-17T10:01:00Z"),
                           event(202, event="unlabeled", created_at="2026-09-17T10:02:00Z")]
        result, transport = self.authorize(table)
        self.assertEqual(result, DENIED)
        self.assertEqual(len(transport.requests), 2)

    def test_uses_event_id_to_order_equal_timestamps(self):
        table = routes()
        table[TIMELINE] = [event(602, created_at="2026-09-17T12:00:00Z"),
                           event(601, event="unlabeled", created_at="2026-09-17T12:00:00Z")]
        self.assertEqual(self.authorize(table)[0], approved("trusted-maintainer", 602))

    def test_equal_timestamp_order_is_independent_of_input_order(self):
        older = event(601, event="unlabeled", created_at="2026-09-17T12:00:00Z")
        newer = event(602, created_at="2026-09-17T12:00:00Z")
        for timeline in ([older, newer], [newer, older]):
            table = routes()
            table[TIMELINE] = timeline
            self.assertEqual(self.authorize(table)[0], approved("trusted-maintainer", 602))

    def test_compares_timestamps_as_instants(self):
        table = routes()
        table[TIMELINE] = [event(701, created_at="2026-09-17T12:00:00Z"),
                           event(700, event="unlabeled", created_at="2026-09-17T05:00:01-07:00")]
        self.assertEqual(self.authorize(table)[0], DENIED)

    def test_write_maintain_and_admin_collaborators_are_approved(self):
        # The REST permission field reports maintain as write.
        for permission in ("write", "admin"):
            with self.subTest(permission=permission):
                self.assertEqual(self.authorize(routes(permission=permission))[0], approved("trusted-maintainer", 303))

    def test_none_read_and_triage_collaborators_are_denied(self):
        # The REST permission field reports triage as read.
        for permission in ("none", "read"):
            with self.subTest(permission=permission):
                self.assertEqual(self.authorize(routes(permission=permission))[0], DENIED)

    def test_denies_a_bot_actor_without_querying_the_permission_endpoint(self):
        # "github-actions[bot]" is not a valid GitHub user login (letters, digits,
        # hyphens only), so it must not reach the collaborator-permission request.
        table = routes(latest=event(304, login="github-actions[bot]"))
        result, transport = self.authorize(table)
        self.assertEqual(result, DENIED)
        self.assertEqual(transport.urls(), [PULL, TIMELINE])

    def test_never_requests_a_server_supplied_pagination_url(self):
        table = routes()
        table[TIMELINE] = json_response([], headers={
            "link": '<https://attacker.example/x?after=opaque>; rel="next", <https://attacker.example/y>; rel="next"'})
        table[TIMELINE_2] = [event(501)]
        result, transport = self.authorize(table)
        self.assertEqual(result, approved("trusted-maintainer", 501))
        self.assertEqual(transport.urls(), [PULL, TIMELINE, TIMELINE_2, PERMISSION])

    def test_fails_closed_when_a_later_timeline_page_errors(self):
        table = routes()
        table[TIMELINE] = json_response([], headers={"link": f'<{TIMELINE_2}>; rel="next"'})
        table[TIMELINE_2] = HttpResponse(500, {}, b"")
        with self.assertRaisesRegex(ActionError, "HTTP 500"):
            self.authorize(table)

    def test_fails_closed_for_malformed_responses(self):
        cases = [
            ("malformed pull request", {PULL: {"labels": "safe-to-test"}}, "pull request response"),
            ("unnamed label", {PULL: {"labels": [{}]}}, "pull request response"),
            ("malformed timeline", {TIMELINE: {"events": []}}, "malformed list page"),
            ("event without type", {TIMELINE: [{"id": 1}]}, "event is missing"),
            ("lifecycle without label", {TIMELINE: [{"event": "labeled", "id": 1}]}, "label is missing"),
            ("missing matching event", {TIMELINE: [event(401, name="different-label")]},
             "matching label lifecycle event"),
            ("missing actor", {TIMELINE: [event(402, login=None)]}, "actor"),
            ("padded actor", {TIMELINE: [event(403, login=" trusted-maintainer")]}, "actor"),
            ("newline actor", {TIMELINE: [event(603, login="trusted-maintainer\napproved=true")]}, "actor"),
            ("invalid event ID", {TIMELINE: [event(0)]}, "event ID"),
            ("invalid timestamp", {TIMELINE: [event(404, created_at="yesterday")]}, "timestamp"),
            ("unknown permission", {PERMISSION: {"permission": "owner"}}, "permission"),
            ("unmapped maintain permission", {PERMISSION: {"permission": "maintain"}}, "permission"),
            ("unmapped triage permission", {PERMISSION: {"permission": "triage"}}, "permission"),
            ("missing permission", {PERMISSION: {}}, "permission"),
        ]
        for name, overrides, error in cases:
            with self.subTest(name=name), self.assertRaisesRegex(ActionError, error):
                self.authorize({**routes(), **overrides})

    def test_rejects_a_whitespace_label(self):
        client, transport = route_client(routes())
        with self.assertRaisesRegex(ActionError, "required_label"):
            authorize(client, 42, "   ")
        self.assertEqual(transport.requests, [])

    def test_batch_uses_one_pull_and_timeline_and_caches_shared_actor_permission(self):
        table = routes(labels=("safe-to-test", "run-e2e"))
        table[TIMELINE] = [event(303), event(304, name="run-e2e")]
        client, transport = route_client(table)
        results = authorization.authorize_labels(client, 42, ["safe-to-test", "run-e2e", "absent"])
        self.assertEqual(results, {
            "safe-to-test": approved("trusted-maintainer", 303),
            "run-e2e": approved("trusted-maintainer", 304),
            "absent": DENIED,
        })
        self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])

    def test_batch_keeps_each_label_lifecycle_independent(self):
        table = routes(labels=("safe-to-test", "run-e2e"))
        table[TIMELINE] = [event(303), event(304, name="run-e2e", event="unlabeled")]
        client, transport = route_client(table)
        self.assertEqual(
            authorization.authorize_labels(client, 42, ["safe-to-test", "run-e2e"]),
            {"safe-to-test": approved("trusted-maintainer", 303), "run-e2e": DENIED},
        )
        self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])

    def test_batch_fails_closed_for_duplicate_labels_and_missing_lifecycle(self):
        client, transport = route_client(routes())
        with self.assertRaisesRegex(ActionError, "duplicate"):
            authorization.authorize_labels(client, 42, ["safe-to-test", "safe-to-test"])
        self.assertEqual(transport.urls(), [])
        table = routes(labels=("safe-to-test", "run-e2e"))
        client, _ = route_client(table)
        with self.assertRaisesRegex(ActionError, "run-e2e"):
            authorization.authorize_labels(client, 42, ["safe-to-test", "run-e2e"])


class ComputeAuthorizedCommandTests(unittest.TestCase):
    ENV = {"GITHUB_REPOSITORY": "aptos-labs/aptos-core", "GH_TOKEN": "test-token",
           "INPUT_PR_NUMBER": "42", "INPUT_REQUIRED_LABEL": "safe-to-test"}

    def run_action(self, server_routes, **env):
        with local_server(server_routes) as (base, requests):
            code, stderr, outputs, _ = run_main("compute-authorized", main,
                                                {**self.ENV, "GITHUB_API_URL": base, **env})
        return code, stderr, outputs, requests

    def server_routes(self, **overrides):
        table = {path: json_route(value) for path, value in routes().items()}
        table.update(overrides)
        return table

    def test_writes_approval_outputs(self):
        code, stderr, outputs, requests = self.run_action(self.server_routes())
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs, {"approved": "true", "approver": "trusted-maintainer", "approval_event_id": "303"})
        self.assertTrue(all(request["authorization"] == "Bearer test-token" for request in requests))

    def test_writes_denied_outputs_without_failing(self):
        table = self.server_routes()
        table[PULL] = json_route({"labels": []})
        code, stderr, outputs, _ = self.run_action(table)
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs, {"approved": "false", "approver": "", "approval_event_id": ""})

    def test_invalid_inputs_fail_closed_before_any_request(self):
        for env in (
            {"INPUT_PR_NUMBER": "0"}, {"INPUT_PR_NUMBER": "1.5"}, {"INPUT_PR_NUMBER": "9007199254740992"},
            {"INPUT_REQUIRED_LABEL": ""}, {"INPUT_REQUIRED_LABEL": "   "},
        ):
            with self.subTest(env=env):
                code, stderr, outputs, _ = run_command("compute-authorized", {
                    **self.ENV, "GITHUB_API_URL": "https://api.github.test", **env})
                self.assertEqual(code, 1)
                self.assertEqual(outputs["approved"], "false")
                self.assertRegex(stderr, "failed closed: .*(pr_number|INPUT_REQUIRED_LABEL|required_label)")

    def test_production_launcher_rejects_a_loopback_http_api(self):
        with local_server({}) as (base, requests):
            code, stderr, outputs, _ = run_command("compute-authorized", {**self.ENV, "GITHUB_API_URL": base})
        self.assertEqual(code, 1)
        self.assertEqual(outputs, {"approved": "false", "approver": "", "approval_event_id": ""})
        self.assertIn("GITHUB_API_URL must use HTTPS", stderr)
        self.assertEqual(requests, [])

    def test_api_errors_fail_closed_with_denied_outputs(self):
        table = self.server_routes()
        table[PERMISSION] = json_route({"message": "error"}, status=500)
        code, stderr, outputs, _ = self.run_action(table)
        self.assertEqual(code, 1)
        self.assertEqual(outputs, {"approved": "false", "approver": "", "approval_event_id": ""})
        self.assertIn("GitHub API request failed with HTTP 500", stderr)

    def test_ghes_api_base_with_a_path_prefix(self):
        table = {f"/api/v3{path}": route for path, route in self.server_routes().items()}
        with local_server(table) as (base, _):
            code, stderr, outputs, _ = run_main("compute-authorized", main,
                                                {**self.ENV, "GITHUB_API_URL": f"{base}/api/v3"})
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs["approved"], "true")

    def test_batch_mode_writes_json_approvals_and_aggregate_approval(self):
        table = routes(labels=("safe-to-test", "run-e2e"))
        table[TIMELINE] = [event(303), event(304, name="run-e2e", event="unlabeled")]
        client, transport = route_client(table)
        with action_env(INPUT_REQUIRED_LABEL="", INPUT_REQUIRED_LABELS='["safe-to-test","run-e2e"]',
                        INPUT_PR_NUMBER="42") as output, \
                mock.patch.object(authorization.GitHubClient, "from_env", return_value=client):
            main()
            outputs = read_outputs(output)
        self.assertEqual(outputs["approved"], "true")
        self.assertEqual(json.loads(outputs["approvals"]), {
            "safe-to-test": {"approved": "true", "approver": "trusted-maintainer", "approval_event_id": "303"},
            "run-e2e": {"approved": "false", "approver": "", "approval_event_id": ""},
        })
        self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])

    def test_batch_mode_rejects_conflicting_inputs(self):
        client, transport = route_client(routes())
        with action_env(INPUT_PR_NUMBER="42", INPUT_REQUIRED_LABEL="safe-to-test",
                        INPUT_REQUIRED_LABELS='["safe-to-test"]') as output, \
                mock.patch.object(authorization.GitHubClient, "from_env", return_value=client):
            with self.assertRaises(ActionError):
                main()
            self.assertEqual(read_outputs(output)["approved"], "false")
        self.assertEqual(transport.requests, [])

    def test_batch_mode_rejects_malformed_arrays(self):
        for labels in ('[]', '["safe-to-test","safe-to-test"]', '{"x":1}'):
            with self.subTest(labels=labels):
                client, transport = route_client(routes())
                with action_env(INPUT_PR_NUMBER="42", INPUT_REQUIRED_LABEL="", INPUT_REQUIRED_LABELS=labels) as output, \
                        mock.patch.object(authorization.GitHubClient, "from_env", return_value=client):
                    with self.assertRaises(ActionError):
                        main()
                    self.assertEqual(read_outputs(output)["approved"], "false")
                self.assertEqual(transport.requests, [])


class AuthorizationProperties(unittest.TestCase):
    @given(data=st.data(), components=st.lists(SAFE_COMPONENT, min_size=1, max_size=4, unique=True),
           supplied_pull=st.booleans())
    def test_latest_lifecycle_batch_model(self, data, components, supplied_pull):
        labels = ["label-" + component for component in components]
        present = data.draw(st.sets(st.sampled_from(labels), max_size=len(labels)), label="present labels")
        actors = ("User-A", "User-B", "github-actions[bot]", "with_underscore")
        permissions = {actor: data.draw(st.sampled_from(("none", "read", "write", "admin")), label=actor + " permission")
                       for actor in actors[:2]}
        # Unique IDs avoid conflicting equal ordering keys; equal timestamps remain allowed.
        records = data.draw(st.lists(st.tuples(st.sampled_from(labels + ["noise-label"]),
                                               st.sampled_from(("labeled", "unlabeled")), st.sampled_from(actors),
                                               st.integers(0, 5), st.sampled_from((-420, 0, 330))), max_size=16), label="events")
        for name in labels:
            if name in present and not any(record[0] == name for record in records):
                records.append((name, "labeled", "User-A", 0, 0))
        timeline = []
        expected = {name: DENIED for name in labels}
        selected = {}
        for event_id, (name, lifecycle, actor, seconds, offset) in enumerate(records, 1):
            utc = datetime(2026, 9, 17, 12) + timedelta(seconds=seconds)
            local = utc + timedelta(minutes=offset)
            zone = "Z" if offset == 0 else f"{'+' if offset >= 0 else '-'}{abs(offset)//60:02}:{abs(offset)%60:02}"
            timeline.append(event(event_id, name=name, event=lifecycle, login=actor,
                                  created_at=local.strftime("%Y-%m-%dT%H:%M:%S") + zone))
            if name in present:
                candidate = (seconds, event_id, lifecycle, actor)
                if name not in selected or candidate[:2] > selected[name][:2]:
                    selected[name] = candidate
        expected_actors = []
        for name in labels:
            if name not in selected:
                continue
            _, event_id, lifecycle, actor = selected[name]
            if lifecycle == "labeled" and actor in actors[:2]:
                if actor not in expected_actors:
                    expected_actors.append(actor)
                if permissions[actor] in ("write", "admin"):
                    expected[name] = approved(actor, event_id)
        order = data.draw(st.permutations(tuple(range(len(timeline)))), label="timeline permutation")
        shuffled = [timeline[index] for index in order]
        page_count = data.draw(st.integers(1, 4), label="timeline pages")
        pull = {"labels": [label(name) for name in present]}
        for sequence in (timeline, shuffled):
            table = {PULL: pull}
            for index in range(page_count):
                start, end = index * len(sequence) // page_count, (index + 1) * len(sequence) // page_count
                headers = {"link": '<https://attacker.test/endpoint>; rel="next"'} if index < page_count - 1 else {}
                table[f"{REPO_PATH}/issues/42/timeline?per_page=100&page={index + 1}"] = json_response(sequence[start:end], headers=headers)
            for actor, permission in permissions.items():
                table[f"{REPO_PATH}/collaborators/{actor}/permission"] = {"permission": permission, "role_name": "ignored"}
            client, transport = route_client(table)
            result = authorization.authorize_labels(client, 42, labels, pull_request=pull if supplied_pull else None)
            self.assertEqual(result, expected)
            expected_urls = [] if supplied_pull else [PULL]
            if present:
                expected_urls += [f"{REPO_PATH}/issues/42/timeline?per_page=100&page={index + 1}" for index in range(page_count)]
                expected_urls += [f"{REPO_PATH}/collaborators/{actor}/permission" for actor in expected_actors]
            self.assertEqual(transport.urls(), expected_urls)
            serialized = authorization._batch_outputs(result)
            self.assertEqual(serialized["approved"], "true" if any(value.approved for value in expected.values()) else "false")
            self.assertEqual(json.loads(serialized["approvals"]), {
                name: {"approved": "true" if value.approved else "false", "approver": value.approver or "",
                       "approval_event_id": "" if value.approval_event_id is None else str(value.approval_event_id)}
                for name, value in expected.items()})

    @given(component=SAFE_COMPONENT)
    def test_permission_categories(self, component):
        for permission in ("none", "read", "write", "admin"):
            table = routes(permission=permission)
            table[PERMISSION]["role_name"] = component
            client, transport = route_client(table)
            self.assertEqual(authorize(client, 42, "safe-to-test"), approved("trusted-maintainer", 303)
                             if permission in ("write", "admin") else DENIED)
            self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])
        for bad in (None, [], {}, {"permission": None}, {"permission": True}, {"permission": "owner"},
                    {"permission": "maintain"}, {"permission": "triage"}):
            table = routes()
            table[PERMISSION] = json_response(bad)
            client, transport = route_client(table)
            with self.assertRaises(ActionError):
                authorize(client, 42, "safe-to-test")
            self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])

    @given(component=SAFE_COMPONENT)
    def test_required_label_categories_before_requests(self, component):
        valid = " " + component + " "
        self.assertEqual(authorization.parse_required_label(valid), valid)
        for bad in (None, (), [], [None], [1], [""], [" \t"], [component, component]):
            client, transport = route_client({})
            with self.assertRaises(ActionError):
                authorization.authorize_labels(client, 42, bad)
            self.assertEqual(transport.requests, [])

    @given(event_id=st.integers(1, 2**53 - 1))
    def test_response_rejection_categories(self, event_id):
        cases = [
            (PULL, None, [PULL]), (PULL, {"labels": "safe-to-test"}, [PULL]),
            (PULL, {"labels": [{}]}, [PULL]), (TIMELINE, {"events": []}, [PULL, TIMELINE]),
            (TIMELINE, [{"id": event_id}], [PULL, TIMELINE]),
            (TIMELINE, [{"event": "labeled", "id": event_id}], [PULL, TIMELINE]),
            (TIMELINE, [event(event_id, name="different-label")], [PULL, TIMELINE]),
        ]
        for bad_id in (True, None, 0, -1, 2**53, "1", 1.0):
            cases.append((TIMELINE, [event(bad_id)], [PULL, TIMELINE]))
        for timestamp in (None, "yesterday", "2026-02-30T12:00:00Z"):
            cases.append((TIMELINE, [event(event_id, created_at=timestamp)], [PULL, TIMELINE]))
        for actor in (None, "", " trusted-maintainer", "trusted-maintainer\napproved=true", "trusted-maintainer\r"):
            cases.append((TIMELINE, [event(event_id, login=actor)], [PULL, TIMELINE]))
        for path, body, expected_urls in cases:
            table = routes()
            table[path] = json_response(body)
            client, transport = route_client(table)
            with self.assertRaises(ActionError):
                authorize(client, 42, "safe-to-test")
            self.assertEqual(transport.urls(), expected_urls)

    @given(component=SAFE_COMPONENT)
    def test_unrelated_record_validation_precedence(self, component):
        ignored = [{"event": "commented", "id": None, "actor": None},
                   {"event": "labeled", "label": {"name": "unrelated-" + component}, "id": None, "actor": None}]
        table = routes()
        table[TIMELINE] = ignored + [event(303)]
        client, transport = route_client(table)
        try:
            result = authorize(client, 42, "safe-to-test")
        except ActionError as error:
            self.fail(f"Unrelated records must be filtered before ID/actor validation; requests {transport.urls()}: {error}")
        self.assertEqual(result, approved("trusted-maintainer", 303))
        self.assertEqual(transport.urls(), [PULL, TIMELINE, PERMISSION])
        # Lifecycle label validation happens before filtering; event schema validation is unconditional.
        for bad in ({"event": "labeled"}, {"event": None}, None):
            table[TIMELINE] = [bad, event(303)]
            client, transport = route_client(table)
            with self.assertRaises(ActionError):
                authorize(client, 42, "safe-to-test")
            self.assertEqual(transport.urls(), [PULL, TIMELINE])
        table[PULL] = {"labels": []}
        client, transport = route_client(table)
        self.assertEqual(authorize(client, 42, "safe-to-test"), DENIED)
        self.assertEqual(transport.urls(), [PULL])


if __name__ == "__main__":
    unittest.main()
