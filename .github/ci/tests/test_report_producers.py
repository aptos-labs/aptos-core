import json
import unittest

from ci_actions import report_schema
from ci_actions.github import ActionError
from ci_actions.report_producers import ReportProducer, load_report_producers, parse_report_producers
from ci_actions.run_binding import Producer

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
    def test_parses_producers_in_order_and_builds_run_bindings(self):
        other = {**E2E, "key": "other", "workflow_path": ".github/workflows/other.yaml"}
        producers = parse_report_producers(manifest(E2E, other))
        self.assertEqual(["mono-move-e2e-perf", "other"], list(producers))
        self.assertEqual(ReportProducer(**E2E), producers["mono-move-e2e-perf"])
        self.assertEqual(
            Producer("mono-move-e2e-perf", ".github/workflows/mono-move-e2e-perf.yaml", "mono-move-e2e-perf"),
            producers["mono-move-e2e-perf"].binding(),
        )

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


class CheckedInManifestTests(unittest.TestCase):
    def test_schema_row_contracts_cover_exactly_the_manifest_producers(self):
        self.assertEqual(set(load_report_producers()), set(report_schema.PRODUCER_ROWS))

    def test_renderers_cover_exactly_the_manifest_producers(self):
        self.assertEqual(set(load_report_producers()), set(report_schema.RENDERERS))
