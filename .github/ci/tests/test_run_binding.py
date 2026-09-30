import copy
import unittest

from ci_actions.github import ActionError
from ci_actions.run_binding import (
    Producer, RunBinding, RunOrigin, bind_pull_request, bind_run_to_pull_request, check_origin,
    list_pull_request_candidates, list_run_jobs,
)
from tests.helpers import REPO_PATH, REPOSITORY, SHA, json_response, route_client

OLD_SHA = "a" * 40
E2E = Producer("mono-move-e2e-perf", ".github/workflows/mono-move-e2e-perf.yaml", "mono-move-e2e-perf")
MICRO = Producer("mono-move-micro-bench", ".github/workflows/mono-move-micro-bench.yaml", "mono-move-micro-bench")
PRODUCERS = (E2E, MICRO)


def fixtures():
    return {
        "run": {
            "id": 1234, "name": "mono-move-e2e-perf", "event": "pull_request_target", "status": "completed",
            "workflow_id": 55, "repository": {"full_name": REPOSITORY},
            "head_repository": {"full_name": "contributor/aptos-core"},
            "head_branch": "feature/report-fix", "head_sha": SHA, "pull_requests": [],
        },
        "workflow": {"id": 55, "name": "mono-move-e2e-perf", "path": ".github/workflows/mono-move-e2e-perf.yaml"},
        "pull": {
            "number": 99, "state": "open",
            "head": {"ref": "feature/report-fix", "sha": SHA, "repo": {"full_name": "contributor/aptos-core"}},
            "base": {"repo": {"full_name": REPOSITORY}},
        },
    }


def origin(**overrides):
    values = {"producer": E2E, "run_id": 1234, "head_repository": "contributor/aptos-core",
              "head_branch": "feature/report-fix", "head_sha": SHA, **overrides}
    return RunOrigin(**values)


def pull(number, **head):
    value = copy.deepcopy(fixtures()["pull"])
    value["number"] = number
    value["head"].update(head)
    return value


class CheckOriginTests(unittest.TestCase):
    def test_fork_run_binds_stable_workflow_and_exact_head_without_associated_prs(self):
        value = fixtures()
        self.assertEqual(check_origin(value["run"], value["workflow"], REPOSITORY, 1234, PRODUCERS), origin())

    def test_rejects_workflow_repository_event_run_and_head_mismatches(self):
        mutations = [
            lambda x: x["workflow"].update(path=".github/workflows/other.yaml"),
            lambda x: x["run"].update(name="lookalike"),
            lambda x: x["workflow"].update(name="lookalike"),
            lambda x: x["run"]["repository"].update(full_name="attacker/fork"),
            lambda x: x["run"].update(event="pull_request"),
            lambda x: x["run"].update(status="in_progress"),
            lambda x: x["run"].update(id=777),
            lambda x: x["run"].update(workflow_id=0),
            lambda x: x["workflow"].update(id=56),
            lambda x: x["run"]["head_repository"].update(full_name="invalid"),
            lambda x: x["run"].update(head_repository=None),
            lambda x: x["run"].update(head_branch=""),
            lambda x: x["run"].update(head_branch="a\x00b"),
            lambda x: x["run"].update(head_branch="b" * 256),
            lambda x: x["run"].update(head_sha="not-a-sha"),
        ]
        for index, mutate in enumerate(mutations):
            value = fixtures()
            mutate(value)
            with self.subTest(index=index), self.assertRaises(ActionError):
                check_origin(value["run"], value["workflow"], REPOSITORY, 1234, PRODUCERS)

    def test_docker_producer_uses_its_display_name(self):
        docker = Producer("docker-build-test", ".github/workflows/docker-build-test.yaml", "Build+Test Docker Images")
        value = fixtures()
        value["run"]["name"] = value["workflow"]["name"] = "Build+Test Docker Images"
        value["workflow"]["path"] = docker.workflow_path
        self.assertEqual(check_origin(value["run"], value["workflow"], REPOSITORY, 1234, [docker]).producer, docker)


class BindingTests(unittest.TestCase):
    def routes(self, *pulls, candidates=None):
        routes = {
            f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Freport-fix&per_page=100&page=1":
                candidates if candidates is not None else [{"number": p["number"]} for p in pulls],
        }
        for value in pulls:
            routes[f"{REPO_PATH}/pulls/{value['number']}"] = value
        return routes

    def test_fetches_every_candidate_and_selects_one_exact_match(self):
        client, transport = route_client(self.routes(pull(99), pull(100, sha=OLD_SHA)))
        binding = bind_pull_request(client, origin())
        self.assertEqual(binding, RunBinding(E2E, 1234, 99, SHA, "contributor/aptos-core", "feature/report-fix"))
        self.assertEqual(transport.urls()[1:], [f"{REPO_PATH}/pulls/{n}" for n in (99, 100, 99)])

    def test_fails_closed_on_zero_or_ambiguous_matches(self):
        for pulls in ([pull(99, sha=OLD_SHA)], [pull(99), pull(100)]):
            client, _ = route_client(self.routes(*pulls))
            with self.subTest(pulls=len(pulls)), self.assertRaisesRegex(ActionError, "exactly one pull request"):
                bind_pull_request(client, origin())

    def test_candidate_with_foreign_base_or_bad_state_fails_closed(self):
        for mutate in (
            lambda p: p["base"]["repo"].update(full_name="attacker/fork"),
            lambda p: p.update(state="merged"),
            lambda p: p.update(number=98),
            lambda p: p["head"].update(sha="bad"),
        ):
            value = pull(100)
            mutate(value)
            routes = self.routes(pull(99))
            routes[f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Freport-fix&per_page=100&page=1"] = [
                {"number": 99}, {"number": 100}]
            routes[f"{REPO_PATH}/pulls/100"] = value
            client, _ = route_client(routes)
            with self.assertRaises(ActionError):
                bind_pull_request(client, origin())

    def test_head_change_after_matching_fails_as_stale(self):
        responses = [pull(99), pull(99, sha="f" * 40)]
        routes = self.routes(pull(99))
        routes[f"{REPO_PATH}/pulls/99"] = lambda request: json_response(responses.pop(0))
        client, _ = route_client(routes)
        with self.assertRaisesRegex(ActionError, "stale workflow run"):
            bind_pull_request(client, origin())

    def test_pull_request_listing_encodes_owner_and_branch_and_is_bounded(self):
        branch_origin = origin(head_branch="feature/a+b")
        prefix = f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Fa%2Bb&per_page=100"
        client, transport = route_client({
            f"{prefix}&page=1": [{"number": index + 1} for index in range(100)],
            f"{prefix}&page=2": [{"number": 101}],
        })
        self.assertEqual(len(list_pull_request_candidates(client, branch_origin)), 101)
        self.assertEqual(transport.urls(), [f"{prefix}&page=1", f"{prefix}&page=2"])

        full_pages = {f"{prefix}&page={page}": [{"number": (page - 1) * 100 + index + 1} for index in range(100)]
                      for page in range(1, 12)}
        client, _ = route_client(full_pages)
        with self.assertRaisesRegex(ActionError, "pagination limit"):
            list_pull_request_candidates(client, branch_origin)

    def test_candidates_must_be_unique_positive_numbers(self):
        for candidates in ([{"number": 99}, {"number": 99}], [{"number": 0}], [None], [{"number": "99"}]):
            client, _ = route_client(self.routes(candidates=candidates))
            with self.subTest(candidates=candidates), self.assertRaises(ActionError):
                list_pull_request_candidates(client, origin())

    def test_bind_run_to_pull_request_reads_run_and_workflow_first(self):
        value = fixtures()
        routes = self.routes(pull(99))
        routes[f"{REPO_PATH}/actions/runs/1234"] = value["run"]
        routes[f"{REPO_PATH}/actions/workflows/55"] = value["workflow"]
        client, transport = route_client(routes)
        self.assertEqual(bind_run_to_pull_request(client, 1234, PRODUCERS).pr_number, 99)
        self.assertEqual(transport.urls()[:2], [f"{REPO_PATH}/actions/runs/1234", f"{REPO_PATH}/actions/workflows/55"])

    def test_invalid_run_workflow_id_fails_before_workflow_lookup(self):
        client, transport = route_client({f"{REPO_PATH}/actions/runs/1234": {"id": 1234, "workflow_id": "55"}})
        with self.assertRaisesRegex(ActionError, "workflow ID"):
            bind_run_to_pull_request(client, 1234, PRODUCERS)
        self.assertEqual(len(transport.requests), 1)


class RunJobsTests(unittest.TestCase):
    def test_lists_latest_jobs_across_pages_with_a_stable_total(self):
        prefix = f"{REPO_PATH}/actions/runs/1234/jobs?filter=latest&per_page=100"
        client, _ = route_client({
            f"{prefix}&page=1": {"total_count": 101, "jobs": [{"id": index} for index in range(100)]},
            f"{prefix}&page=2": {"total_count": 101, "jobs": [{"id": 100}]},
        })
        self.assertEqual(len(list_run_jobs(client, 1234)), 101)

    def test_fails_closed_on_count_mismatch_and_invalid_run_id(self):
        prefix = f"{REPO_PATH}/actions/runs/1234/jobs?filter=latest&per_page=100"
        client, _ = route_client({f"{prefix}&page=1": {"total_count": 2, "jobs": [{"id": 1}]}})
        with self.assertRaisesRegex(ActionError, "does not match"):
            list_run_jobs(client, 1234)
        with self.assertRaisesRegex(ActionError, "run ID"):
            list_run_jobs(client, 0)


if __name__ == "__main__":
    unittest.main()
