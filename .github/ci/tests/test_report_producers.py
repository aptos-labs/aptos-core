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


class ReportProducerManifestTests(unittest.TestCase):
    def test_rejects_malformed_manifests(self):
        without_label = {key: value for key, value in E2E.items() if key != "label"}
        cases = {
            "not JSON": "{",
            "duplicate JSON key": '{"producers": [], "producers": []}',
            "not an object": "[]",
            "unknown root key": json.dumps({"producers": [E2E], "version": 1}),
            "empty list": manifest(),
            "missing field": manifest(without_label),
            "unknown field": manifest({**E2E, "extra": "x"}),
            "empty string": manifest({**E2E, "job": ""}),
            "non-string": manifest({**E2E, "job": 1}),
            "duplicate producer key": manifest(E2E, E2E),
        }
        for name, text in cases.items():
            with self.subTest(name), self.assertRaises(ActionError):
                parse_report_producers(text)


class ReportProducerManifestProperties(unittest.TestCase):
    @example(entries=[E2E, {**E2E, "key": "other", "workflow_path": ".github/workflows/other.yaml"}])
    @given(entries=ENTRIES)
    def test_manifest_round_trip_order_and_bindings(self, entries):
        entries = [dict(entry) for entry in entries]
        parsed = parse_report_producers(manifest(*entries))
        reversed_keys = [{key: entry[key] for key in reversed(MANIFEST_FIELDS)} for entry in entries]
        self.assertEqual(parsed, parse_report_producers(manifest(*reversed_keys)))
        self.assertEqual([entry["key"] for entry in entries], list(parsed))
        for entry in entries:
            actual = parsed[entry["key"]]
            self.assertEqual(entry, {key: getattr(actual, key) for key in MANIFEST_FIELDS})
            self.assertEqual((entry["key"], entry["workflow_path"], entry["workflow_name"]),
                             (actual.binding().key, actual.binding().workflow_path, actual.binding().workflow_name))

    @given(entry=ENTRY)
    def test_every_entry_field_and_duplicate_is_validated(self, entry):
        for field in MANIFEST_FIELDS:
            without = {key: value for key, value in entry.items() if key != field}
            with self.assertRaisesRegex(ActionError, "exactly the fields"):
                parse_report_producers(manifest(without))
            for bad in ("", None, True, 1, [], {}):
                with self.assertRaisesRegex(ActionError, "non-empty strings"):
                    parse_report_producers(manifest({**entry, field: bad}))
            duplicate = json.dumps(entry)[:-1] + "," + json.dumps(field) + ":" + json.dumps(entry[field]) + "}"
            with self.assertRaisesRegex(ActionError, "duplicate JSON key"):
                parse_report_producers('{"producers":[' + duplicate + "]}")
        with self.assertRaisesRegex(ActionError, "exactly the fields"):
            parse_report_producers(manifest({**entry, "extra": "x"}))
        with self.assertRaisesRegex(ActionError, "key is duplicated"):
            parse_report_producers(manifest(entry, {**entry, "job": "different"}))

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
    def test_schema_row_contracts_cover_exactly_the_manifest_producers(self):
        self.assertEqual(set(load_report_producers()), set(report_schema.PRODUCER_ROWS))

    def test_renderers_cover_exactly_the_manifest_producers(self):
        self.assertEqual(set(load_report_producers()), set(report_schema.RENDERERS))
