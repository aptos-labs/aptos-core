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
    def test_fails_closed_when_a_later_timeline_page_errors(self):
        table = routes()
        table[TIMELINE] = json_response([], headers={"link": f'<{TIMELINE_2}>; rel="next"'})
        table[TIMELINE_2] = HttpResponse(500, {}, b"")
        client, _ = route_client(table)
        with self.assertRaisesRegex(ActionError, "HTTP 500"):
            authorize(client, 42, "safe-to-test")

    def test_permission_categories(self):
        # The REST permission field reports maintain as write and triage as read.
        cases = [({"permission": name}, approved("trusted-maintainer", 303)) for name in ("write", "admin")]
        cases += [({"permission": name}, DENIED) for name in ("none", "read")]
        cases += [(body, None) for body in (None, [], {}, {"permission": None}, {"permission": True},
                                            {"permission": "owner"}, {"permission": "maintain"},
                                            {"permission": "triage"})]
        for body, expected in cases:
            table = routes()
            table[PERMISSION] = json_response(body)
            client, transport = route_client(table)
            with self.subTest(body=body):
                if expected is None:
                    with self.assertRaisesRegex(ActionError, "permission"):
                        authorize(client, 42, "safe-to-test")
                else:
                    self.assertEqual(expected, authorize(client, 42, "safe-to-test"))
                self.assertEqual([PULL, TIMELINE, PERMISSION], transport.urls())

    def test_response_rejection_categories(self):
        cases = [
            (PULL, None, "pull request response"),
            (PULL, {"labels": "safe-to-test"}, "pull request response"),
            (PULL, {"labels": [{}]}, "pull request response"),
            (TIMELINE, {"events": []}, "malformed list page"),
            (TIMELINE, [{"id": 404}], "event is missing"),
            (TIMELINE, [{"event": "labeled", "id": 404}], "label is missing"),
            (TIMELINE, [event(404, name="different-label")], "matching label lifecycle event"),
        ]
        cases += [(TIMELINE, [event(bad_id)], "event ID") for bad_id in (True, None, 0, -1, 2**53, "1", 1.0)]
        cases += [(TIMELINE, [event(404, created_at=timestamp)], "timestamp")
                  for timestamp in (None, "yesterday", "2026-02-30T12:00:00Z")]
        cases += [(TIMELINE, [event(404, login=actor)], "actor") for actor in (
            None, "", " trusted-maintainer", "trusted-maintainer\napproved=true", "trusted-maintainer\r")]
        for path, body, error in cases:
            table = routes()
            table[path] = json_response(body)
            client, transport = route_client(table)
            with self.subTest(path=path, body=body):
                with self.assertRaisesRegex(ActionError, error):
                    authorize(client, 42, "safe-to-test")
                self.assertEqual([PULL] if path == PULL else [PULL, TIMELINE], transport.urls())


class ComputeAuthorizedCommandTests(unittest.TestCase):
    ENV = {"GITHUB_REPOSITORY": "aptos-labs/aptos-core", "GH_TOKEN": "test-token",
           "INPUT_PR_NUMBER": "42", "INPUT_REQUIRED_LABEL": "safe-to-test", "INPUT_REQUIRED_LABELS": ""}

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
        code, stderr, outputs, _ = self.run_action(self.server_routes())
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs, {"approved": "true", "approver": "trusted-maintainer", "approval_event_id": "303"})

    def test_writes_denied_outputs_without_failing(self):
        table = self.server_routes()
        table[PULL] = json_route({"labels": []})
        code, stderr, outputs, _ = self.run_action(table)
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs, {"approved": "false", "approver": "", "approval_event_id": ""})

    def test_invalid_inputs_fail_closed_before_any_request(self):
        for env in ({"INPUT_PR_NUMBER": "0"}, {"INPUT_REQUIRED_LABEL": "   "}):
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
        cases = [("[]", "non-empty"), ('["safe-to-test","safe-to-test"]', "duplicate"), ('{"x":1}', "array")]
        cases += [(labels, "must be a JSON array")
                  for labels in ("[NaN]", '{"a": 1, "a": 2}', '[{"a": 1, "a": 2}]', "[1e309]", "[")]
        for labels, error in cases:
            with self.subTest(labels=labels):
                client, transport = route_client(routes())
                with action_env(INPUT_PR_NUMBER="42", INPUT_REQUIRED_LABEL="", INPUT_REQUIRED_LABELS=labels) as output, \
                        mock.patch.object(authorization.GitHubClient, "from_env", return_value=client):
                    with self.assertRaisesRegex(ActionError, f"required_labels.*{error}"):
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
    def test_required_label_categories_before_requests(self, component):
        valid = " " + component + " "
        self.assertEqual(authorization.parse_required_label(valid), valid)
        for bad in (None, (), [], [None], [1], [""], [" \t"], [component, component]):
            client, transport = route_client({})
            with self.assertRaises(ActionError):
                authorization.authorize_labels(client, 42, bad)
            self.assertEqual(transport.requests, [])

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
