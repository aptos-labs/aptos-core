from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness.score_round import _score_pending


class UnscorableRunTest(unittest.TestCase):
    """One cell that cannot be scored must not take the round with it.

    Scoring runs after the round, so the alternative to recording a failure is
    discarding sessions that already cost their full budget -- an apparatus
    failure reported as an absence of results.
    """

    def _run(self, error: BaseException) -> list[dict]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entries = [{"run_id": "bad"}, {"run_id": "good"}]
            pending = [
                (entries[0], root / "bad", root / "base", "0x1::m::f", root / "m.json", 10),
                (entries[1], root / "good", root / "base", "0x1::m::f", root / "m.json", 10),
            ]

            async def score(config, candidate, baseline, target, manifest, timeout):
                if "bad" in str(candidate):
                    raise error
                return {"mutation_adequacy": 1.0, "killed": 3, "essential_mutants": 3}

            with mock.patch("harness.score_round.score_mutants", side_effect=score), \
                 mock.patch("harness.score_round.write_json"):
                asyncio.run(_score_pending(mock.MagicMock(), pending, 1))
            return entries

    def test_an_unreadable_workspace_is_recorded_not_raised(self) -> None:
        for error in (
            ValueError("proof does not reproduce"),
            FileNotFoundError("workspace is gone"),
            OSError("input/output error"),
        ):
            with self.subTest(error=type(error).__name__):
                bad, good = self._run(error)
                self.assertEqual("not_scorable", bad["outcome"])
                self.assertIn(type(error).__name__, bad["detail"])
                # The point of the fix: the other cell still has its score.
                self.assertEqual("scored", good["outcome"])
                self.assertTrue(good["strict_success"])


if __name__ == "__main__":
    unittest.main()
