"""Tests for ci_actions.forge_report (docker-forge-pr-report)."""

from __future__ import annotations

import json
import unittest

from hypothesis import given, strategies as st

from ci_actions import forge_report
from ci_actions.docker_plan import load_manifest
from ci_actions.forge_report import PRODUCER, report_comments, run_url
from ci_actions.github import ActionError
from ci_actions.run_binding import JOB_CONCLUSIONS, RunBinding
from tests.helpers import REPO_PATH, REPOSITORY, SHA, RouteTransport, action_env, read_outputs
from tests.property_support import POSITIVE_ID, REPOSITORY as REPO_STRATEGY, SAFE_COMPONENT, configure_profiles

configure_profiles()

RESULTS = tuple(sorted(JOB_CONCLUSIONS))

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


class ReportMainTests(unittest.TestCase):
    def test_main_binds_fork_run_and_writes_report_outputs(self):
        results = {"forge-report-source-compat": "failure", "forge-report-source-framework": "skipped",
                   "forge-report-source-multiregion": "skipped"}
        outputs = run_main(api(marker_jobs(**{key.replace("-", "_"): value for key, value in results.items()})))
        self.assertEqual({"pr_number", "head_sha", "run_url", "report_matrix"}, set(outputs))
        self.assertEqual("99", outputs["pr_number"])
        self.assertEqual(SHA, outputs["head_sha"])
        self.assertEqual(f"https://github.com/{REPOSITORY}/actions/runs/{RUN_ID}", outputs["run_url"])
        expected = [{"key": m.id, "header": m.comment_header, "title": m.title, "result": results.get(m.id, "success")}
                    for m in MARKERS if results.get(m.id) != "skipped"]
        matrix = json.loads(outputs["report_matrix"])
        self.assertIn({"key": "forge-report-source-compat", "header": "forge-compat",
                       "title": "Protected Forge compatibility result", "result": "failure"}, matrix["include"])
        self.assertEqual({"include": expected}, matrix)

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
        # Binding run ID 1 makes the boolean alias equal, so only the type check rejects it.
        binding = RunBinding(PRODUCER, 1, 99, SHA, "contributor/aptos-core", "feature/forge-report")
        for value in (True, 1.0):
            jobs = marker_jobs()
            jobs[0]["run_id"] = value
            with self.subTest(run_id=value), self.assertRaises(ActionError):
                report_comments(MARKERS[:1], jobs[:1], binding)
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

    def test_run_url_rejects_malformed_servers(self):
        for server in ("", "github.com", "https://", "https:///path", "ftp://github.com", "https://[github.com",
                       "https://github.com:no", "https://github.com:65536", "https://github.com:-1",
                       "https://github.com:\uff11"):
            with self.subTest(server=server), self.assertRaises(ActionError):
                run_url(server, REPOSITORY, RUN_ID)


if __name__ == "__main__":
    unittest.main()
