"""Tests for ci_actions.forge_report (docker-forge-pr-report)."""

from __future__ import annotations

import json
import unittest

from hypothesis import given, strategies as st

from ci_actions import forge_report
from ci_actions.docker_plan import load_manifest
from ci_actions.forge_report import PRODUCER, report_comments, run_url
from ci_actions.github import ActionError
from ci_actions.run_binding import RunBinding
from tests.helpers import REPO_PATH, REPOSITORY, SHA, RouteTransport, action_env, read_outputs
from tests.property_support import POSITIVE_ID, REPOSITORY as REPO_STRATEGY, SAFE_COMPONENT, configure_profiles

configure_profiles()

RESULTS = ("action_required", "cancelled", "failure", "neutral", "skipped",
           "stale", "startup_failure", "success", "timed_out")

STALE_SHA = "fedcba9876543210fedcba9876543210fedcba98"
RUN_ID = 1234
MARKERS = load_manifest().markers()
BINDING = RunBinding(PRODUCER, RUN_ID, 99, SHA, "contributor/aptos-core", "feature/forge-report")


def marker_jobs(**conclusions: str) -> list[dict]:
    """One completed job per manifest marker; keyword keys are marker IDs with '_' for '-'."""
    return [
        {
            "id": 700 + index,
            "run_id": RUN_ID,
            "head_sha": SHA,
            "name": marker.id,
            "status": "completed",
            "conclusion": conclusions.get(marker.id.replace("-", "_"), "success"),
        }
        for index, marker in enumerate(MARKERS)
    ]


def api(jobs: list[dict], *, head_sha: str = SHA, workflow_path: str = PRODUCER.workflow_path) -> RouteTransport:
    pull_request = {
        "number": 99,
        "state": "open",
        "base": {"repo": {"full_name": REPOSITORY}},
        "head": {"repo": {"full_name": "contributor/aptos-core"}, "ref": "feature/forge-report", "sha": head_sha},
    }
    return RouteTransport({
        f"{REPO_PATH}/actions/runs/{RUN_ID}": {
            "id": RUN_ID, "name": PRODUCER.workflow_name, "event": "pull_request_target",
            "status": "completed", "workflow_id": 55, "repository": {"full_name": REPOSITORY},
            "head_repository": {"full_name": "contributor/aptos-core"},
            "head_branch": "feature/forge-report", "head_sha": SHA, "pull_requests": [],
        },
        f"{REPO_PATH}/actions/workflows/55": {"id": 55, "name": PRODUCER.workflow_name, "path": workflow_path},
        f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Fforge-report&per_page=100&page=1": [{"number": 99}],
        f"{REPO_PATH}/pulls/99": pull_request,
        f"{REPO_PATH}/actions/runs/{RUN_ID}/jobs?filter=latest&per_page=100&page=1":
            {"total_count": len(jobs), "jobs": jobs},
    })


def run_main(transport: RouteTransport) -> dict[str, str]:
    with action_env(INPUT_RUN_ID=str(RUN_ID)) as output:
        forge_report.main(transport=transport)
        return read_outputs(output)


class ReportCommentTests(unittest.TestCase):
    def test_rejects_boolean_and_float_marker_run_ids(self):
        binding = RunBinding(PRODUCER, 1, 99, SHA, "contributor/aptos-core", "feature/forge-report")
        for value in (True, 1.0):
            jobs = marker_jobs()
            jobs[0]["run_id"] = value
            with self.subTest(value=value), self.assertRaises(ActionError):
                report_comments(MARKERS[:1], jobs[:1], binding)

    def test_run_url_rejects_empty_userinfo(self):
        for url in ("https://@github.com", "https://:@github.com"):
            with self.subTest(url=url), self.assertRaises(ActionError):
                run_url(url, REPOSITORY, RUN_ID)

    def test_run_url_malformed_hosts_and_ports_raise_action_error(self):
        for url in ("https://[github.com", "https://github.com:no", "https://github.com:65536",
                    "https://github.com:-1", "https://github.com:\uff11"):
            with self.subTest(url=url), self.assertRaises(ActionError):
                run_url(url, REPOSITORY, RUN_ID)

    def test_run_url_preserves_enterprise_path_and_valid_port(self):
        self.assertEqual("https://git.example:8443/enterprise/owner/repo/actions/runs/1",
                         run_url("https://git.example:8443/enterprise/", "owner/repo", 1))

    def test_duplicate_marker_fails_and_absent_marker_is_not_reported(self):
        jobs = marker_jobs()
        with self.assertRaisesRegex(ActionError, "at most one"):
            report_comments(MARKERS, jobs + [jobs[0]], BINDING)
        comments = report_comments(MARKERS, jobs[1:], BINDING)
        self.assertEqual([m.id for m in MARKERS[1:]], [c["key"] for c in comments])

    def test_unexpanded_skipped_matrix_job_is_ignored(self):
        skipped = {"id": 1, "run_id": RUN_ID, "head_sha": SHA, "name": "${{ matrix.marker }}",
                   "status": "completed", "conclusion": "skipped"}
        self.assertEqual([], report_comments(MARKERS, [skipped], BINDING))

    def test_marker_conclusion_and_metadata_are_validated(self):
        comments = report_comments(MARKERS, marker_jobs(forge_report_source_compat="failure",
                                                        forge_report_source_framework="skipped"), BINDING)
        by_key = {c["key"]: c for c in comments}
        self.assertEqual("failure", by_key["forge-report-source-compat"]["result"])
        self.assertEqual("forge-compat", by_key["forge-report-source-compat"]["header"])
        self.assertNotIn("forge-report-source-framework", by_key)
        for field, value in [("conclusion", "attacker-controlled"), ("conclusion", None), ("status", "in_progress"),
                             ("run_id", RUN_ID + 1), ("head_sha", STALE_SHA), ("id", 0), ("id", True)]:
            jobs = marker_jobs()
            jobs[0][field] = value
            with self.subTest(field=field, value=value), self.assertRaisesRegex(ActionError, "marker"):
                report_comments(MARKERS, jobs, BINDING)

    def test_run_url_requires_plain_https(self):
        self.assertEqual(f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}",
                         run_url("https://github.com/", REPOSITORY, RUN_ID))
        for bad in ["http://github.com", "https://user:pw@github.com", "https://github.com?x=1",
                    "https://github.com#x", "github.com"]:
            with self.subTest(bad=bad), self.assertRaisesRegex(ActionError, "server URL"):
                run_url(bad, REPOSITORY, RUN_ID)


class ReportMainTests(unittest.TestCase):
    def test_main_binds_fork_run_and_writes_report_outputs(self):
        outputs = run_main(api(marker_jobs(forge_report_source_compat="failure",
                                           forge_report_source_framework="skipped",
                                           forge_report_source_multiregion="skipped")))
        self.assertEqual(["pr_number", "head_sha", "run_url", "report_matrix"], list(outputs))
        self.assertEqual("99", outputs["pr_number"])
        self.assertEqual(SHA, outputs["head_sha"])
        self.assertEqual(f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}", outputs["run_url"])
        self.assertEqual(
            '{"include":['
            '{"key":"forge-report-source-e2e","header":"forge-e2e","title":"Protected Forge E2E result","result":"success"},'
            '{"key":"forge-report-source-compat","header":"forge-compat","title":"Protected Forge compatibility result","result":"failure"},'
            '{"key":"forge-report-source-performance","header":"forge-e2e-performance","title":"Protected Forge performance result","result":"success"},'
            '{"key":"forge-report-source-consensus","header":"consensus-only-realistic-env-max-tps","title":"Protected Forge consensus-only result","result":"success"}'
            ']}',
            outputs["report_matrix"],
        )

    def test_all_skipped_markers_serialize_an_empty_matrix(self):
        skipped = {m.id.replace("-", "_"): "skipped" for m in MARKERS}
        self.assertEqual('{"include":[]}', run_main(api(marker_jobs(**skipped)))["report_matrix"])
        self.assertEqual('{"include":[]}', run_main(api([]))["report_matrix"])

    def test_main_rejects_stale_head_and_foreign_workflow(self):
        for transport in (api(marker_jobs(), head_sha=STALE_SHA),
                          api(marker_jobs(), workflow_path=".github/workflows/other.yaml")):
            with self.subTest(transport=transport), self.assertRaises(ActionError):
                run_main(transport)


class ForgeReportProperties(unittest.TestCase):
    @given(states=st.lists(st.one_of(st.none(), st.sampled_from(RESULTS)),
                          min_size=len(MARKERS), max_size=len(MARKERS)),
           run_id=POSITIVE_ID, job_id=POSITIVE_ID, unrelated=st.lists(SAFE_COMPONENT, max_size=8))
    def test_marker_projection_uses_trusted_fields_and_order(self, states, run_id, job_id, unrelated):
        binding = RunBinding(PRODUCER, run_id, 99, SHA, "contributor/aptos-core", "feature/report")
        jobs = [
            {"id": job_id + index, "run_id": run_id, "head_sha": SHA, "name": marker.id,
             "status": "completed", "conclusion": state, "header": "@attacker", "title": "[untrusted]"}
            for index, (marker, state) in enumerate(zip(MARKERS, states)) if state is not None
        ]
        jobs.extend([{"name": "unrelated-" + name, "conclusion": []} for name in unrelated])
        jobs.extend([None, {}, {"name": "${{ matrix.marker }}", "conclusion": "skipped"}])
        expected = [{"key": marker.id, "header": marker.comment_header, "title": marker.title, "result": state}
                    for marker, state in zip(MARKERS, states) if state not in (None, "skipped")]
        self.assertEqual(expected, report_comments(MARKERS, jobs, binding))
        self.assertEqual(expected, report_comments(MARKERS, list(reversed(jobs)), binding))
        self.assertEqual({"include": expected}, json.loads(json.dumps({"include": report_comments(MARKERS, jobs, binding)})))

    def test_every_marker_rejection_category(self):
        for conclusion in RESULTS:
            jobs = marker_jobs()
            jobs[0]["conclusion"] = conclusion
            expected = [] if conclusion == "skipped" else [{
                "key": MARKERS[0].id, "header": MARKERS[0].comment_header,
                "title": MARKERS[0].title, "result": conclusion,
            }]
            self.assertEqual(expected, report_comments(MARKERS[:1], jobs[:1], BINDING))
        for field, values in {
            "id": (None, True, 0, -1, 1.0, "1", [], {}),
            "run_id": (None, True, 0, -1, 1234.0, "1234", [], {}, 1235),
            "head_sha": (None, [], "f" * 40),
            "status": (None, [], "in_progress"),
            "conclusion": (None, [], {}, True, 1, "", "unknown"),
        }.items():
            for value in values:
                jobs = marker_jobs()
                jobs[0][field] = value
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    report_comments(MARKERS[:1], jobs[:1], BINDING)
        for marker_index in range(len(MARKERS)):
            jobs = marker_jobs()
            with self.subTest(marker=marker_index), self.assertRaisesRegex(ActionError, "at most one"):
                report_comments(MARKERS, jobs + [jobs[marker_index]], BINDING)

    @given(host=SAFE_COMPONENT, repository=REPO_STRATEGY, run_id=POSITIVE_ID,
           port=st.integers(min_value=1, max_value=65535), path=st.lists(SAFE_COMPONENT, max_size=3),
           trailing=st.integers(min_value=0, max_value=3))
    def test_https_url_preserves_enterprise_prefix(self, host, repository, run_id, port, path, trailing):
        server = f"https://{host}.example:{port}" + ("/" + "/".join(path) if path else "")
        self.assertEqual(f"{server}/{repository}/actions/runs/{run_id}",
                         run_url(server + "/" * trailing, repository, run_id))
        for bad in (server.replace("https:", "http:", 1), server + "?x=1", server + "#x",
                    server.replace("https://", "https://user:password@", 1),
                    server.replace("https://", "https://@", 1)):
            with self.assertRaises(ActionError):
                run_url(bad, repository, run_id)

    def test_url_missing_host_and_plain_scheme_rejection(self):
        for server in ("", "github.com", "https://", "https:///path", "ftp://github.com"):
            with self.subTest(server=server), self.assertRaises(ActionError):
                run_url(server, REPOSITORY, RUN_ID)


if __name__ == "__main__":
    unittest.main()
