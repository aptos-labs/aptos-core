from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "select_round", ROOT / "corpus-v3" / "select_round.py"
)
select_round = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(select_round)


class RoundSelectionTest(unittest.TestCase):
    """The recorded selection has to stay derivable from the corpus itself.

    Which tasks a round runs is a corpus decision, so it is made from the
    corpus's own description of each task and its target source -- never from
    an arm's behaviour. Re-deriving it here is what keeps the recorded
    selection auditable rather than merely asserted.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = json.loads(
            (ROOT / "corpus-v3" / "manifest.json").read_text(encoding="utf-8")
        )
        cls.recorded = json.loads(
            (ROOT / "corpus-v3" / "metadata" / "selection.json").read_text(
                encoding="utf-8"
            )
        )

    def test_the_recorded_selection_is_what_the_rule_produces(self) -> None:
        derived = select_round.select(
            self.manifest["records"],
            self.recorded["size"],
            self.recorded["max_guessable"],
        )
        self.assertEqual(self.recorded["selected"], derived["selected"])
        self.assertEqual(self.recorded["held_back"], derived["held_back"])

    def test_no_feature_stratum_is_lost(self) -> None:
        # Dropping the only carrier of a stratum removes a capability from the
        # benchmark, which no saving of sessions justifies.
        self.assertEqual([], self.recorded["strata_lost"])
        self.assertEqual(
            self.recorded["strata_total"], self.recorded["strata_covered"]
        )

    def test_every_unique_stratum_carrier_is_selected(self) -> None:
        self.assertEqual(
            [],
            sorted(
                set(self.recorded["unique_stratum_carriers"])
                - set(self.recorded["selected"])
            ),
        )

    def test_the_guessable_cap_holds(self) -> None:
        self.assertLessEqual(
            self.recorded["guessable"], self.recorded["max_guessable"]
        )

    def test_no_sample_is_removed_by_selecting(self) -> None:
        # Held-back tasks stay in the corpus for a later round.
        records = self.manifest["records"]
        self.assertEqual(25, len(records))
        labelled = {r["task_id"]: r["round_selection"] for r in records}
        self.assertEqual(
            set(self.recorded["selected"]),
            {t for t, v in labelled.items() if v == "selected"},
        )
        self.assertEqual(
            set(self.recorded["held_back"]),
            {t for t, v in labelled.items() if v == "held_back"},
        )
        for record in records:
            if record["screening_status"] != "ready":
                self.assertEqual("not_ready", record["round_selection"])

    def test_no_task_is_both_selected_and_held_back(self) -> None:
        # A task can belong to more than one redundancy cluster, so a winner in
        # one could be recorded as a loser in another. The two sets partition
        # the ready tasks; anything else makes `round_selection` disagree with
        # the recorded rationale.
        selected = set(self.recorded["selected"])
        held = set(self.recorded["held_back"])
        self.assertEqual(set(), selected & held)
        ready = {
            r["task_id"]
            for r in self.manifest["records"]
            if r["screening_status"] == "ready"
        }
        self.assertEqual(ready, selected | held)

    def test_the_manifest_holds_no_second_record_of_the_mutants(self) -> None:
        # The live set is `mutants/<task>/mutants.json`. A top-level block here
        # would be a second record of the same fact, and the stale one looked
        # authoritative for sitting in the manifest.
        self.assertNotIn("mutants", self.manifest)

    def test_a_near_duplicate_pair_is_never_both_selected(self) -> None:
        selected = set(self.recorded["selected"])
        for pair in self.recorded["near_duplicate_targets"]:
            with self.subTest(pair=pair["tasks"]):
                self.assertLess(
                    len(selected.intersection(pair["tasks"])),
                    2,
                    f"{pair['tasks']} are {pair['similarity']} similar",
                )


if __name__ == "__main__":
    unittest.main()
