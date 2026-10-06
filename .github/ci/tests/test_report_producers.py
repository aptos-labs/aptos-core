import json
import unittest

from hypothesis import example, given, strategies as st

from ci_actions import report_schema
from ci_actions.github import ActionError
from ci_actions.report_producers import load_report_producers, parse_report_producers
from tests.property_support import configure_profiles

configure_profiles()

MANIFEST_FIELDS = ("key", "workflow_path", "workflow_name", "job", "benchmark_step", "label")
ENTRY = st.fixed_dictionaries({key: st.text(min_size=1, max_size=32) for key in MANIFEST_FIELDS})
ENTRIES = st.lists(ENTRY, min_size=1, max_size=8, unique_by=lambda entry: entry["key"])

E2E = {
    "key": "mono-move-e2e-perf",
    "workflow_path": ".github/workflows/mono-move-e2e-perf.yaml",
    "workflow_name": "mono-move-e2e-perf",
    "job": "mono-move-e2e-perf",
    "benchmark_step": "Run mono-move e2e performance comparison",
    "label": "mono-move-e2e-perf",
}


def manifest(*producers):
    return json.dumps({"producers": list(producers)})


class ReportProducerManifestProperties(unittest.TestCase):
    @example(entries=[E2E, {**E2E, "key": "other", "workflow_path": ".github/workflows/other.yaml"}])
    @given(entries=ENTRIES)
    def test_manifest_round_trip_order_and_bindings(self, entries):
        entries = [dict(entry) for entry in entries]
        parsed = parse_report_producers(manifest(*entries))
        self.assertEqual([entry["key"] for entry in entries], list(parsed))
        for entry in entries:
            actual = parsed[entry["key"]]
            self.assertEqual(entry, {key: getattr(actual, key) for key in MANIFEST_FIELDS})
            self.assertEqual((entry["key"], entry["workflow_path"], entry["workflow_name"]),
                             (actual.binding().key, actual.binding().workflow_path, actual.binding().workflow_name))

    def test_every_entry_field_and_duplicate_is_validated(self):
        for field in MANIFEST_FIELDS:
            without = {key: value for key, value in E2E.items() if key != field}
            with self.assertRaisesRegex(ActionError, "exactly the fields"):
                parse_report_producers(manifest(without))
            for bad in ("", None, True, 1, [], {}):
                with self.assertRaisesRegex(ActionError, "non-empty strings"):
                    parse_report_producers(manifest({**E2E, field: bad}))
            duplicate = json.dumps(E2E)[:-1] + "," + json.dumps(field) + ":" + json.dumps(E2E[field]) + "}"
            with self.assertRaisesRegex(ActionError, "duplicate JSON key"):
                parse_report_producers('{"producers":[' + duplicate + "]}")
        with self.assertRaisesRegex(ActionError, "exactly the fields"):
            parse_report_producers(manifest({**E2E, "extra": "x"}))
        with self.assertRaisesRegex(ActionError, "key is duplicated"):
            parse_report_producers(manifest(E2E, {**E2E, "job": "different"}))

    def test_every_manifest_root_and_list_rejection_category(self):
        for text in ("{", "", "{} trailing"):
            with self.subTest(malformed=text), self.assertRaisesRegex(ActionError, "valid JSON"):
                parse_report_producers(text)
        for root in (None, [], "", 1, True, {}, {"producers": [E2E], "extra": 1}):
            with self.subTest(root=root), self.assertRaisesRegex(ActionError, "only a producers key"):
                parse_report_producers(json.dumps(root))
        for entries in (None, {}, "", 1, True, []):
            with self.subTest(entries=entries), self.assertRaisesRegex(ActionError, "non-empty list"):
                parse_report_producers(json.dumps({"producers": entries}))
        for entry in (None, [], "", 1, True):
            with self.subTest(entry=entry), self.assertRaisesRegex(ActionError, "exactly the fields"):
                parse_report_producers(manifest(entry))
        with self.assertRaisesRegex(ActionError, "duplicate JSON key"):
            parse_report_producers('{"producers":[],"producers":[]}')


class CheckedInManifestTests(unittest.TestCase):
    def test_schema_rows_and_renderers_cover_exactly_the_manifest_producers(self):
        self.assertEqual(set(load_report_producers()), set(report_schema.PRODUCER_ROWS))
        self.assertEqual(set(load_report_producers()), set(report_schema.RENDERERS))
