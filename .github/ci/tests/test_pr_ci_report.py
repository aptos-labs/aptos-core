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
from ci_actions.github import ActionError
from ci_actions.pr_ci_report import (
    REPORT_PRODUCERS, benchmark_job_disposition, build_comment, main, read_report_archive, select_report_artifact,
)
from ci_actions.run_binding import JOB_CONCLUSIONS
from tests.helpers import REPO_PATH, REPOSITORY, SHA, json_route, local_server, run_main
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, configure_profiles
from tests.report_support import e2e_report

configure_profiles()

JOB_RESULTS = tuple(sorted(JOB_CONCLUSIONS))
EXECUTED_RESULTS = ("cancelled", "failure", "neutral", "success", "timed_out")
E2E = REPORT_PRODUCERS["mono-move-e2e-perf"]


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


def step(name=E2E.benchmark_step, conclusion="success", **overrides):
    return {"number": 4, "name": name, "status": "completed", "conclusion": conclusion,
            "started_at": "2026-09-17T10:00:00Z", "completed_at": "2026-09-17T10:05:00Z", **overrides}


def job(conclusion="success", steps=None, **overrides):
    return {"id": 701, "run_id": 1234, "name": E2E.job, "status": "completed",
            "conclusion": conclusion, "steps": [step()] if steps is None else steps, **overrides}


def api_routes(jobs, artifacts):
    runs = f"{REPO_PATH}/actions/runs/1234"
    return {
        runs: {"id": 1234, "name": E2E.workflow_name, "event": "pull_request_target", "status": "completed",
               "workflow_id": 55, "repository": {"full_name": REPOSITORY},
               "head_repository": {"full_name": "contributor/aptos-core"}, "head_branch": "feature/report-fix",
               "head_sha": SHA, "pull_requests": []},
        f"{REPO_PATH}/actions/workflows/55": {"id": 55, "name": E2E.workflow_name, "path": E2E.workflow_path},
        f"{runs}/jobs?filter=latest&per_page=100&page=1": {"total_count": len(jobs), "jobs": jobs},
        f"{REPO_PATH}/pulls?state=all&head=contributor%3Afeature%2Freport-fix&per_page=100&page=1": [{"number": 99}],
        f"{REPO_PATH}/pulls/99": {"number": 99, "state": "open", "base": {"repo": {"full_name": REPOSITORY}},
                                  "head": {"ref": "feature/report-fix", "sha": SHA,
                                           "repo": {"full_name": "contributor/aptos-core"}}},
        f"{runs}/artifacts?per_page=100&page=1": {"total_count": len(artifacts), "artifacts": artifacts},
    }


ARTIFACT = {"id": 808, "name": "pr-ci-report-v1", "expired": False, "size_in_bytes": 300, "workflow_run": {"id": 1234}}


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
            {"should_comment": "true", "comment_header": E2E.key, "pr_number": "99"},
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
           producer=st.sampled_from(tuple(REPORT_PRODUCERS)), conclusion=st.sampled_from(JOB_RESULTS),
           step_conclusion=st.sampled_from(EXECUTED_RESULTS),
           offset=st.integers(min_value=-23 * 60 - 59, max_value=23 * 60 + 59),
           duration=st.integers(min_value=0, max_value=3600), unrelated=st.lists(SAFE_COMPONENT, max_size=8))
    def test_disposition_uses_exact_names_and_absolute_instants(
        self, run_id, job_id, number, producer, conclusion, step_conclusion, offset, duration, unrelated
    ):
        job_name, step_name = REPORT_PRODUCERS[producer].job, REPORT_PRODUCERS[producer].benchmark_step
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
                benchmark_job_disposition(jobs, E2E.key, 1234)
        for field, values in {
            "id": (None, True, 0, -1, 1.0, "1", [], {}),
            "run_id": (None, True, 0, -1, 1.0, "1234", [], {}, 1235),
            "status": (None, [], "in_progress"),
            "conclusion": (None, [], {}, True, "unknown"),
            "steps": (None, {}, "", 1, True, [], [None], [step(), step(number=5)]),
        }.items():
            for value in values:
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    benchmark_job_disposition([{**job(), field: value}], E2E.key, 1234)
        # Expected run ID 1 makes the boolean alias equal, so only the type check rejects it.
        for value in (True, 1.0):
            with self.subTest(run_id=value), self.assertRaises(ActionError):
                benchmark_job_disposition([job(run_id=value)], E2E.key, 1)
        for field, values in {
            "number": (None, True, 0, -1, 1.0, "1", [], {}),
            "name": (None, "similarly named", []),
            "status": (None, [], "in_progress"),
            "conclusion": (None, [], {}, True, "skipped", "unknown"),
            "started_at": (None, "invalid", "2027-01-01T00:00:00Z"),
            "completed_at": (None, "2020-01-01T00:00:00Z"),
        }.items():
            for value in values:
                with self.subTest(field=field, value=value), self.assertRaises(ActionError):
                    benchmark_job_disposition([job(steps=[step(**{field: value})])], E2E.key, 1234)
        for conclusion in JOB_RESULTS:
            for step_conclusion in EXECUTED_RESULTS:
                self.assertEqual("skip" if conclusion == "skipped" else "report", benchmark_job_disposition(
                    [job(conclusion, [step(conclusion=step_conclusion)])], E2E.key, 1234))
        self.assertEqual("skip", benchmark_job_disposition([job("skipped", steps="ignored")], E2E.key, 1234))


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
        valid = ARTIFACT
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
        for value in (True, 1.0):
            with self.subTest(workflow_run_id=value), self.assertRaises(ActionError):
                select_report_artifact([{**valid, "workflow_run": {"id": value}}], 1)


class ReportBuildProperties(unittest.TestCase):
    @given(run_id=POSITIVE_ID, pr_number=POSITIVE_ID, has_artifact=st.booleans(), skipped=st.booleans())
    def test_build_comment_orders_effects_and_validates_payload(self, run_id, pr_number, has_artifact, skipped):
        producer = SimpleNamespace(key=E2E.key)
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
        producer = SimpleNamespace(key=E2E.key)
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
