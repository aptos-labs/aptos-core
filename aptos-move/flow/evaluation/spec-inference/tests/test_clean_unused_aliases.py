from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.clean_unused_aliases import clean_unused_aliases


def _warning(path: Path, line: int, column: int, alias: str) -> dict[str, object]:
    """One record as `flow experiment check-package` writes it."""
    return {
        "headline": "unused alias",
        "label": f"Unused 'use' of alias '{alias}'. Consider removing it",
        "file": str(path),
        "line": line,
        "column": column,
        "is_error": False,
    }


class CleanUnusedAliasesTest(unittest.TestCase):
    def _clean(self, source: str, warnings: list[tuple[int, int, str]]) -> str:
        """Run the cleaner over a one-module file and return the result."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            move = package / "sources/m.move"
            move.parent.mkdir(parents=True)
            move.write_text(source, encoding="utf-8")
            report = root / "compile.json"
            report.write_text(
                json.dumps(
                    {
                        "schema_version": 2,
                        "diagnostics": [],
                        "records": [
                            _warning(move, line, column, alias)
                            for line, column, alias in warnings
                        ],
                    }
                ),
                encoding="utf-8",
            )
            clean_unused_aliases(package, root, report, root / "cleanup.json")
            return move.read_text(encoding="utf-8")

    def test_removes_a_declaration_that_binds_one_alias(self) -> None:
        result = self._clean(
            "spec 0x1::m {\n    use 0x1::unused;\n    ensures true;\n}\n",
            [(2, 14, "unused")],
        )

        self.assertNotIn("unused", result)
        self.assertIn("ensures true", result)

    def test_an_unused_module_alias_leaves_the_members_it_shares_a_line_with(
        self,
    ) -> None:
        # `use a::dep::{Self, T};` binds both `dep` and `T`. The compiler
        # anchors the unused `dep` at the module name in the path, outside the
        # member group, so removing the line would delete a `T` still in use.
        result = self._clean(
            "module 0x1::m {\n    use 0x1::dep::{Self, T};\n\n"
            "    public fun f(t: T): T { t }\n}\n",
            [(2, 14, "dep")],
        )

        self.assertIn("use 0x1::dep::{T};", result)
        self.assertIn("public fun f(t: T): T { t }", result)

    def test_one_unused_member_leaves_the_rest_of_the_group(self) -> None:
        result = self._clean(
            "module 0x1::m {\n    use 0x1::dep::{Self, T, U};\n\n"
            "    public fun g(): T { dep::make(2) }\n}\n",
            [(2, 29, "U")],
        )

        self.assertIn("use 0x1::dep::{Self, T};", result)

    def test_a_group_whose_every_member_is_unused_loses_the_line(self) -> None:
        result = self._clean(
            "module 0x1::m {\n    use 0x1::dep::{Self, T};\n\n"
            "    public fun h(): u64 { 1 }\n}\n",
            [(2, 14, "dep"), (2, 26, "T")],
        )

        self.assertNotIn("use", result)
        self.assertIn("public fun h(): u64 { 1 }", result)

    def test_a_renamed_alias_loses_the_line(self) -> None:
        result = self._clean(
            "module 0x1::m {\n    use 0x1::dep::T as Alias;\n\n"
            "    public fun j(): u64 { 3 }\n}\n",
            [(2, 24, "Alias")],
        )

        self.assertNotIn("use", result)

    def test_a_non_use_line_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError, "refusing to remove non-use line"):
            self._clean(
                "module 0x1::m {\n    public fun k(): u64 { 4 }\n}\n",
                [(2, 5, "k")],
            )


if __name__ == "__main__":
    unittest.main()
