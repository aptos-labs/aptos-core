"""Tests for ci_actions.forge_report (docker-forge-pr-report)."""

from __future__ import annotations

import unittest

from ci_actions import forge_report
from ci_actions.docker_plan import load_manifest
from ci_actions.forge_report import PRODUCER, report_comments, run_url
from ci_actions.github import ActionError
from ci_actions.run_binding import RunBinding
from tests.helpers import REPO_PATH, REPOSITORY, SHA, RouteTransport, action_env, read_outputs

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


if __name__ == "__main__":
    unittest.main()
