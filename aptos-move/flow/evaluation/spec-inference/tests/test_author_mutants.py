from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "author_mutants", ROOT / "corpus-v3.2" / "author_mutants.py"
)
author_mutants = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(author_mutants)


def load_copy(corpus: str):
    spec = importlib.util.spec_from_file_location(
        f"author_mutants_{corpus.replace('-', '_').replace('.', '_')}",
        ROOT / corpus / "author_mutants.py",
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

SOURCE = """module m {
    fun f(a: u64): u64 {
        let total = 0;
        total += a;
        total
    }
}
"""


class BuildCaseTest(unittest.TestCase):
    """An anchored mutant is only as trustworthy as its offset.

    The corpus stores an offset, a length and a digest instead of the source
    text, so `harness.mutants` refuses a mutant whose anchored fragment has
    moved. That check protects a *recorded* mutant; it cannot rescue one whose
    offset was ambiguous when it was authored. These cases cover that gap.
    """

    def setUp(self) -> None:
        self.package = Path(tempfile.mkdtemp())
        (self.package / "sources").mkdir()
        (self.package / "sources" / "m.move").write_text(SOURCE, encoding="utf-8")

    def entry(self, **overrides: object) -> dict[str, object]:
        entry = {
            "mutant_id": "T-001-example",
            "file": "sources/m.move",
            "anchor": "let total = 0;",
            "replace": "0",
            "with": "1",
            "obligation_category": "normal-result",
            "rationale": "off by one",
        }
        entry.update(overrides)
        return entry

    def test_offset_and_digest_locate_the_anchor(self) -> None:
        case = author_mutants.build_case(self.package, self.entry())
        anchor = case["anchor"]
        fragment = SOURCE[anchor["offset"]:anchor["offset"] + anchor["length"]]
        self.assertEqual(fragment, "let total = 0;")
        # The edit offset is relative to the anchor, not the file.
        self.assertEqual(fragment[case["edit"]["at"]], "0")

    def test_edit_reproduces_the_intended_text(self) -> None:
        case = author_mutants.build_case(self.package, self.entry())
        anchor = case["anchor"]
        fragment = SOURCE[anchor["offset"]:anchor["offset"] + anchor["length"]]
        edit = case["edit"]
        mutated = (
            fragment[: edit["at"]] + edit["to"] + fragment[edit["at"] + edit["length"]:]
        )
        self.assertEqual(mutated, "let total = 1;")

    def test_ambiguous_anchor_is_refused(self) -> None:
        # `total` occurs three times, so an offset recorded for it is a guess
        # about which occurrence the author meant.
        with self.assertRaises(SystemExit) as raised:
            author_mutants.build_case(self.package, self.entry(anchor="total"))
        self.assertIn("occurs 3 times", str(raised.exception))

    def test_absent_anchor_is_refused(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            author_mutants.build_case(self.package, self.entry(anchor="no such text"))
        self.assertIn("occurs 0 times", str(raised.exception))

    def test_ambiguous_replacement_within_anchor_is_refused(self) -> None:
        # "t" appears repeatedly inside the anchor, so `at` would be arbitrary.
        with self.assertRaises(SystemExit) as raised:
            author_mutants.build_case(self.package, self.entry(replace="t"))
        self.assertIn("inside its own anchor", str(raised.exception))

    def test_case_records_its_review_and_category(self) -> None:
        case = author_mutants.build_case(self.package, self.entry())
        self.assertTrue(case["essential"])
        self.assertEqual(case["obligation_category"], "normal-result")
        self.assertTrue(case["reviews"][0]["approved"])
        self.assertIn("held-out scoring set", case["reviews"][0]["basis"])


class ScoringSetTest(unittest.TestCase):
    """The committed scoring set must stay disjoint from the refutation set.

    Refutation shows surviving mutants to the agent, so the refutation set is
    training material. A scoring mutant that repeats one of them scores the
    arm on what it was already told.
    """

    def test_committed_sets_are_disjoint(self) -> None:
        refutation = ROOT / "corpus-v3.2" / "mutants"
        scoring = ROOT / "corpus-v3.2" / "mutants-scoring"
        if not scoring.is_dir():
            self.skipTest("no scoring set is committed yet")

        package = ROOT / "corpus-v3.2" / "package"
        if not (package / "sources").is_dir():
            self.skipTest(
                "private Etna-derived sources are not materialized; "
                "run corpus-v3.2/build.py"
            )

        def identities(root: Path, task: str) -> set[str]:
            # The relation the controller actually enforces with, so a
            # divergence between authoring and enforcement fails here.
            from harness.mutants import mutation_fingerprint

            path = root / task / "mutants.json"
            cases = json.loads(path.read_text(encoding="utf-8"))["mutants"]
            return {mutation_fingerprint(c, package) for c in cases}

        for task_dir in sorted(scoring.iterdir()):
            if not task_dir.is_dir():
                continue
            task = task_dir.name
            overlap = identities(scoring, task) & identities(refutation, task)
            self.assertEqual(
                overlap, set(), f"{task}: scoring repeats a refutation mutant"
            )

    def test_every_scoring_mutant_ids_are_unique_across_tasks(self) -> None:
        scoring = ROOT / "corpus-v3.2" / "mutants-scoring"
        if not scoring.is_dir():
            self.skipTest("no scoring set is committed yet")
        seen: dict[str, str] = {}
        for task_dir in sorted(scoring.iterdir()):
            if not task_dir.is_dir():
                continue
            cases = json.loads(
                (task_dir / "mutants.json").read_text(encoding="utf-8")
            )["mutants"]
            for case in cases:
                mutant_id = case["mutant_id"]
                self.assertNotIn(
                    mutant_id,
                    seen,
                    f"{mutant_id} appears in both {seen.get(mutant_id)} and {task_dir.name}",
                )
                seen[mutant_id] = task_dir.name



class TaskIdConfinementTest(unittest.TestCase):
    """A spec's task id is checked before it reaches any path.

    The disjointness check reads `--disjoint-from/TASK/mutants.json` before the
    output is written, so a climbing task id must be refused there too. The
    planted manifest outside the root holds the same mutant, so reading it
    would end in a duplicate error rather than the task-id error.
    """

    def test_climbing_task_id_is_refused_before_reading(self) -> None:
        for corpus in ("corpus-v3.2", "corpus-v4"):
            with self.subTest(corpus=corpus), tempfile.TemporaryDirectory() as directory:
                module = load_copy(corpus)
                root = Path(directory)
                package = root / "package"
                (package / "sources").mkdir(parents=True)
                (package / "sources" / "m.move").write_text(SOURCE, encoding="utf-8")
                entry = {
                    "mutant_id": "T-1-a", "file": "sources/m.move", "anchor": "total += a;",
                    "replace": "+=", "with": "-=", "obligation_category": "normal-result",
                    "rationale": "r",
                }
                outside = root / "escape"
                outside.mkdir()
                planted = module.build_case(package, entry)
                (outside / "mutants.json").write_text(
                    json.dumps({"task_id": "x", "mutants": [planted]}), encoding="utf-8"
                )
                spec = root / "spec.json"
                spec.write_text(json.dumps({"../escape": [entry]}), encoding="utf-8")
                (root / "refutation").mkdir()
                argv = [
                    "author_mutants.py", "--spec", str(spec), "--package", str(package),
                    "--out", str(root / "out"), "--disjoint-from", str(root / "refutation"),
                ]
                with patch("sys.argv", argv):
                    with self.assertRaisesRegex(ValueError, "task_id"):
                        module.main()


if __name__ == "__main__":
    unittest.main()
