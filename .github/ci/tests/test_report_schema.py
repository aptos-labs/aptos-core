import copy
import json
import math
import string
import unittest
from itertools import product

from hypothesis import given, strategies as st

from ci_actions import report_schema
from tests.property_support import POSITIVE_ID, SHA as SHA_STRATEGY, configure_profiles
from tests.report_support import CONTRACTS, e2e_report, micro_report, trusted_binding

configure_profiles()

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


BINDING = trusted_binding(e2e_report())
MICRO_BINDING = trusted_binding(micro_report())


class ReportSchemaTables(unittest.TestCase):
    def test_renders_a_fixed_e2e_row(self):
        rendered = report_schema.render_report(report_schema.validate_report(e2e_report(), BINDING))
        self.assertIn("| `apt-fa-transfer` | ok | 100.00 | 125.00 | 1.25x | 2.00% |", rendered)

    def test_each_object_key_and_duplicate_is_checked(self):
        for report in (e2e_report(), micro_report()):
            binding = trusted_binding(report)
            row = report["metrics"][0]
            for key in report:
                with self.subTest(missing=key), self.assertRaisesRegex(ValueError, "missing key"):
                    report_schema.validate_report({name: value for name, value in report.items() if name != key}, binding)
            for key in row:
                altered = {**report, "metrics": [{name: value for name, value in row.items() if name != key}]}
                with self.subTest(missing=key), self.assertRaisesRegex(ValueError, "missing key"):
                    report_schema.validate_report(altered, binding)
            for altered in ({**report, "unexpected": None}, {**report, "metrics": [{**row, "unexpected": None}]}):
                with self.assertRaisesRegex(ValueError, "unknown key"):
                    report_schema.validate_report(altered, binding)
            payload = json.dumps(report)
            root_duplicate = payload[:-1] + ',"status":"failed"}'
            row_duplicate = payload.replace('"verdict": ', '"verdict": null, "verdict": ')
            for duplicate in (root_duplicate, row_duplicate):
                with self.subTest(payload=duplicate), self.assertRaisesRegex(ValueError, "duplicate key"):
                    report_schema.parse_and_validate_report(duplicate.encode(), binding)

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


if __name__ == "__main__":
    unittest.main()
