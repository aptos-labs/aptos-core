import asyncio
import json
from pathlib import Path
import tempfile
import unittest

from harness.materialize import preparation_patch
from harness.screen_v3 import screen_corpus_v3

ROOT = Path(__file__).resolve().parent.parent


class PreparationPatchTest(unittest.TestCase):
    def test_a_patch_inside_the_corpus_resolves(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            corpus = Path(directory)
            self.assertEqual(
                preparation_patch(corpus, {"preparation_patch": "patches/T.patch"}),
                (corpus / "patches/T.patch").resolve(),
            )

    def test_a_patch_outside_the_corpus_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            corpus = Path(directory) / "corpus"
            for relative in ("../outside.patch", "patches/../../outside.patch", "/etc/passwd"):
                with self.subTest(relative=relative):
                    with self.assertRaisesRegex(ValueError, "escapes"):
                        preparation_patch(corpus, {"preparation_patch": relative})

    def test_screening_refuses_a_patch_outside_the_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            corpus = root / "corpus"
            (corpus / "package").mkdir(parents=True)
            (corpus / "package" / "Move.toml").write_text("[package]\nname = \"p\"\n")
            # The patch exists, so only the confinement can refuse it.
            (root / "outside.patch").write_text("")
            (corpus / "manifest.json").write_text(json.dumps({"records": [{
                "task_id": "T-x-001", "target": "0x1::m::f", "screening_status": "ready",
                "round_selection": "selected", "preparation_patch": "../outside.patch",
            }]}))
            (root / "corpus.json").write_text(json.dumps({"compatibility_threshold_seconds": 20}))
            with self.assertRaisesRegex(ValueError, "escapes"):
                asyncio.run(screen_corpus_v3(
                    corpus / "manifest.json", ROOT / "config/default.json", root / "corpus.json",
                    root / "results", root / "summary.json",
                ))


if __name__ == "__main__":
    unittest.main()
