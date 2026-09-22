from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.replacement import replace_task


class ReplacementTest(unittest.TestCase):
    def test_falls_back_deterministically_when_same_cell_has_no_reserve(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "manifest.json"
            manifest = {
                "corpus_status": "screened",
                "minimum_feature_counts": {},
                "records": [
                    {
                        "task_id": "selected",
                        "selection_status": "selected",
                        "source_root": "framework",
                        "granularity": "module",
                        "sampling_cell": "large:loop",
                        "module": "0x1::selected",
                        "feature_strata": ["large", "loop"],
                    },
                    {
                        "task_id": "later-rank",
                        "selection_status": "reserve",
                        "source_root": "framework",
                        "granularity": "module",
                        "sampling_cell": "large:global-state",
                        "module": "0x1::later",
                        "feature_strata": ["large", "global-state"],
                        "reserve_order": 2,
                    },
                    {
                        "task_id": "first-rank",
                        "selection_status": "reserve",
                        "source_root": "framework",
                        "granularity": "module",
                        "sampling_cell": "large:global-state",
                        "module": "0x1::first",
                        "feature_strata": ["large", "global-state"],
                        "reserve_order": 1,
                    },
                ],
            }
            path.write_text(json.dumps(manifest), encoding="utf-8")
            result = replace_task(path, "selected", "compatibility_timeout")
            history = result["replacement_history"][0]
            self.assertEqual("first-rank", history["replacement_task_id"])
            self.assertEqual("same_size_stratum", history["fallback_tier"])


if __name__ == "__main__":
    unittest.main()
