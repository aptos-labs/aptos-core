import io
import json
import stat
import unittest
import zipfile

from ci_actions.github import ActionError, HttpResponse
from ci_actions.pr_ci_report import (
    MAX_ARCHIVE_BYTES, benchmark_job_disposition, build_comment, main, read_report_archive, select_report_artifact,
)
from tests.helpers import REPO_PATH, REPOSITORY, SHA, json_route, local_server, route_client, run_command, run_main
from tests.test_report_schema import e2e_report

E2E_STEP = "Run mono-move e2e performance comparison"


def archive(entries):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as zipped:
        for name, data, mode, system in entries:
            info = zipfile.ZipInfo(name)
            info.create_system = system
            info.external_attr = mode << 16
            zipped.writestr(info, data)
    return output.getvalue()


def report_archive(report=None):
    payload = json.dumps(report or e2e_report()).encode()
    return archive([("pr-ci-report-v1.json", payload, stat.S_IFREG | 0o644, 3)])


def step(name=E2E_STEP, conclusion="success", **overrides):
    return {"number": 4, "name": name, "status": "completed", "conclusion": conclusion,
            "started_at": "2026-09-17T10:00:00Z", "completed_at": "2026-09-17T10:05:00Z", **overrides}


def job(conclusion="success", steps=None, **overrides):
    return {"id": 701, "run_id": 1234, "name": "mono-move-e2e-perf", "status": "completed",
            "conclusion": conclusion, "steps": [step()] if steps is None else steps, **overrides}


class ArchiveTests(unittest.TestCase):
    def test_reads_exactly_one_regular_bounded_report(self):
        self.assertEqual(read_report_archive(report_archive()), json.dumps(e2e_report()).encode())

    def test_rejects_extra_files_directories_symlinks_traversal_and_non_unix_members(self):
        regular = stat.S_IFREG | 0o644
        cases = [
            [("pr-ci-report-v1.json", b"{}", regular, 3), ("extra.txt", b"x", regular, 3)],
            [("nested/", b"", stat.S_IFDIR | 0o755, 3)],
            [("pr-ci-report-v1.json", b"target", stat.S_IFLNK | 0o777, 3)],
            [("../pr-ci-report-v1.json", b"{}", regular, 3)],
            [("pr-ci-report-v1.json", b"{}", regular, 0)],
            [],
        ]
        for index, entries in enumerate(cases):
            with self.subTest(index=index), self.assertRaises(ActionError):
                read_report_archive(archive(entries))
        with self.assertRaisesRegex(ActionError, "malformed artifact ZIP"):
            read_report_archive(b"not a zip")

    def test_rejects_declared_or_actual_content_over_one_mib(self):
        big = archive([("pr-ci-report-v1.json", b"a" * (1024 * 1024 + 1), stat.S_IFREG | 0o644, 3)])
        with self.assertRaisesRegex(ActionError, "one MiB"):
            read_report_archive(big)


class DispositionTests(unittest.TestCase):
    def test_distinguishes_skipped_approval_from_executed_producer_steps(self):
        self.assertEqual(benchmark_job_disposition([job("skipped", [])], "mono-move-e2e-perf", 1234), "skip")
        self.assertEqual(benchmark_job_disposition([job()], "mono-move-e2e-perf", 1234), "report")
        micro = job("failure", [step("Run A/B benchmark gate", "failure")], name="mono-move-micro-bench")
        self.assertEqual(benchmark_job_disposition([micro], "mono-move-micro-bench", 1234), "report")
        with self.assertRaisesRegex(ActionError, "exactly one"):
            benchmark_job_disposition([], "mono-move-e2e-perf", 1234)
        with self.assertRaisesRegex(ActionError, "exactly one"):
            benchmark_job_disposition([job(), job(id=702)], "mono-move-e2e-perf", 1234)
        with self.assertRaisesRegex(ActionError, "conclusion"):
            benchmark_job_disposition([job("unknown")], "mono-move-e2e-perf", 1234)
        for overrides in ({"id": 0}, {"run_id": 999}, {"status": "in_progress"}):
            with self.subTest(overrides=overrides), self.assertRaisesRegex(ActionError, "job metadata"):
                benchmark_job_disposition([job(**overrides)], "mono-move-e2e-perf", 1234)

    def test_accepts_numeric_offsets_and_compares_absolute_instants(self):
        offsets = step(started_at="2020-01-20T09:42:40.000-08:00", completed_at="2020-01-20T17:43:40.000Z")
        self.assertEqual(benchmark_job_disposition([job(steps=[offsets])], "mono-move-e2e-perf", 1234), "report")

    def test_rejects_non_skipped_jobs_without_an_executed_benchmark_step(self):
        for conclusion, steps in [
            ("startup_failure", []), ("action_required", []), ("cancelled", []), ("success", []),
            ("success", [step(conclusion="skipped")]), ("success", [step("A similarly named benchmark step")]),
            ("success", "not-a-list"),
        ]:
            with self.subTest(conclusion=conclusion, steps=steps):
                with self.assertRaisesRegex(ActionError, "benchmark step"):
                    benchmark_job_disposition([job(conclusion, steps)], "mono-move-e2e-perf", 1234)

    def test_rejects_duplicate_steps_and_invalid_execution_timestamps(self):
        for steps in [
            [step(), step(number=5)],
            [step(number=0)],
            [step(status="in_progress")],
            [step(started_at=None)],
            [step(completed_at="not-a-timestamp")],
            [step(started_at="2020-01-20T09:42:40.000+24:00")],
            [step(started_at="2020-01-20T09:42:40.000-08:60")],
            [step(started_at="2020-02-30T09:42:40.000-08:00")],
            [step(started_at="2026-09-17T10:05:00Z", completed_at="2026-09-17T10:00:00Z")],
            [step(started_at="2020-01-20T09:42:40.000-08:00", completed_at="2020-01-20T17:41:40.000Z")],
        ]:
            with self.subTest(steps=steps), self.assertRaisesRegex(ActionError, "benchmark step"):
                benchmark_job_disposition([job(steps=steps)], "mono-move-e2e-perf", 1234)


class ArtifactSelectionTests(unittest.TestCase):
    VALID = {"id": 7, "name": "pr-ci-report-v1", "expired": False, "size_in_bytes": 100, "workflow_run": {"id": 1234}}

    def test_distinguishes_missing_and_rejects_duplicate_expired_foreign_and_oversized(self):
        valid = self.VALID
        self.assertEqual(select_report_artifact([valid, {**valid, "name": "other"}], 1234)["id"], 7)
        self.assertIsNone(select_report_artifact([], 1234))
        for artifacts, error in [
            ([None], "malformed"), ([{}], "malformed"), ([{**valid, "name": ""}], "malformed"),
            ([valid, {**valid, "id": 8}], "exactly one"),
            ([{**valid, "id": 0}], "artifact ID"),
            ([{**valid, "expired": True}], "expired"), ([{**valid, "expired": None}], "expired"),
            ([{**valid, "workflow_run": {"id": 999}}], "originating run"),
            ([{**valid, "size_in_bytes": MAX_ARCHIVE_BYTES + 1}], "archive size"),
            ([{**valid, "size_in_bytes": 0}], "archive size"),
        ]:
            with self.subTest(artifacts=artifacts), self.assertRaisesRegex(ActionError, error):
                select_report_artifact(artifacts, 1234)


def api_routes(jobs, artifacts, zip_response=None):
    runs = f"{REPO_PATH}/actions/runs/1234"
    routes = {
        runs: {"id": 1234, "name": "mono-move-e2e-perf", "event": "pull_request_target", "status": "completed",
               "workflow_id": 55, "repository": {"full_name": REPOSITORY},
               "head_repository": {"full_name": "contributor/aptos-core"}, "head_branch": "feature/report-fix",
               "head_sha": SHA, "pull_requests": []},
        f"{REPO_PATH}/actions/workflows/55": {"id": 55, "name": "mono-move-e2e-perf",
                                              "path": ".github/workflows/mono-move-e2e-perf.yaml"},
        f"{runs}/jobs?filter=latest&per_page=100&page=1": {"total_count": len(jobs), "jobs": jobs},
        f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Freport-fix&per_page=100&page=1": [{"number": 99}],
        f"{REPO_PATH}/pulls/99": {"number": 99, "state": "open", "base": {"repo": {"full_name": REPOSITORY}},
                                  "head": {"ref": "feature/report-fix", "sha": SHA,
                                           "repo": {"full_name": "contributor/aptos-core"}}},
        f"{runs}/artifacts?per_page=100&page=1": {"total_count": len(artifacts), "artifacts": artifacts},
    }
    if zip_response is not None:
        routes[f"{REPO_PATH}/actions/artifacts/808/zip"] = zip_response
    return routes


ARTIFACT = {"id": 808, "name": "pr-ci-report-v1", "expired": False, "size_in_bytes": 300, "workflow_run": {"id": 1234}}


class BuildCommentTests(unittest.TestCase):
    def test_skipped_benchmark_needs_no_pull_request_or_artifact_lookup(self):
        client, transport = route_client(api_routes([job("skipped", [])], []))
        self.assertIsNone(build_comment(client, 1234))
        self.assertFalse(any("/pulls" in url or "artifacts" in url for url in transport.urls()))

    def test_jobs_without_an_executed_step_fail_before_pull_request_lookup(self):
        for conclusion, steps in [("startup_failure", []), ("action_required", []), ("cancelled", []),
                                  ("success", []), ("success", [step(conclusion="skipped")])]:
            client, transport = route_client(api_routes([job(conclusion, steps)], []))
            with self.subTest(conclusion=conclusion), self.assertRaisesRegex(ActionError, "benchmark step"):
                build_comment(client, 1234)
            self.assertFalse(any("/pulls" in url or "artifacts" in url for url in transport.urls()))

    def test_executed_benchmark_without_artifact_renders_the_trusted_missing_report(self):
        for conclusion in ("success", "failure"):
            client, transport = route_client(api_routes([job(conclusion, [step(conclusion=conclusion)])], []))
            with self.subTest(conclusion=conclusion):
                self.assertEqual(build_comment(client, 1234), e2e_report(status="failed", metrics=[]))
                urls = transport.urls()
                self.assertEqual(urls.count(f"{REPO_PATH}/pulls/99"), 2)
                self.assertLess(max(i for i, url in enumerate(urls) if "/pulls" in url),
                                min(i for i, url in enumerate(urls) if "artifacts" in url))

    def test_downloads_the_bound_artifact_with_redirects_allowed(self):
        zip_bytes = report_archive()
        client, transport = route_client(api_routes([job()], [ARTIFACT], HttpResponse(200, {}, zip_bytes)))
        self.assertEqual(build_comment(client, 1234), e2e_report())
        download = transport.requests[-1]
        self.assertEqual(download["url"], f"{REPO_PATH}/actions/artifacts/808/zip")
        self.assertTrue(download["follow_redirect"])
        self.assertEqual(download["max_bytes"], MAX_ARCHIVE_BYTES)
        self.assertFalse(any(request["follow_redirect"] for request in transport.requests[:-1]))

    def test_report_bound_to_another_head_fails_closed(self):
        forged = report_archive(e2e_report(head_sha="f" * 40))
        client, _ = route_client(api_routes([job()], [ARTIFACT], HttpResponse(200, {}, forged)))
        with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
            build_comment(client, 1234)


class PrCiReportCommandTests(unittest.TestCase):
    def test_writes_rendered_comment_and_outputs_without_forwarding_the_token(self):
        zip_bytes = report_archive()

        def blob(handler):
            handler.send_response(200)
            handler.send_header("content-length", str(len(zip_bytes)))
            handler.end_headers()
            handler.wfile.write(zip_bytes)

        with local_server({"/blob.zip": blob}) as (blob_base, blob_requests):
            def redirect(handler):
                handler.send_response(302)
                handler.send_header("location", f"{blob_base}/blob.zip")
                handler.end_headers()

            routes = {path: json_route(value) for path, value in api_routes([job()], [ARTIFACT]).items()}
            routes[f"{REPO_PATH}/actions/artifacts/808/zip"] = redirect
            with local_server(routes) as (api_base, _):
                code, stderr, outputs, files = run_main("pr-ci-report", main, {
                    "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_API_URL": api_base, "GH_TOKEN": "test-token",
                    "INPUT_RUN_ID": "1234",
                })
        self.assertEqual(code, 0, stderr)
        self.assertEqual(
            {key: value for key, value in outputs.items() if key != "comment_path"},
            {"should_comment": "true", "comment_header": "mono-move-e2e-perf", "pr_number": "99"},
        )
        self.assertIn("apt-fa-transfer", files["comment_path"])
        self.assertIsNone(blob_requests[0]["authorization"])

    def test_skipped_run_writes_only_should_comment_false(self):
        routes = {path: json_route(value) for path, value in api_routes([job("skipped", [])], []).items()}
        with local_server(routes) as (api_base, _):
            code, stderr, outputs, _ = run_main("pr-ci-report", main, {
                "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_API_URL": api_base, "GH_TOKEN": "t", "INPUT_RUN_ID": "1234",
            })
        self.assertEqual(code, 0, stderr)
        self.assertEqual(outputs, {"should_comment": "false"})

    def test_invalid_run_id_fails_closed(self):
        code, stderr, outputs, _ = run_command("pr-ci-report", {
            "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_API_URL": "https://api.github.test", "GH_TOKEN": "t",
            "INPUT_RUN_ID": "0",
        })
        self.assertEqual(code, 1)
        self.assertIn("pr-ci-report failed closed: run_id", stderr)
        self.assertEqual(outputs, {})


if __name__ == "__main__":
    unittest.main()
