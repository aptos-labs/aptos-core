from __future__ import annotations

import unittest

from harness.catalog import _expected_selected_count


class CatalogTests(unittest.TestCase):
    def test_selection_count_comes_from_the_corpus_quotas(self) -> None:
        corpus = {
            "quotas": {
                "framework": {"function": 10, "module": 3},
                "experimental": {"function": 5, "module": 2},
            }
        }
        self.assertEqual(20, _expected_selected_count(corpus))


if __name__ == "__main__":
    unittest.main()
