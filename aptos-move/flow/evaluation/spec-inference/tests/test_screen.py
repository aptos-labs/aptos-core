import json
import tempfile
import unittest
from pathlib import Path

from harness.screen import _load_ledger


class ScreeningLedgerTests(unittest.TestCase):
    def test_ignores_entries_from_a_different_tool_build(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "ledger.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "source_commit": "a" * 40,
                        "entries": [
                            {
                                "task_id": "task",
                                "threshold_seconds": 40,
                                "tool_executables": {
                                    "compile": {
                                        "path": "/old/move-flow",
                                        "sha256": "old",
                                    }
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            entries = _load_ledger(
                path,
                "a" * 40,
                40,
                {
                    "compile": {
                        "path": "/new/move-flow",
                        "sha256": "new",
                    }
                },
            )

        self.assertEqual(entries, {})


if __name__ == "__main__":
    unittest.main()
