import copy
import unittest
from urllib.parse import quote

from hypothesis import given, strategies as st

from ci_actions.github import ActionError
from ci_actions.run_binding import (
    Producer, RunBinding, RunOrigin, bind_pull_request, bind_run_to_pull_request, check_origin,
    list_pull_request_candidates, list_run_jobs,
)
from tests.helpers import REPO_PATH, REPOSITORY, SHA, json_response, route_client
from tests.property_support import (
    BRANCH, POSITIVE_ID, REPOSITORY as REPOSITORY_STRATEGY, SHA as SHA_STRATEGY, configure_profiles,
)

configure_profiles()

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


def generated_fixtures(repository, head_repository, branch, sha, run_id, workflow_id, producer=E2E):
    """Build independent API objects for every generated example."""
    value = fixtures()
    value["run"].update(
        id=run_id, workflow_id=workflow_id, name=producer.workflow_name,
        repository={"full_name": repository}, head_repository={"full_name": head_repository},
        head_branch=branch, head_sha=sha,
    )
    value["workflow"].update(id=workflow_id, name=producer.workflow_name, path=producer.workflow_path)
    value["pull"]["base"]["repo"]["full_name"] = repository
    value["pull"]["head"].update(ref=branch, sha=sha, repo={"full_name": head_repository})
    return value


def invalid_ids(valid_id):
    return (None, False, True, 0, -valid_id, 2**53, str(valid_id), float(valid_id), [], {})


def candidate_routes(repository, run_origin, pulls, candidates=None):
    repo_path = f"/repos/{repository}"
    owner = run_origin.head_repository.split("/", 1)[0]
    head = quote(f"{owner}:{run_origin.head_branch}", safe="")
    routes = {
        f"{repo_path}/pulls?state=all&head={head}&per_page=100&page=1":
            candidates if candidates is not None else [{"number": p["number"]} for p in pulls],
    }
    routes.update({f"{repo_path}/pulls/{p['number']}": p for p in pulls})
    return routes


class OriginProperties(unittest.TestCase):
    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY, branch=BRANCH,
           sha=SHA_STRATEGY, run_id=POSITIVE_ID, workflow_id=POSITIVE_ID, producer=st.sampled_from(PRODUCERS))
    def test_valid_origin_preserves_all_security_fields(
        self, repository, head_repository, branch, sha, run_id, workflow_id, producer,
    ):
        value = generated_fixtures(repository, head_repository, branch, sha, run_id, workflow_id, producer)
        expected = RunOrigin(producer, run_id, head_repository, branch, sha)
        self.assertEqual(check_origin(value["run"], value["workflow"], repository, run_id, PRODUCERS), expected)

    @given(run_id=POSITIVE_ID, workflow_id=POSITIVE_ID)
    def test_requested_run_and_run_workflow_ids_reject_every_invalid_category(self, run_id, workflow_id):
        for field in ("requested", "run", "run_workflow"):
            for invalid in invalid_ids(run_id if field != "run_workflow" else workflow_id):
                value = generated_fixtures(REPOSITORY, "contributor/aptos-core", "feature", SHA, run_id, workflow_id)
                requested = run_id
                if field == "requested":
                    requested = invalid
                else:
                    value["run"]["id" if field == "run" else "workflow_id"] = invalid
                with self.assertRaises(ActionError, msg=f"{field}: {invalid!r}"):
                    check_origin(value["run"], value["workflow"], REPOSITORY, requested, PRODUCERS)

        for invalid in (*invalid_ids(workflow_id), workflow_id % (2**53 - 1) + 1):
            value = generated_fixtures(REPOSITORY, "contributor/aptos-core", "feature", SHA, run_id, workflow_id)
            value["workflow"]["id"] = invalid
            with self.assertRaises(ActionError, msg=f"workflow mismatch: {invalid!r}"):
                check_origin(value["run"], value["workflow"], REPOSITORY, run_id, PRODUCERS)

    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY,
           branch=BRANCH, sha=SHA_STRATEGY)
    def test_malformed_head_fields_reject_every_category(self, repository, head_repository, branch, sha):
        malformed = {
            "head_repository": (None, [], "owner/repo", {}, {"full_name": None},
                                {"full_name": head_repository.replace("/", "")},
                                {"full_name": f"{head_repository}/extra"},
                                {"full_name": "owner/répo"}, {"full_name": "owner/repo\x00"}),
            "head_branch": (None, False, [], "", f"{branch}\x00", f"{branch}\n", f"{branch}\x7f", "b" * 256),
            "head_sha": (None, False, [], sha[:-1], f"{sha}0", f"A{sha[1:]}", f"g{sha[1:]}", f"{sha}\n"),
        }
        for field, invalid_values in malformed.items():
            for invalid in invalid_values:
                value = generated_fixtures(repository, head_repository, branch, sha, 1234, 55)
                value["run"][field] = invalid
                with self.assertRaises(ActionError, msg=f"{field}: {invalid!r}"):
                    check_origin(value["run"], value["workflow"], repository, 1234, PRODUCERS)

    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY,
           branch=BRANCH, sha=SHA_STRATEGY, producer=st.sampled_from(PRODUCERS))
    def test_trusted_repository_event_status_path_and_display_names_must_match(
        self, repository, head_repository, branch, sha, producer,
    ):
        mutations = (
            ("repository", lambda x: x["run"]["repository"].update(full_name=f"{repository}-other")),
            ("missing repository", lambda x: x["run"].update(repository=None)),
            ("event", lambda x: x["run"].update(event=f"pull_request_target-{branch}")),
            ("pull_request event", lambda x: x["run"].update(event="pull_request")),
            ("status", lambda x: x["run"].update(status=f"completed-{branch}")),
            ("workflow path", lambda x: x["workflow"].update(path=f"{producer.workflow_path}-{branch}")),
            ("run display name", lambda x: x["run"].update(name=f"{producer.workflow_name}-{branch}")),
            ("workflow display name", lambda x: x["workflow"].update(name=f"{producer.workflow_name}-{branch}")),
            ("run mismatch", lambda x: x["run"].update(id=1235)),
            ("missing run", lambda x: x.update(run=None)),
            ("missing workflow", lambda x: x.update(workflow=None)),
        )
        for category, mutate in mutations:
            value = generated_fixtures(repository, head_repository, branch, sha, 1234, 55, producer)
            mutate(value)
            with self.assertRaises(ActionError, msg=category):
                check_origin(value["run"], value["workflow"], repository, 1234, PRODUCERS)


class BindingProperties(unittest.TestCase):
    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY, branch=BRANCH,
           sha=SHA_STRATEGY, run_id=POSITIVE_ID,
           numbers=st.lists(POSITIVE_ID, min_size=3, max_size=3, unique=True),
           order=st.permutations((0, 1, 2)))
    def test_candidate_order_cannot_change_zero_one_or_multiple_match_result(
        self, repository, head_repository, branch, sha, run_id, numbers, order,
    ):
        value = generated_fixtures(repository, head_repository, branch, sha, run_id, 55)
        run_origin = RunOrigin(E2E, run_id, head_repository, branch, sha)
        for matches in (0, 1, 2):
            pulls = []
            for index, number in enumerate(numbers):
                candidate = copy.deepcopy(value["pull"])
                candidate["number"] = number
                candidate["state"] = "open" if index % 2 else "closed"
                if index >= matches:
                    candidate["head"]["sha"] = ("1" if sha[0] == "0" else "0") + sha[1:]
                pulls.append(candidate)
            for permutation in (tuple(order), tuple(reversed(order))):
                ordered = [copy.deepcopy(pulls[index]) for index in permutation]
                client, transport = route_client(candidate_routes(repository, run_origin, ordered), repository=repository)
                if matches == 1:
                    expected = RunBinding(E2E, run_id, numbers[0], sha, head_repository, branch)
                    self.assertEqual(bind_pull_request(client, run_origin), expected)
                else:
                    with self.assertRaisesRegex(ActionError, "exactly one pull request"):
                        bind_pull_request(client, run_origin)
                expected_reads = [f"/repos/{repository}/pulls/{p['number']}" for p in ordered]
                if matches == 1:
                    expected_reads.append(f"/repos/{repository}/pulls/{numbers[0]}")
                self.assertEqual(transport.urls()[1:], expected_reads)

    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY,
           branch=BRANCH, sha=SHA_STRATEGY, number=POSITIVE_ID)
    def test_only_exact_head_identity_matches(self, repository, head_repository, branch, sha, number):
        run_origin = RunOrigin(E2E, 1234, head_repository, branch, sha)
        mutations = (
            lambda p: p["head"].update(repo={"full_name": f"{head_repository}-other"}),
            lambda p: p["head"].update(ref=f"{branch}-other"),
            lambda p: p["head"].update(sha=("1" if sha[0] == "0" else "0") + sha[1:]),
            lambda p: p["head"].update(repo=None),
        )
        for mutate in mutations:
            value = generated_fixtures(repository, head_repository, branch, sha, 1234, 55)
            candidate = value["pull"]
            candidate["number"] = number
            mutate(candidate)
            client, _ = route_client(candidate_routes(repository, run_origin, [candidate]), repository=repository)
            with self.assertRaisesRegex(ActionError, "exactly one pull request"):
                bind_pull_request(client, run_origin)

    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY,
           branch=BRANCH, number=POSITIVE_ID)
    def test_duplicate_and_malformed_candidate_numbers_reject_every_category(
        self, repository, head_repository, branch, number,
    ):
        run_origin = RunOrigin(E2E, 1234, head_repository, branch, SHA)
        malformed = [[{"number": number}, {"number": number}], [None], [{}], [[]]]
        malformed.extend([[{"number": invalid}] for invalid in invalid_ids(number)])
        for candidates in malformed:
            client, transport = route_client(
                candidate_routes(repository, run_origin, [], candidates=candidates), repository=repository,
            )
            with self.assertRaises(ActionError, msg=f"candidates: {candidates!r}"):
                list_pull_request_candidates(client, run_origin)
            self.assertEqual(len(transport.requests), 1)

    @given(repository=REPOSITORY_STRATEGY, head_repository=REPOSITORY_STRATEGY,
           branch=BRANCH, sha=SHA_STRATEGY,
           numbers=st.lists(POSITIVE_ID, min_size=2, max_size=2, unique=True))
    def test_invalid_candidate_metadata_fails_even_with_one_valid_match(
        self, repository, head_repository, branch, sha, numbers,
    ):
        run_origin = RunOrigin(E2E, 1234, head_repository, branch, sha)
        mutations = (
            ("foreign base", lambda p: p["base"]["repo"].update(full_name=f"{repository}-other"),
             "base repository does not match the trusted repository"),
            ("missing base", lambda p: p.update(base=None), "base metadata is missing"),
            ("missing base repository", lambda p: p["base"].update(repo=None),
             "base repository does not match the trusted repository"),
            ("invalid state", lambda p: p.update(state=f"open-{branch}"), "invalid pull request state"),
            ("merged state", lambda p: p.update(state="merged"), "invalid pull request state"),
            ("mismatched number", lambda p: p.update(number=numbers[0]), "mismatched pull request"),
            ("string number", lambda p: p.update(number=str(numbers[1])), "mismatched pull request"),
            ("float number", lambda p: p.update(number=float(numbers[1])), "mismatched pull request"),
            ("boolean number", lambda p: p.update(number=True), "mismatched pull request"),
            ("missing head", lambda p: p.update(head=None), "head metadata is invalid"),
            ("short SHA", lambda p: p["head"].update(sha=sha[:-1]), "head metadata is invalid"),
            ("uppercase SHA", lambda p: p["head"].update(sha=f"A{sha[1:]}"), "head metadata is invalid"),
            ("missing SHA", lambda p: p["head"].update(sha=None), "head metadata is invalid"),
        )
        for category, mutate, expected_error in mutations:
            value = generated_fixtures(repository, head_repository, branch, sha, 1234, 55)
            matching = value["pull"]
            matching["number"] = numbers[0]
            malformed = copy.deepcopy(matching)
            malformed["number"] = numbers[1]
            mutate(malformed)
            routes = candidate_routes(repository, run_origin, [matching])
            routes.update(candidate_routes(repository, run_origin, [], candidates=[
                {"number": numbers[0]}, {"number": numbers[1]},
            ]))
            routes[f"/repos/{repository}/pulls/{numbers[1]}"] = malformed
            client, _ = route_client(routes, repository=repository)
            with self.assertRaisesRegex(ActionError, expected_error, msg=category):
                bind_pull_request(client, run_origin)


class CheckOriginTests(unittest.TestCase):
    def test_producer_is_matched_by_display_name_not_key(self):
        docker = Producer("docker-build-test", ".github/workflows/docker-build-test.yaml", "Build+Test Docker Images")
        value = fixtures()
        value["run"]["name"] = value["workflow"]["name"] = "Build+Test Docker Images"
        value["workflow"]["path"] = docker.workflow_path
        self.assertEqual(check_origin(value["run"], value["workflow"], REPOSITORY, 1234, [docker]).producer, docker)


class BindingTests(unittest.TestCase):
    def test_head_change_after_matching_fails_as_stale(self):
        responses = [pull(99), pull(99, sha="f" * 40)]
        routes = candidate_routes(REPOSITORY, origin(), [pull(99)])
        routes[f"{REPO_PATH}/pulls/99"] = lambda request: json_response(responses.pop(0))
        client, _ = route_client(routes)
        with self.assertRaisesRegex(ActionError, "stale workflow run"):
            bind_pull_request(client, origin())

    def test_candidate_listing_is_bounded(self):
        prefix = f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Freport-fix&per_page=100"
        full_pages = {f"{prefix}&page={page}": [{"number": (page - 1) * 100 + index + 1} for index in range(100)]
                      for page in range(1, 12)}
        client, _ = route_client(full_pages)
        with self.assertRaisesRegex(ActionError, "pagination limit"):
            list_pull_request_candidates(client, origin())

    def test_invalid_run_workflow_id_fails_before_workflow_lookup(self):
        client, transport = route_client({f"{REPO_PATH}/actions/runs/1234": {"id": 1234, "workflow_id": "55"}})
        with self.assertRaisesRegex(ActionError, "workflow ID"):
            bind_run_to_pull_request(client, 1234, PRODUCERS)
        self.assertEqual(len(transport.requests), 1)


class RunJobsTests(unittest.TestCase):
    def test_fails_closed_on_count_mismatch_and_invalid_run_id(self):
        prefix = f"{REPO_PATH}/actions/runs/1234/jobs?filter=latest&per_page=100"
        client, _ = route_client({f"{prefix}&page=1": {"total_count": 2, "jobs": [{"id": 1}]}})
        with self.assertRaisesRegex(ActionError, "does not match"):
            list_run_jobs(client, 1234)
        with self.assertRaisesRegex(ActionError, "run ID"):
            list_run_jobs(client, 0)


if __name__ == "__main__":
    unittest.main()
