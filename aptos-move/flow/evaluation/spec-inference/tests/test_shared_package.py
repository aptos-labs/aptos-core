from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness.shared_package import normalize_standalone_specs


class SharedPackageTests(unittest.TestCase):
    def test_qualifies_file_local_aliases_idempotently(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            spec = (
                package
                / "sources/AptosFramework/dispatchable_fungible_asset.spec.move"
            )
            spec.parent.mkdir(parents=True)
            spec.write_text(
                "ensures object::object_address(x) == @0x1;\n", encoding="utf-8"
            )

            first = normalize_standalone_specs(package)
            second = normalize_standalone_specs(package)

            self.assertEqual(1, len(first))
            self.assertEqual([], second)
            self.assertEqual(
                "ensures aptos_framework::object::object_address(x) == @0x1;\n",
                spec.read_text(encoding="utf-8"),
            )


if __name__ == "__main__":
    unittest.main()
