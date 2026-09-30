import io
import json
import stat
import unittest
import zipfile
from datetime import datetime, timedelta, timezone
from types import SimpleNamespace
from unittest.mock import Mock, patch

from hypothesis import given, strategies as st

from ci_actions import pr_ci_report
from ci_actions.github import ActionError, HttpResponse
from ci_actions.pr_ci_report import (
    MAX_ARCHIVE_BYTES, benchmark_job_disposition, build_comment, main, read_report_archive, select_report_artifact,
)
from tests.helpers import REPO_PATH, REPOSITORY, SHA, json_route, local_server, route_client, run_command, run_main
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, configure_profiles
from tests.test_report_schema import e2e_report

configure_profiles()

JOB_RESULTS = ("action_required", "cancelled", "failure", "neutral", "skipped",
               "stale", "startup_failure", "success", "timed_out")
EXECUTED_RESULTS = ("cancelled", "failure", "neutral", "success", "timed_out")
PRODUCER_STEPS = {
    "mono-move-e2e-perf": ("mono-move-e2e-perf", "Run mono-move e2e performance comparison"),
    "mono-move-micro-bench": ("mono-move-micro-bench", "Run A/B benchmark gate"),
}

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
    def test_rejects_boolean_and_float_job_run_ids(self):
        for value in (True, 1.0):
            with self.subTest(value=value), self.assertRaises(ActionError):
                benchmark_job_disposition([job(run_id=value)], "mono-move-e2e-perf", 1)

    def test_malformed_job_conclusions_raise_action_error(self):
        for value in ([], {}, None, True, 0):
            with self.subTest(value=value), self.assertRaises(ActionError):
                benchmark_job_disposition([job(conclusion=value)], "mono-move-e2e-perf", 1234)

    def test_malformed_step_conclusions_raise_action_error(self):
        for value in ([], {}, None, True, 0):
            with self.subTest(value=value), self.assertRaises(ActionError):
                benchmark_job_disposition([job(steps=[step(conclusion=value)])], "mono-move-e2e-perf", 1234)

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

    def test_rejects_boolean_and_float_artifact_run_ids(self):
        for value in (True, 1.0):
            with self.subTest(value=value), self.assertRaises(ActionError):
                select_report_artifact([{**self.VALID, "workflow_run": {"id": value}}], 1)

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


class ReportArchiveProperties(unittest.TestCase):
    @given(payload=st.binary(max_size=4096), permissions=st.integers(min_value=0, max_value=0o777))
    def test_archive_round_trip_preserves_payload(self, payload, permissions):
        zipped = archive([("pr-ci-report-v1.json", payload, stat.S_IFREG | permissions, 3)])
        self.assertEqual(payload, read_report_archive(zipped))

    @given(payload=st.binary(max_size=256), name=SAFE_COMPONENT)
    def test_archive_structure_tampering_rejects(self, payload, name):
        regular = stat.S_IFREG | 0o644
        members = [("pr-ci-report-v1.json", payload, regular, 3)]
        cases = [[], members * 2, members + [(name + ".txt", payload, regular, 3)]]
        for prefix in ("../", "./", "/", "nested/", name + "-"):
            cases.append([(prefix + "pr-ci-report-v1.json", payload, regular, 3)])
        for mode in (stat.S_IFDIR, stat.S_IFLNK, stat.S_IFIFO, stat.S_IFSOCK, stat.S_IFCHR, stat.S_IFBLK, 0):
            cases.append([("pr-ci-report-v1.json", payload, mode | 0o644, 3)])
        for system in (0, 1, 2, 4):
            cases.append([("pr-ci-report-v1.json", payload, regular, system)])
        for entries in cases:
            with self.assertRaises(ActionError):
                read_report_archive(archive(entries))

    def test_archive_exact_size_and_actual_size_guards(self):
        regular = stat.S_IFREG | 0o644
        payload = b"a" * (1024 * 1024)
        self.assertEqual(payload, read_report_archive(archive([("pr-ci-report-v1.json", payload, regular, 3)])))
        with self.assertRaisesRegex(ActionError, "^report exceeds one MiB$"):
            read_report_archive(archive([("pr-ci-report-v1.json", payload + b"a", regular, 3)]))
        # Reach the reader's own guard independently of zipfile's CRC/size rejection.
        for actual, declared in ((b"a", 2), (b"ab", 1), (payload + b"a", 1024 * 1024)):
            member = SimpleNamespace(filename="pr-ci-report-v1.json", create_system=3,
                                     external_attr=regular << 16, file_size=declared)
            zipped = Mock()
            zipped.__enter__ = Mock(return_value=zipped)
            zipped.__exit__ = Mock(return_value=False)
            zipped.infolist.return_value = [member]
            zipped.open.return_value = io.BytesIO(actual)
            with self.subTest(declared=declared, actual=len(actual)), patch.object(
                pr_ci_report.zipfile, "ZipFile", return_value=zipped
            ), self.assertRaisesRegex(ActionError, "actual size"):
                read_report_archive(b"ignored")

    def test_archive_corruption_rejects(self):
        valid = bytearray(archive([("pr-ci-report-v1.json", b"payload", stat.S_IFREG | 0o644, 3)]))
        # Local-header payload begins after fixed header, filename and extra lengths.
        offset = 30 + int.from_bytes(valid[26:28], "little") + int.from_bytes(valid[28:30], "little")
        valid[offset] ^= 1
        for bad in (b"", b"not a zip", bytes(valid), bytes(valid[:-22])):
            with self.subTest(length=len(bad)), self.assertRaisesRegex(ActionError, "malformed artifact ZIP"):
                read_report_archive(bad)


class ReportDispositionProperties(unittest.TestCase):
    @given(run_id=POSITIVE_ID, job_id=POSITIVE_ID, number=POSITIVE_ID,
           producer=st.sampled_from(tuple(PRODUCER_STEPS)), conclusion=st.sampled_from(JOB_RESULTS),
           step_conclusion=st.sampled_from(EXECUTED_RESULTS),
           offset=st.integers(min_value=-23 * 60 - 59, max_value=23 * 60 + 59),
           duration=st.integers(min_value=0, max_value=3600), unrelated=st.lists(SAFE_COMPONENT, max_size=8))
    def test_disposition_uses_exact_names_and_absolute_instants(
        self, run_id, job_id, number, producer, conclusion, step_conclusion, offset, duration, unrelated
    ):
        job_name, step_name = PRODUCER_STEPS[producer]
        start = datetime(2026, 1, 1, 12, tzinfo=timezone.utc)
        local = start.astimezone(timezone(timedelta(minutes=offset)))
        executed = step(step_name, step_conclusion, number=number, started_at=local.isoformat(),
                        completed_at=(start + timedelta(seconds=duration)).isoformat())
        target = job(conclusion, [executed], name=job_name, id=job_id, run_id=run_id)
        noise = [job(name="unrelated-" + name) for name in unrelated] + [None, {}]
        expected = "skip" if conclusion == "skipped" else "report"
        self.assertEqual(expected, benchmark_job_disposition(noise + [target], producer, run_id))
        self.assertEqual(expected, benchmark_job_disposition([target] + list(reversed(noise)), producer, run_id))

    def test_each_disposition_rejection_category(self):
        with self.assertRaises(ActionError):
            benchmark_job_disposition([job()], "unknown", 1234)
        for jobs in ([], [None, {}], [job(), job(id=2)]):
            with self.subTest(jobs=jobs), self.assertRaises(ActionError):
                benchmark_job_disposition(jobs, "mono-move-e2e-perf", 1234)
        for field, values in {
            "id": (None, True, 0, -1, 1.0, "1", [], {}),
            "run_id": (None, True, 0, -1, 1.0, "1234", [], {}, 1235),
            "status": (None, [], "in_progress"),
            "conclusion": (None, [], {}, True, "unknown"),
            "steps": (None, {}, "", 1, True, [], [None], [step(), step(number=5)]),
        }.items():
            for value in values:
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    benchmark_job_disposition([{**job(), field: value}], "mono-move-e2e-perf", 1234)
        for field, values in {
            "number": (None, True, 0, -1, 1.0, "1", [], {}),
            "name": (None, "similarly named", []),
            "status": (None, [], "in_progress"),
            "conclusion": (None, [], {}, True, "skipped", "unknown"),
            "started_at": (None, [], "", "2020-02-30T00:00:00Z", "2020-01-01T00:00:00+24:00",
                           "2020-01-01T00:00:00-08:60", "2027-01-01T00:00:00Z"),
            "completed_at": (None, {}, "", "invalid", "2020-01-01T00:00:00Z"),
        }.items():
            for value in values:
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    benchmark_job_disposition([job(steps=[step(**{field: value})])], "mono-move-e2e-perf", 1234)
        for conclusion in JOB_RESULTS:
            for step_conclusion in EXECUTED_RESULTS:
                self.assertEqual("skip" if conclusion == "skipped" else "report", benchmark_job_disposition(
                    [job(conclusion, [step(conclusion=step_conclusion)])], "mono-move-e2e-perf", 1234))
        self.assertEqual("skip", benchmark_job_disposition([job("skipped", steps="ignored")], "mono-move-e2e-perf", 1234))


class ReportArtifactProperties(unittest.TestCase):
    @given(run_id=POSITIVE_ID, artifact_id=POSITIVE_ID, size=st.integers(min_value=1, max_value=2 * 1024 * 1024),
           names=st.lists(SAFE_COMPONENT, max_size=8))
    def test_selection_is_bound_and_permutation_invariant(self, run_id, artifact_id, size, names):
        valid = {"id": artifact_id, "name": "pr-ci-report-v1", "expired": False,
                 "size_in_bytes": size, "workflow_run": {"id": run_id}}
        noise = [{"name": "unrelated-" + name} for name in names]
        self.assertIsNone(select_report_artifact(noise, run_id))
        self.assertEqual(valid, select_report_artifact(noise + [valid], run_id))
        self.assertEqual(valid, select_report_artifact([valid] + list(reversed(noise)), run_id))
        with self.assertRaisesRegex(ActionError, "exactly one"):
            select_report_artifact([valid, {**valid, "id": artifact_id + 1}], run_id)
        with self.assertRaisesRegex(ActionError, "originating run"):
            select_report_artifact([{**valid, "workflow_run": {"id": run_id + 1}}], run_id)

    def test_each_artifact_rejection_category_and_size_boundary(self):
        valid = ArtifactSelectionTests.VALID
        for bad in (None, [], {}, {"name": None}, {"name": ""}, {"name": 1}, {"name": []}):
            with self.subTest(unrelated=bad), self.assertRaises(ActionError):
                select_report_artifact([valid, bad], 1234)
        for field, values in {
            "id": (None, True, 0, -1, 1.0, "1", [], {}),
            "expired": (None, True, 0, 1, "false", [], {}),
            "size_in_bytes": (None, True, 0, -1, 1.0, "1", [], {}, 2 * 1024 * 1024 + 1),
            "workflow_run": (None, [], {}, {"id": None}, {"id": 1234.0}, {"id": "1234"}, {"id": 1235}),
        }.items():
            for value in values:
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    select_report_artifact([{**valid, field: value}], 1234)
        for size in (1, 2 * 1024 * 1024):
            accepted = {**valid, "size_in_bytes": size}
            self.assertEqual(accepted, select_report_artifact([accepted], 1234))


class ReportBuildProperties(unittest.TestCase):
    @given(run_id=POSITIVE_ID, pr_number=POSITIVE_ID, has_artifact=st.booleans(), skipped=st.booleans())
    def test_build_comment_orders_effects_and_validates_payload(self, run_id, pr_number, has_artifact, skipped):
        producer = SimpleNamespace(key="mono-move-e2e-perf")
        origin = SimpleNamespace(producer=producer)
        binding = SimpleNamespace(producer=producer, run_id=run_id, pr_number=pr_number, head_sha=SHA)
        report = e2e_report(run_id=run_id, pr_number=pr_number)
        artifacts = [{**ARTIFACT, "workflow_run": {"id": run_id}}] if has_artifact else []
        calls = []
        client = Mock()
        client.get_bytes.side_effect = lambda *args, **kwargs: calls.append("download") or report_archive(report)
        with patch.object(pr_ci_report, "inspect_run", side_effect=lambda *args: calls.append("inspect") or origin), \
                patch.object(pr_ci_report, "list_run_jobs", side_effect=lambda *args: calls.append("jobs") or [
                    job("skipped" if skipped else "success", run_id=run_id)
                ]), patch.object(pr_ci_report, "bind_pull_request", side_effect=lambda *args: calls.append("bind") or binding), \
                patch.object(pr_ci_report, "paginate", side_effect=lambda *args, **kwargs: calls.append("artifacts") or artifacts) as listing:
            result = build_comment(client, run_id)
        expected = None if skipped else report if has_artifact else {**report, "status": "failed", "metrics": []}
        self.assertEqual(expected, result)
        self.assertEqual(["inspect", "jobs"] if skipped else ["inspect", "jobs", "bind", "artifacts"]
                         + (["download"] if has_artifact else []), calls)
        if has_artifact and not skipped:
            client.get_bytes.assert_called_once_with("/actions/artifacts/808/zip", max_bytes=2 * 1024 * 1024,
                                                     follow_redirect=True)
        else:
            client.get_bytes.assert_not_called()
        if not skipped:
            listing.assert_called_once_with(client, f"/actions/runs/{run_id}/artifacts", items_key="artifacts",
                                             total_count_key="total_count", max_pages=100)

    @given(run_id=POSITIVE_ID, pr_number=POSITIVE_ID)
    def test_build_rejects_every_report_binding_tamper(self, run_id, pr_number):
        producer = SimpleNamespace(key="mono-move-e2e-perf")
        binding = SimpleNamespace(producer=producer, run_id=run_id, pr_number=pr_number, head_sha=SHA)
        changes = {"run_id": run_id + 1 if run_id < 2**53 - 1 else run_id - 1,
                   "pr_number": pr_number + 1 if pr_number < 2**53 - 1 else pr_number - 1,
                   "head_sha": "f" * 40, "producer": "mono-move-micro-bench"}
        with patch.object(pr_ci_report, "inspect_run", return_value=SimpleNamespace(producer=producer)), \
                patch.object(pr_ci_report, "list_run_jobs", return_value=[job(run_id=run_id)]), \
                patch.object(pr_ci_report, "bind_pull_request", return_value=binding), \
                patch.object(pr_ci_report, "paginate", return_value=[{**ARTIFACT, "workflow_run": {"id": run_id}}]):
            for key, value in changes.items():
                client = Mock()
                client.get_bytes.return_value = report_archive(e2e_report(**{
                    "run_id": run_id, "pr_number": pr_number, key: value,
                }))
                with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                    build_comment(client, run_id)


if __name__ == "__main__":
    unittest.main()
