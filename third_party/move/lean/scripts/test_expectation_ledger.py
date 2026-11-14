# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import importlib.util
import contextlib
import io
import json
import tempfile
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "expectation_ledger", Path(__file__).with_name("expectation-ledger.py"))
ledger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ledger)

A = "a" * 40
B = "b" * 40
C = "c" * 40


class ClassifyTest(unittest.TestCase):
    def test_untouched_fixture_is_same(self):
        self.assertEqual(ledger.classify(A, A, B, B), "same")

    def test_clean_fixture_without_baseline_is_same(self):
        self.assertEqual(ledger.classify(A, A, None, None), "same")

    def test_changed_baseline_is_reported(self):
        self.assertEqual(ledger.classify(A, A, B, C), "baseline-changed")

    def test_clean_fixture_that_started_failing_is_an_addition(self):
        self.assertEqual(ledger.classify(A, A, None, B), "baseline-added")

    def test_failing_fixture_that_became_clean_is_a_removal(self):
        self.assertEqual(ledger.classify(A, A, B, None), "baseline-removed")

    def test_new_and_removed_fixtures_win_over_their_baselines(self):
        self.assertEqual(ledger.classify(None, A, None, B), "fixture-new")
        self.assertEqual(ledger.classify(A, None, B, None), "fixture-removed")

    def test_source_change_under_an_equal_baseline(self):
        self.assertEqual(ledger.classify(A, B, C, C), "source-changed")

    def test_a_changed_baseline_outranks_its_changed_source(self):
        self.assertEqual(ledger.classify(A, B, A, C), "baseline-changed")

    def test_orphan_baseline_without_a_source_is_classified_by_it(self):
        self.assertEqual(ledger.classify(None, None, A, B), "baseline-changed")


class SourceStatusTest(unittest.TestCase):
    def test_statuses(self):
        self.assertEqual(ledger.source_status(A, A), "identical")
        self.assertEqual(ledger.source_status(A, B), "changed")
        self.assertEqual(ledger.source_status(None, A), "new")
        self.assertEqual(ledger.source_status(A, None), "removed")
        self.assertEqual(ledger.source_status(None, None), "none")


class ErrorLinesTest(unittest.TestCase):
    def test_counts_lines_not_occurrences(self):
        text = "error: one error here\nfine\nerror: two\n"
        self.assertEqual(ledger.error_lines(text), 2)

    def test_empty_baseline_has_no_errors(self):
        self.assertEqual(ledger.error_lines(""), 0)


class FixtureNameTest(unittest.TestCase):
    e2e = next(s for s in ledger.SCOPES if s["package"] == "verex-e2e-tests")
    prover = next(s for s in ledger.SCOPES if s["package"] == "move-prover-lean")

    def test_source_and_baseline_answer_the_same_name(self):
        base = "dir/Arithmetic"
        self.assertEqual(ledger.fixture_name(self.e2e, base + ".move"), base)
        self.assertEqual(ledger.fixture_name(self.e2e, base + ".exp"), base)

    def test_rust_specification_belongs_to_its_rust_fixture(self):
        self.assertEqual(ledger.fixture_name(self.e2e, "dir/arrays.spec.lean"),
                         "dir/arrays")
        self.assertEqual(ledger.fixture_name(self.e2e, "dir/arrays.rs"), "dir/arrays")

    def test_the_prover_baseline_is_lean_exp(self):
        self.assertEqual(ledger.fixture_name(self.prover, "s/aborts_if.lean_exp"),
                         "s/aborts_if")
        self.assertIsNone(ledger.fixture_name(self.prover, "s/aborts_if.exp"))


class ScopeTest(unittest.TestCase):
    def test_the_prover_trees_without_a_baseline_are_left_out(self):
        self.assertIsNone(ledger.scope_of(
            f"{ledger.PROVER}/move-stdlib/vector.move"))
        self.assertEqual(ledger.scope_of(f"{ledger.PROVER}/functional/enum.move")
                         ["package"], "move-prover-lean")

    def test_a_path_outside_every_scope_has_none(self):
        self.assertIsNone(ledger.scope_of("third_party/move/lean/README.md"))


class SourceFingerprintTest(unittest.TestCase):
    def test_the_baseline_is_not_part_of_the_source(self):
        files = {"d/f.rs": A, "d/f.spec.lean": B, "d/f.exp": C}
        with_other_baseline = {"d/f.rs": A, "d/f.spec.lean": B, "d/f.exp": A}
        self.assertEqual(ledger.source_fingerprint(files, "d/f.exp"),
                         ledger.source_fingerprint(with_other_baseline, "d/f.exp"))

    def test_a_changed_specification_changes_the_fingerprint(self):
        self.assertNotEqual(
            ledger.source_fingerprint({"d/f.rs": A, "d/f.spec.lean": B}, "d/f.exp"),
            ledger.source_fingerprint({"d/f.rs": A, "d/f.spec.lean": C}, "d/f.exp"))

    def test_a_fixture_without_sources_has_no_fingerprint(self):
        self.assertIsNone(ledger.source_fingerprint({"d/f.exp": A}, "d/f.exp"))


class IntroducedAtTest(unittest.TestCase):
    def states(self, *values):
        return {revision: value for (revision, _), value
                in zip(ledger.ATTRIBUTION, values)}

    def test_the_first_matching_revision_is_reported(self):
        now = (A, B)
        self.assertEqual(ledger.introduced_at(now, self.states((A, C), now, now)),
                         ledger.ATTRIBUTION[1][0])

    def test_content_unknown_to_every_revision_is_the_worktree(self):
        now = (A, B)
        self.assertEqual(
            ledger.introduced_at(now, self.states((A, C), (A, C), (A, C))),
            "worktree")


class AutoReasonTest(unittest.TestCase):
    rust = ledger.ATTRIBUTION[1][0]

    def entry(self, introduced, baseline, sources):
        return {"introduced_at": introduced, "baseline": baseline, "sources": sources}

    def test_a_rust_fixture_at_the_rust_revision_reads_as_rust(self):
        self.assertEqual(ledger.auto_reason(self.entry(
            self.rust, "d/SourceVerify/arrays.exp",
            ["d/SourceVerify/arrays.rs", "d/SourceVerify/arrays.spec.lean"])), "rust")

    def test_the_exporter_tree_reads_as_rust(self):
        self.assertEqual(ledger.auto_reason(self.entry(
            self.rust, "third_party/move/lean/verex-rust/rust-exporter/tests/a.exp.json",
            [])), "rust")

    def test_a_move_fixture_at_the_rust_revision_has_no_reason(self):
        self.assertEqual(ledger.auto_reason(self.entry(
            self.rust, "d/functional/enum.lean_exp", ["d/functional/enum.move"])), "")

    def test_a_rust_fixture_at_another_revision_has_no_reason(self):
        self.assertEqual(ledger.auto_reason(self.entry(
            ledger.ATTRIBUTION[0][0], "d/arrays.exp", ["d/arrays.rs"])), "")


class CheckTest(unittest.TestCase):
    def entries(self):
        return [
            {"package": "p", "fixture": "a", "class": "same",
             "baseline": "a.exp", "reason": ""},
            {"package": "p", "fixture": "b", "class": "baseline-changed",
             "baseline": "b.exp", "reason": ""},
        ]

    def check_against(self, recorded):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ledger.json"
            path.write_text(json.dumps({"entries": recorded}))
            with contextlib.redirect_stdout(io.StringIO()) as out:
                code = ledger.check(self.entries(), str(path))
            return code, out.getvalue()

    def test_a_difference_without_a_reason_fails(self):
        code, out = self.check_against(
            [{"package": "p", "fixture": "b", "reason": "   "}])
        self.assertEqual(code, 1)
        self.assertIn("b.exp", out)

    def test_a_difference_with_a_reason_passes(self):
        code, _ = self.check_against(
            [{"package": "p", "fixture": "b", "reason": "rust"}])
        self.assertEqual(code, 0)

    def test_an_unchanged_fixture_needs_no_reason(self):
        code, _ = self.check_against(
            [{"package": "p", "fixture": "b", "reason": "rust"},
             {"package": "p", "fixture": "a", "reason": ""}])
        self.assertEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
