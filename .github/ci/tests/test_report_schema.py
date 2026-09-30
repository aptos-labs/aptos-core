import copy
import json
import math
import string
import unittest
from itertools import product

from hypothesis import given, strategies as st

from ci_actions import report_schema
from tests.helpers import SHA
from tests.property_support import POSITIVE_ID, SHA as SHA_STRATEGY, configure_profiles

configure_profiles()

# Test-side protocol specification; these constants are independent of the validator.
CONTRACTS = {
    "mono-move-e2e-perf": {
        "verdicts": ("ok", "improvement", "regression", "noisy", "uncalibrated", "failed", "self-compare"),
        "numbers": {"v1_tps": (0, 1e12), "mono_tps": (0, 1e12),
                    "execution_speedup": (0, 1e6), "max_execution_spread": (0, 1e6)},
    },
    "mono-move-micro-bench": {
        "verdicts": ("regression", "improvement", "ok", "notable", "noise", "new", "absent",
                     "workload changed", "incomplete"),
        "numbers": {"mean_percent": (-1e6, 1e6), "ci_low_percent": (-1e6, 1e6),
                    "ci_high_percent": (-1e6, 1e6), "base_median_ns": (0, 1e18), "pr_median_ns": (0, 1e18)},
    },
}
SAFE_NAME = st.tuples(
    st.sampled_from(string.ascii_letters + string.digits),
    st.text(alphabet=string.ascii_letters + string.digits + "._/+ :-", max_size=31),
).map("".join)


def nullable_number(lower, upper):
    return st.one_of(st.none(), st.integers(min_value=int(lower), max_value=int(upper)),
                     st.floats(min_value=lower, max_value=upper, allow_nan=False, allow_infinity=False))


def reports(producer):
    contract = CONTRACTS[producer]
    row = st.fixed_dictionaries({
        "name": SAFE_NAME, "verdict": st.sampled_from(contract["verdicts"]),
        **{key: nullable_number(*bounds) for key, bounds in contract["numbers"].items()},
    })
    return st.fixed_dictionaries({
        "schema": st.just("pr-ci-report-v1"), "producer": st.just(producer),
        "run_id": POSITIVE_ID, "pr_number": POSITIVE_ID, "head_sha": SHA_STRATEGY,
        "status": st.sampled_from(("passed", "failed")), "metrics": st.lists(row, max_size=20),
    })


def trusted_binding(report):
    return {key: report[key] for key in ("producer", "run_id", "pr_number", "head_sha")}


def expected_cells(producer, row):
    def number(value, suffix=""):
        return "n/a" if value is None else f"{value:.2f}{suffix}"
    if producer == "mono-move-e2e-perf":
        spread = row["max_execution_spread"]
        cells = [number(row["v1_tps"]), number(row["mono_tps"]), number(row["execution_speedup"], "x"),
                 number(None if spread is None else spread * 100, "%")]
    else:
        low, high = row["ci_low_percent"], row["ci_high_percent"]
        interval = "n/a" if low is None or high is None else f"[{number(low, '%')}, {number(high, '%')}]"
        cells = [number(row["mean_percent"], "%"), interval,
                 number(row["base_median_ns"], "ns"), number(row["pr_median_ns"], "ns")]
    return [f"`{row['name']}`", row["verdict"], *cells]


BINDING = {"producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99, "head_sha": SHA}
MICRO_BINDING = {**BINDING, "producer": "mono-move-micro-bench"}


def e2e_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "passed",
        "metrics": [{"name": "apt-fa-transfer", "verdict": "ok", "v1_tps": 100.0, "mono_tps": 125.0,
                     "execution_speedup": 1.25, "max_execution_spread": 0.02}],
        **overrides,
    }


def micro_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-micro-bench", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "failed",
        "metrics": [{"name": "fib/mono", "verdict": "regression", "mean_percent": 4.2, "ci_low_percent": 3.1,
                     "ci_high_percent": 5.3, "base_median_ns": 1000.0, "pr_median_ns": 1042.0}],
        **overrides,
    }


class ReportSchemaTests(unittest.TestCase):
    def validate(self, report, binding=BINDING):
        return report_schema.parse_and_validate_report(json.dumps(report).encode(), binding)

    def test_valid_reports_for_both_producers_render_from_fixed_templates(self):
        e2e = report_schema.render_report(self.validate(e2e_report()))
        micro = report_schema.render_report(self.validate(micro_report(), MICRO_BINDING))
        self.assertRegex(e2e, r"^### MonoMove E2E performance")
        self.assertIn("| `apt-fa-transfer` | ok | 100.00 | 125.00 | 1.25x | 2.00% |", e2e)
        self.assertRegex(micro, r"^### MonoMove micro-benchmark")
        self.assertIn("| `fib/mono` | regression | 4.20% | [3.10%, 5.30%] | 1000.00ns | 1042.00ns |", micro)

    def test_rejects_forged_producer_pr_sha_and_run_id(self):
        for key, value in [("producer", "mono-move-micro-bench"), ("pr_number", 100),
                           ("head_sha", "f" * 40), ("run_id", 1235)]:
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                self.validate(e2e_report(), {**BINDING, key: value})

    def test_rejects_unknown_keys_invalid_enums_and_non_finite_values(self):
        with self.assertRaisesRegex(ValueError, "unknown key"):
            self.validate(e2e_report(extra=True))
        with self.assertRaisesRegex(ValueError, "missing key"):
            self.validate({key: value for key, value in e2e_report().items() if key != "status"})
        with self.assertRaisesRegex(ValueError, "invalid report schema"):
            self.validate(e2e_report(schema="pr-ci-report-v2"))
        with self.assertRaisesRegex(ValueError, "invalid report status"):
            self.validate(e2e_report(status="maybe"))
        report = e2e_report()
        report["metrics"][0]["verdict"] = "fast"
        with self.assertRaisesRegex(ValueError, "invalid verdict"):
            self.validate(report)
        report = e2e_report()
        report["metrics"][0]["v1_tps"] = math.inf
        with self.assertRaisesRegex(ValueError, "finite"):
            report_schema.validate_report(report, BINDING)
        for field, value in [("producer", []), ("status", []), ("run_id", True), ("pr_number", 0),
                             ("head_sha", "A" * 40), ("metrics", {})]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(e2e_report(**{field: value}))
        report = e2e_report()
        report["metrics"][0]["verdict"] = []
        with self.assertRaises(ValueError):
            self.validate(report)
        with self.assertRaisesRegex(ValueError, "positive safe integer"):
            self.validate(e2e_report(run_id=2**53), {**BINDING, "run_id": 2**53})

    def test_rejects_extreme_finite_metric_magnitudes(self):
        for field, value in [("v1_tps", -1.0), ("mono_tps", 1e308), ("execution_speedup", 1e308),
                             ("max_execution_spread", 1e308), ("max_execution_spread", 10**400)]:
            report = e2e_report()
            report["metrics"][0][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "bounded range"):
                self.validate(report)
        report = micro_report()
        report["metrics"][0]["mean_percent"] = 1e308
        with self.assertRaisesRegex(ValueError, "bounded range"):
            self.validate(report, MICRO_BINDING)

    def test_rejects_excess_rows_long_strings_mentions_controls_and_markdown(self):
        with self.assertRaisesRegex(ValueError, "at most 100"):
            self.validate(e2e_report(metrics=e2e_report()["metrics"] * 101))
        for name in ["a" * 129, "hello@team", "line\nbreak", "[link](bad)", "tick`code", "-leading", ""]:
            report = e2e_report()
            report["metrics"][0]["name"] = name
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "safe plain text"):
                self.validate(report)

    def test_rejects_malformed_json_non_standard_constants_and_oversized_reports(self):
        with self.assertRaisesRegex(ValueError, "malformed JSON"):
            report_schema.parse_and_validate_report(b"{", BINDING)
        with self.assertRaisesRegex(ValueError, "malformed JSON"):
            report_schema.parse_and_validate_report(b"\xff", BINDING)
        with self.assertRaisesRegex(ValueError, "finite"):
            report_schema.parse_and_validate_report(json.dumps(e2e_report()).replace("100.0", "NaN").encode(), BINDING)
        with self.assertRaisesRegex(ValueError, "one MiB"):
            report_schema.parse_and_validate_report(b" " * (1024 * 1024 + 1), BINDING)

    def test_rejects_duplicate_keys_at_every_object_level(self):
        root_duplicate = json.dumps(e2e_report())[:-1] + ',"status":"failed"}'
        row_duplicate = json.dumps(e2e_report()).replace('"verdict": "ok"', '"verdict": "ok", "verdict": "failed"')
        for payload in (root_duplicate, row_duplicate):
            with self.assertRaisesRegex(ValueError, "duplicate key"):
                report_schema.parse_and_validate_report(payload.encode(), BINDING)

    def test_missing_report_is_fixed_from_the_trusted_binding(self):
        report = report_schema.missing_report(BINDING)
        self.assertEqual(report, e2e_report(status="failed", metrics=[]))
        rendered = report_schema.render_report(report)
        self.assertIn("Validated result: **failed**", rendered)
        self.assertIn("No metric rows were produced", rendered)


class ReportSchemaProperties(unittest.TestCase):
    def assert_round_trip_and_render(self, report):
        original = copy.deepcopy(report)
        binding = trusted_binding(report)
        parsed = report_schema.parse_and_validate_report(json.dumps(report, allow_nan=False).encode(), binding)
        self.assertEqual(original, parsed)
        self.assertEqual(original, report)
        rendered = report_schema.render_report(parsed)
        title = {"mono-move-e2e-perf": "MonoMove E2E performance",
                 "mono-move-micro-bench": "MonoMove micro-benchmark"}[report["producer"]]
        self.assertTrue(rendered.startswith(f"### {title}\n\n"))
        self.assertIn(f"Validated result: **{report['status']}**. PR #{report['pr_number']} at "
                      f"`{report['head_sha']}` (originating run {report['run_id']}).", rendered)
        table = [line for line in rendered.splitlines() if line.startswith("|")]
        self.assertEqual(2 + len(report["metrics"]), len(table))
        for line, row in zip(table[2:], report["metrics"]):
            self.assertEqual(7, line.count("|"))
            self.assertEqual(expected_cells(report["producer"], row),
                             [cell.strip() for cell in line.split("|")[1:-1]])
        self.assertTrue(rendered.endswith("\n"))
        for unsafe in ("@", "<", ">", "[link]", "\r", "\x00", "\t"):
            self.assertNotIn(unsafe, rendered)
        self.assertEqual(2 + 2 * len(report["metrics"]), rendered.count("`"))
        self.assertEqual(not report["metrics"], "No metric rows were produced" in rendered)

    @given(report=reports("mono-move-e2e-perf"))
    def test_e2e_round_trip_and_safe_renderer(self, report):
        self.assert_round_trip_and_render(report)

    @given(report=reports("mono-move-micro-bench"))
    def test_micro_round_trip_and_safe_renderer(self, report):
        self.assert_round_trip_and_render(report)

    @given(report=st.one_of(*(reports(key) for key in CONTRACTS)))
    def test_each_binding_field_is_required_and_tamper_rejected(self, report):
        binding = trusted_binding(report)
        replacements = {"producer": next(key for key in CONTRACTS if key != report["producer"]),
                        "run_id": 1 if report["run_id"] != 1 else 2,
                        "pr_number": 1 if report["pr_number"] != 1 else 2,
                        "head_sha": ("1" if report["head_sha"][0] == "0" else "0") + report["head_sha"][1:]}
        for key in binding:
            with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                report_schema.validate_report(report, {**binding, key: replacements[key]})
            with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                report_schema.validate_report(report, {name: value for name, value in binding.items() if name != key})
        self.assertEqual({**binding, "schema": "pr-ci-report-v1", "status": "failed", "metrics": []},
                         report_schema.missing_report(binding))

    @given(report=st.one_of(*(reports(key) for key in CONTRACTS)))
    def test_each_object_key_and_duplicate_is_checked(self, report):
        binding = trusted_binding(report)
        for key in report:
            with self.assertRaisesRegex(ValueError, "missing key"):
                report_schema.validate_report({name: value for name, value in report.items() if name != key}, binding)
        with self.assertRaisesRegex(ValueError, "unknown key"):
            report_schema.validate_report({**report, "unexpected": None}, binding)
        row = report["metrics"][0] if report["metrics"] else {
            "name": "metric", "verdict": CONTRACTS[report["producer"]]["verdicts"][0],
            **dict.fromkeys(CONTRACTS[report["producer"]]["numbers"]),
        }
        for key in row:
            altered = {**report, "metrics": [{name: value for name, value in row.items() if name != key}]}
            with self.assertRaisesRegex(ValueError, "missing key"):
                report_schema.validate_report(altered, binding)
        with self.assertRaisesRegex(ValueError, "unknown key"):
            report_schema.validate_report({**report, "metrics": [{**row, "unexpected": None}]}, binding)
        for obj, key, is_root in ((report, "status", True), (row, "verdict", False)):
            duplicate = json.dumps(obj)[:-1] + "," + json.dumps(key) + ":" + json.dumps(obj[key]) + "}"
            payload = duplicate if is_root else "{" + ",".join(
                json.dumps(name) + ":" + ("[" + duplicate + "]" if name == "metrics" else json.dumps(value))
                for name, value in report.items()
            ) + "}"
            with self.assertRaisesRegex(ValueError, "duplicate key"):
                report_schema.parse_and_validate_report(payload.encode(), binding)

    def test_all_finite_rejection_categories_and_adjacent_bounds(self):
        for producer, contract in CONTRACTS.items():
            base = e2e_report() if producer == "mono-move-e2e-perf" else micro_report()
            binding = trusted_binding(base)
            for value in (None, [], "", 0, True):
                with self.subTest(root=value), self.assertRaisesRegex(ValueError, "must be an object"):
                    report_schema.validate_report(value, binding)
                with self.subTest(row=value), self.assertRaisesRegex(ValueError, "must be an object"):
                    report_schema.validate_report({**base, "metrics": [value]}, binding)
            for key, values in {
                "schema": ("wrong", None, [], 1), "producer": ("unknown", None, [], 1),
                "status": ("unknown", None, [], 1), "metrics": (None, {}, "", 1, True),
                "head_sha": ("", "a" * 39, "a" * 41, "A" * 40, "g" * 40, "a" * 40 + "\n", None, 1),
                "run_id": (True, False, 0, -1, 1.0, "1", None, 2**53),
                "pr_number": (True, False, 0, -1, 1.0, "1", None, 2**53),
            }.items():
                for value in values:
                    with self.subTest(producer=producer, field=key, value=value), self.assertRaises(ValueError):
                        report_schema.validate_report({**base, key: value}, {**binding, key: value})
            for key in ("run_id", "pr_number"):
                for value in (1, 2**53 - 1):
                    accepted = {**base, key: value}
                    self.assertEqual(accepted, report_schema.validate_report(accepted, {**binding, key: value}))
            for name in ("", "a" * 129, "-leading", ".leading", " name", "@team", "a|b", "a\nb",
                         "a\rb", "a\tb", "a\x00b", "a`b", "<html>", "[link](url)", "é"):
                row = {**base["metrics"][0], "name": name}
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, "safe plain text"):
                    report_schema.validate_report({**base, "metrics": [row]}, binding)
            for value in (None, [], {}, True, "unknown"):
                with self.subTest(verdict=value), self.assertRaisesRegex(ValueError, "invalid verdict"):
                    report_schema.validate_report({**base, "metrics": [{**base["metrics"][0], "verdict": value}]}, binding)
            for verdict in contract["verdicts"]:
                accepted = {**base, "metrics": [{**base["metrics"][0], "verdict": verdict}]}
                self.assertEqual(accepted, report_schema.validate_report(accepted, binding))
            for key, (lower, upper) in contract["numbers"].items():
                for value in (True, False, "", [], {}, math.nan, math.inf, -math.inf,
                              math.nextafter(float(lower), -math.inf), math.nextafter(float(upper), math.inf)):
                    row = {**base["metrics"][0], key: value}
                    with self.subTest(field=key, value=value), self.assertRaises(ValueError):
                        report_schema.validate_report({**base, "metrics": [row]}, binding)
                for value in (None, int(lower), lower, int(upper), upper):
                    accepted = {**base, "metrics": [{**base["metrics"][0], key: value}]}
                    self.assertEqual(accepted, report_schema.validate_report(accepted, binding))
            accepted = {**base, "metrics": [{**base["metrics"][0], "name": "a" * 128}] * 100}
            self.assertEqual(accepted, report_schema.validate_report(accepted, binding))
            with self.assertRaisesRegex(ValueError, "at most 100"):
                report_schema.validate_report({**accepted, "metrics": accepted["metrics"] + [accepted["metrics"][0]]}, binding)

    def test_wire_rejections_and_exact_byte_limit(self):
        payload = json.dumps(e2e_report()).encode()
        at_limit = payload + b" " * (1024 * 1024 - len(payload))
        self.assertEqual(e2e_report(), report_schema.parse_and_validate_report(at_limit, BINDING))
        with self.assertRaisesRegex(ValueError, "one MiB"):
            report_schema.parse_and_validate_report(at_limit + b" ", BINDING)
        for bad in (b"{", b"\xff", b"{} trailing", b"", b"["):
            with self.subTest(payload=bad), self.assertRaisesRegex(ValueError, "malformed JSON"):
                report_schema.parse_and_validate_report(bad, BINDING)
        for constant in ("NaN", "Infinity", "-Infinity", "1e309"):
            with self.subTest(constant=constant), self.assertRaisesRegex(ValueError, "finite"):
                report_schema.parse_and_validate_report(payload.replace(b"100.0", constant.encode()), BINDING)

    def test_micro_interval_null_truth_table(self):
        fields = ("mean_percent", "ci_low_percent", "ci_high_percent", "base_median_ns", "pr_median_ns")
        values = (4.2, 3.1, 5.3, 1000.0, 1042.0)
        # Exercise all 32 null/non-null combinations, including both partial intervals.
        for nulls in product((False, True), repeat=len(fields)):
            report = micro_report()
            report["metrics"][0].update({
                field: None if absent else value for field, value, absent in zip(fields, values, nulls)
            })
            rendered = report_schema.render_report(report_schema.validate_report(report, MICRO_BINDING))
            table_row = [line for line in rendered.splitlines() if line.startswith("|")][2]
            cells = [cell.strip() for cell in table_row.split("|")[1:-1]]
            expected = [
                "`fib/mono`", "regression",
                "n/a" if nulls[0] else "4.20%",
                "n/a" if nulls[1] or nulls[2] else "[3.10%, 5.30%]",
                "n/a" if nulls[3] else "1000.00ns",
                "n/a" if nulls[4] else "1042.00ns",
            ]
            with self.subTest(nulls=nulls):
                self.assertEqual(expected, cells)


if __name__ == "__main__":
    unittest.main()
