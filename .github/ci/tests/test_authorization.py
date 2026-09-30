import unittest

from ci_actions.authorization import DENIED, Authorization, authorize, main
from ci_actions.github import ActionError, HttpResponse
from tests.helpers import REPO_PATH, json_response, json_route, local_server, route_client, run_command, run_main

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
        # Covers the JS cases for foreign-origin, switched-endpoint, /repositories/,
        # non-page and duplicate next links: the next URL is always rebuilt locally.
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


if __name__ == "__main__":
    unittest.main()
