import json
import tempfile
import unittest
from pathlib import Path

from verify_hosted_shards import selected_tests, verify


def inventory(names):
    return {
        "rust-suites": {
            "binary": {
                "testcases": {
                    name: {"ignored": False, "filter-match": {"status": "matches"}}
                    for name in names
                }
            }
        }
    }


class VerifyShardsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.expected = self.root / "expected.json"
        self.expected.write_text(json.dumps(inventory(["a", "b", "c", "d"])))
        for shard, name in enumerate("abcd", 1):
            self.write_shard(shard, [name])

    def write_shard(self, shard, names):
        (self.root / f"inventory-{shard}.json").write_text(json.dumps(inventory(names)))
        (self.root / f"tests-{shard}.log").write_text(
            f"Summary [1.0s] {len(names)} tests run: {len(names)} passed (1 flaky), 4 skipped"
        )

    def test_complete_disjoint_results(self):
        self.assertEqual(verify(self.expected, self.root, 4), [1, 1, 1, 1])

    def test_duplicate_rejected(self):
        self.write_shard(4, ["a"])
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            verify(self.expected, self.root, 4)

    def test_missing_and_unexpected_rejected(self):
        self.write_shard(4, ["e"])
        with self.assertRaisesRegex(ValueError, "Coverage mismatch"):
            verify(self.expected, self.root, 4)

    def test_missing_artifact_rejected(self):
        (self.root / "inventory-4.json").unlink()
        with self.assertRaises(FileNotFoundError):
            verify(self.expected, self.root, 4)

    def test_failed_summary_rejected(self):
        (self.root / "tests-1.log").write_text("Summary [1s] 1 tests run: 0 passed, 1 failed")
        with self.assertRaisesRegex(ValueError, "Executed count"):
            verify(self.expected, self.root, 4)

    def test_no_summary_rejected(self):
        (self.root / "tests-1.log").write_text("process crashed")
        with self.assertRaisesRegex(ValueError, "summary"):
            verify(self.expected, self.root, 4)

    def test_ignored_and_filtered_excluded(self):
        data = inventory(["a", "b", "c"])
        data["rust-suites"]["binary"]["testcases"]["b"]["ignored"] = True
        data["rust-suites"]["binary"]["testcases"]["c"]["filter-match"]["status"] = "mismatch"
        self.assertEqual(selected_tests(data), {("binary", "a")})

    def test_empty_rejected(self):
        with self.assertRaisesRegex(ValueError, "No selected"):
            selected_tests(inventory([]))


if __name__ == "__main__":
    unittest.main()
