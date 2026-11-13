# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import importlib.util
import contextlib
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location("verex_bench", Path(__file__).with_name("verex-bench.py"))
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


class BenchmarkOutcomeTest(unittest.TestCase):
    problem = {"expected_rejections": ["amm::negative"]}

    def result(self, outcomes, errors=2):
        return {"status": "failed", "outcomes": outcomes, "errors": errors}

    def test_only_declared_rejection_is_expected(self):
        result = self.result([
            {"target": "amm::positive", "status": "verified", "errors": 0},
            {"target": "amm::negative", "status": "rejected", "errors": 2},
        ])
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "expected rejection")

    def test_timeout_is_not_expected_rejection(self):
        result = self.result([{"target": "amm::negative", "status": "timeout", "errors": 2}])
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "timeout")

    def test_unrelated_rejection_is_not_hidden(self):
        result = self.result([
            {"target": "amm::positive", "status": "rejected", "errors": 1},
            {"target": "amm::negative", "status": "rejected", "errors": 2},
        ], errors=3)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_unattributed_error_is_not_hidden(self):
        result = self.result([{"target": "amm::negative", "status": "rejected", "errors": 2}], errors=3)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_inconsistent_success_cannot_satisfy_expected_rejection(self):
        result = {"status": "verified", "errors": 0, "outcomes": [
            {"target": "amm::negative", "status": "rejected", "errors": 1},
        ]}
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_missing_target_and_accepted_negative_are_failures(self):
        self.assertEqual(bench.expected_outcome(self.problem, self.result([]))["status"], "failed")
        result = self.result([{"target": "amm::negative", "status": "verified", "errors": 0}], errors=0)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "unexpected acceptance")

    def test_suite_includes_rejections_and_available_measurements(self):
        point = {"problems": [
            {"status": "verified", "heartbeats": {"total": 10}},
            {"status": "expected rejection", "heartbeats": {"total": 20}},
            {"status": "failed", "heartbeats": {"total": 30}},
            {"status": "timeout", "heartbeats": {"total": 40}},
            {"status": "crashed"},
        ]}
        self.assertEqual(bench.suite_value(point, "heartbeats"), 100)
        self.assertIsNone(bench.suite_value({**point, "subset": True}, "heartbeats"))

    def test_comparison_includes_rejections_and_displays_status(self):
        before = {"problems": [
            {"name": "amm", "status": "failed", "wall_ms": {"total": 100},
             "heartbeats": {"total": 1500}},
            {"name": "missing", "status": "crashed"},
        ]}
        after = {"problems": [
            {"name": "amm", "status": "expected rejection", "wall_ms": {"total": 20},
             "heartbeats": {"total": 100}},
            {"name": "missing", "status": "verified", "wall_ms": {"total": 5},
             "heartbeats": {"total": 50}},
        ]}
        rows = bench.local_comparison(before, after)
        self.assertEqual(rows, [("amm", 1500, 100, 1.0, 1.0, "failed", "expected rejection")])
        changes = bench.latest_changes([before, after], "amm")
        self.assertAlmostEqual(changes["heartbeats"], -93.33333333333333)


class FrameworkBenchmarkTest(unittest.TestCase):
    def test_selection_is_cost_ranked_and_resolves_address_aliases(self):
        selected = bench.framework_selection(
            ["aptos_framework::a", "aptos_framework::b", "aptos_framework::c", "aptos_framework::d"],
            {"0x1::a": 20, "aptos_framework::b": 90, "aptos_framework::c": 10}, 0.3)
        self.assertEqual(selected["modules"], ["aptos_framework::b", "aptos_framework::a"])
        self.assertEqual(selected["corpus_modules"], 4)

    def test_stage_results_preserve_budget_failure_and_interruption(self):
        spec = importlib.util.spec_from_file_location(
            "verify_modules", Path(__file__).with_name("verify-modules.py"))
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)
        targets = runner.targets_of("""1 good: semantics
10 good: theorem start
20 good: theorem done, 100 heartbeats, verified, automatic
21 costly: theorem start
31 costly: theorem done, 300000001 heartbeats, budget, automatic
32 wrong: theorem start
36 wrong: theorem done, 20 heartbeats, failed, authored
37 unfinished: theorem start
""")
        self.assertEqual([target["status"] for target in targets],
                         ["verified", "budget", "failed", "interrupted"])
        self.assertEqual(targets[0]["wall_ms"], 10)
        self.assertNotIn("heartbeats", targets[-1])
        sweep = {"modules": [{"module": "a::m", "outcome": "timeout", "wall": 2,
                              "cpu": 1, "targets": targets}]}
        problem = bench.framework_problems(sweep)[0]
        self.assertEqual(problem["status"], "timeout")
        self.assertEqual(problem["heartbeats"]["total"], 300000121)
        self.assertEqual(len(problem["outcomes"]), 4)
        self.assertEqual(len(problem["targets"]), 3)
        point = {"subset": True, "flavor": "framework", "problems": [problem]}
        self.assertEqual(bench.suite_value(point, "heartbeats"), 300000121)


@unittest.skipUnless(os.name == "posix" and hasattr(os, "wait4"), "runner uses POSIX process groups")
class BenchmarkConcurrencyTest(unittest.TestCase):
    child_source = r'''
import json, os, subprocess, sys, time
from pathlib import Path
root, name, output, delay, barrier, mode = sys.argv[1:]
root, output = Path(root), Path(output)
pid = os.getpid()
(root / f"start-{pid}.json").write_text(json.dumps(
    {"name": name, "pid": pid, "time": time.monotonic_ns()}))
if mode in ("tree", "invalid"):
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    (root / f"descendant-{pid}.json").write_text(json.dumps({"pid": child.pid}))
deadline = time.monotonic() + 5
while len(list(root.glob("start-*.json"))) < int(barrier):
    if time.monotonic() > deadline:
        raise RuntimeError("children did not overlap")
    time.sleep(0.01)
time.sleep(float(delay))
if mode == "invalid":
    output.write_text("{")
else:
    output.write_text(json.dumps({"status": "verified", "wall_ms": {"total": round(float(delay)*1000)},
                                  "heartbeats": {"total": 1}}))
(root / f"end-{pid}.json").write_text(json.dumps({"pid": pid, "time": time.monotonic_ns()}))
'''

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.child = self.root / "child.py"
        self.child.write_text(self.child_source)

    def problem(self, name, delay=0.03, barrier=0, mode="normal", repeat=1):
        return {"name": name, "kind": "lean", "file": "unused.lean", "delay": delay,
                "barrier": barrier, "mode": mode, "repeat": repeat}

    def command(self, problem, executable, out):
        return [sys.executable, str(self.child), str(self.root), problem["name"], str(out),
                str(problem["delay"]), str(problem["barrier"]), problem["mode"]]

    def records(self, prefix):
        return [json.loads(path.read_text()) for path in self.root.glob(f"{prefix}-*.json")]

    def run_problems(self, problems, jobs, timeout=None):
        progress = io.StringIO()
        with mock.patch.object(bench, "command_of", self.command), \
                mock.patch.object(bench, "input_id", return_value="test"), \
                contextlib.redirect_stderr(progress):
            result = bench.measure_problems(problems, None, dict(os.environ), self.root, jobs,
                                            timeout=timeout)
        return result, progress.getvalue()

    def intervals(self):
        ends = {entry["pid"]: entry["time"] for entry in self.records("end")}
        return [(entry["name"], entry["pid"], entry["time"], ends[entry["pid"]])
                for entry in self.records("start")]

    def peak(self, intervals):
        events = sorted([(start, 1) for _, _, start, _ in intervals]
                        + [(end, -1) for _, _, _, end in intervals])
        active = peak = 0
        for _, change in events:
            active += change
            peak = max(peak, active)
        return peak

    def test_real_process_overlap_cap_order_and_sequential_repeats(self):
        problems = [self.problem("slow", 0.25, 2, repeat=2), self.problem("fast", 0.02, 2),
                    self.problem("third", 0.03), self.problem("fourth", 0.03)]
        results, progress = self.run_problems(problems, 2)
        self.assertEqual([result["name"] for result in results], [p["name"] for p in problems])
        intervals = self.intervals()
        self.assertEqual(len({pid for _, pid, _, _ in intervals}), 5)
        self.assertEqual(self.peak(intervals), 2)
        slow = sorted((start, end) for name, _, start, end in intervals if name == "slow")
        self.assertLessEqual(slow[0][1], slow[1][0])
        self.assertEqual(len(results[0]["repeats"]["heartbeats"]), 2)
        self.assertIn("active 2/2", progress)
        self.assertIn("PID ", progress)
        completions = [line for line in progress.splitlines() if "/4] " in line]
        self.assertTrue(completions[0].startswith("[1/4] fast:"))
        self.assertTrue(completions[-1].startswith("[4/4] slow:"))

    def test_jobs_one_runs_actual_children_sequentially(self):
        names = ["first", "second", "third"]
        results, _ = self.run_problems([self.problem(name) for name in names], 1)
        intervals = sorted(self.intervals(), key=lambda entry: entry[2])
        self.assertEqual(self.peak(intervals), 1)
        self.assertEqual([entry[0] for entry in intervals], names)
        self.assertEqual([entry["name"] for entry in results], names)

    @staticmethod
    def running(pid):
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        stat = Path(f"/proc/{pid}/stat")
        try:
            # A killed orphan may await init's reaping; it cannot execute.
            return stat.read_text().rsplit(")", 1)[1].split()[0] != "Z"
        except FileNotFoundError:
            return False if Path("/proc").exists() else True

    def wait_for(self, predicate):
        deadline = time.monotonic() + 5
        while not predicate():
            if time.monotonic() >= deadline:
                self.fail("subprocess condition did not complete")
            time.sleep(0.01)

    def driver(self, worker_error=False):
        problems = ([self.problem("bad", 0.1, 2, "invalid"),
                     self.problem("long", 30, 2, "tree"), self.problem("queued")]
                    if worker_error else
                    [self.problem("left", 30, 2, "tree"), self.problem("right", 30, 2, "tree")])
        harness = self.root / "driver.py"
        harness.write_text('''
import importlib.util, os, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location("bench", sys.argv[1])
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)
root = Path(sys.argv[2])
bench.command_of = lambda p, executable, out: [sys.executable, str(root / "child.py"),
    str(root), p["name"], str(out), str(p["delay"]), str(p["barrier"]), p["mode"]]
bench.input_id = lambda p: "test"
bench.measure_problems(''' + repr(problems) + ''', None, dict(os.environ), root, 2)
''')
        process = subprocess.Popen([sys.executable, str(harness), str(Path(bench.__file__)),
                                    str(self.root)], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   text=True)
        self.addCleanup(self.cleanup_driver, process)
        return process

    def cleanup_driver(self, process):
        for entry in self.records("start"):
            if self.running(entry["pid"]):
                bench.kill_process_group(entry["pid"])
        if process.poll() is None:
            process.kill()
        process.communicate(timeout=5)

    def assert_children_stopped(self):
        pids = [entry["pid"] for entry in self.records("start") + self.records("descendant")]
        self.assertGreaterEqual(len(pids), 3)
        self.wait_for(lambda: not any(self.running(pid) for pid in pids))

    def check_signal(self, signum):
        process = self.driver()
        self.wait_for(lambda: len(self.records("descendant")) == 2)
        process.send_signal(signum)
        _, errors = process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("stopping all active problem process groups", errors)
        self.assert_children_stopped()

    def test_ctrl_c_stops_all_process_groups(self):
        self.check_signal(signal.SIGINT)

    def test_sigterm_stops_all_process_groups(self):
        self.check_signal(signal.SIGTERM)

    def test_worker_error_stops_other_groups_and_queued_work(self):
        process = self.driver(worker_error=True)
        _, errors = process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("JSONDecodeError", errors)
        self.assertNotIn("queued", [entry["name"] for entry in self.records("start")])
        self.assert_children_stopped()

    def test_timeout_stops_its_descendants_and_records_timeout(self):
        problem = {**self.problem("timeout", 30, mode="tree"), "timeout": 0.2}
        result, _ = self.run_problems([problem, self.problem("after")], 1)
        self.assertEqual([entry["status"] for entry in result], ["timeout", "verified"])
        self.assertIn("wall-time limit", result[0]["error_messages"][0])
        pids = [entry["pid"] for entry in self.records("start") + self.records("descendant")]
        self.assertEqual(len(pids), 3)
        self.wait_for(lambda: not any(self.running(pid) for pid in pids))

    def test_whole_case_deadline_covers_repeats_and_cli_overrides_manifest(self):
        problem = {**self.problem("repeated", 0.15, repeat=8), "timeout": 30}
        started = time.monotonic()
        results, _ = self.run_problems([problem, self.problem("after")], 1, timeout=0.45)
        self.assertEqual([entry["status"] for entry in results], ["timeout", "verified"])
        self.assertEqual(results[0]["timeout_seconds"], 0.45)
        self.assertLess(time.monotonic() - started, 2)
        starts = [entry for entry in self.records("start") if entry["name"] == "repeated"]
        self.assertGreaterEqual(len(starts), 2)
        self.assertLess(len(starts), 8)
        self.assertNotIn("heartbeats", results[0])

    def test_existing_four_argument_attempt_launcher_still_works(self):
        result = {"status": "verified", "wall_ms": {"total": 1}, "cpu_ms": 1,
                  "heartbeats": {"total": 1}}
        calls = []
        def launcher(problem, executable, env, workdir):
            calls.append(problem["name"])
            return result
        with mock.patch.object(bench, "attempt", launcher), \
                mock.patch.object(bench, "input_id", return_value="test"):
            measured = bench.measure(self.problem("custom", repeat=2), None, {}, self.root)
        self.assertEqual(calls, ["custom", "custom"])
        self.assertEqual(measured["status"], "verified")


class BenchmarkJobsOptionsTest(unittest.TestCase):
    def test_local_and_ci_defaults_and_explicit_sequential_mode(self):
        with mock.patch.dict(os.environ, {}, clear=True), mock.patch.object(os, "cpu_count", return_value=8):
            self.assertEqual(bench.parser().parse_args(["run"]).jobs, 4)
            args = bench.parser().parse_args(["run", "--jobs", "1", "--threads", "1"])
            self.assertEqual((args.jobs, args.threads), (1, 1))
        with mock.patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}, clear=True):
            self.assertEqual(bench.parser().parse_args(["run"]).jobs, 1)
            self.assertEqual(bench.parser().parse_args(["run", "--jobs", "3"]).jobs, 3)
        with mock.patch.dict(os.environ, {"CI": "true"}, clear=True):
            self.assertEqual(bench.default_jobs(), 1)
        with mock.patch.dict(os.environ, {}, clear=True), mock.patch.object(os, "cpu_count", return_value=None):
            self.assertEqual(bench.default_jobs(), 1)

    def test_one_thread_requests_synchronous_target_execution(self):
        original = {"VEREX_OPTIONS": "verex.verifyHeartbeats=123,verex.jobs=8,Elab.async=true"}
        completed = subprocess.CompletedProcess([], 0, stdout=json.dumps(original), stderr="")
        with mock.patch.object(bench, "run_process_group", return_value=completed), \
                mock.patch.object(Path, "is_file", return_value=True):
            sequential = bench.bench_environment(Path("package"), 1)
            parallel = bench.bench_environment(Path("package"), 8)
        self.assertEqual(sequential["LEAN_NUM_THREADS"], "1")
        self.assertEqual(sequential["VEREX_OPTIONS"],
                         original["VEREX_OPTIONS"] + ",verex.jobs=1,Elab.async=false")
        self.assertEqual(parallel["VEREX_OPTIONS"], original["VEREX_OPTIONS"])
        self.assertEqual(parallel["LEAN_NUM_THREADS"], "8")

    def test_timeout_default_and_validation(self):
        self.assertEqual(bench.DEFAULT_TIMEOUT, 300)
        self.assertIsNone(bench.parser().parse_args(["run"]).timeout)
        self.assertEqual(bench.parser().parse_args(["run", "--timeout", "2.5"]).timeout, 2.5)
        for value in ("0", "-1", "nan", "inf"):
            with self.subTest(value=value), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    bench.parser().parse_args(["run", "--timeout", value])

    def test_invalid_worker_counts_are_rejected(self):
        for option in ("--jobs", "--threads"):
            for value in ("0", "-1"):
                with self.subTest(option=option, value=value), contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        bench.parser().parse_args(["run", option, value])


class BenchmarkReportFilterTest(unittest.TestCase):
    def test_full_run_filter_keeps_failures_and_does_not_change_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            history = root / "history.jsonl"
            runs = [{"schema": bench.SCHEMA, "version": bench.VERSION, "branch": "test",
                     "commit": str(index) * 40, "date": f"2026-11-0{index}T00:00:00+00:00",
                     "runner": "test", "cpu": "test", "threads": 1, "subset": subset,
                     "problems": [{"name": "target", "kind": "lean", "group": "VerexSL",
                                   "input": "test", "status": status,
                                   "wall_ms": {"total": 1}, "heartbeats": {"total": 1}}]}
                    for index, (subset, status) in enumerate(
                        [(False, "failed"), (True, "verified"), (False, "timeout")], 1)]
            original = "".join(json.dumps(run) + "\n" for run in runs)
            history.write_text(original)
            output = root / "report.html"
            args = bench.parser().parse_args(["report", "--no-history", "--local-runs",
                                              "--full-runs-only", "--html", str(output)])
            with mock.patch.object(bench, "LOCAL_HISTORY", history), \
                    mock.patch.object(bench, "current_branch", return_value="test"), \
                    mock.patch.object(bench, "page", wraps=bench.page) as page, \
                    contextlib.redirect_stderr(io.StringIO()):
                bench.report(args)
            rendered = page.call_args.args[0]
            self.assertEqual([point["problems"][0]["status"] for point in rendered],
                             ["failed", "timeout"])
            self.assertEqual(history.read_text(), original)
            self.assertTrue(output.is_file())

    def test_concurrency_metadata_handles_historical_runs(self):
        self.assertEqual(bench.concurrency_description({"threads": 8}),
                         "1 problem processes, 8 Lean threads per process")
        self.assertEqual(bench.concurrency_description({"threads": 8, "flavor": "framework"}),
                         "8 module processes")


if __name__ == "__main__":
    unittest.main()
