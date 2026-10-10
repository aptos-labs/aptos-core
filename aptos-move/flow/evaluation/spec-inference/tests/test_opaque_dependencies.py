from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.opaque_dependencies import (
    _append_opaque_stub,
    make_dependency_contracts_opaque,
)


class OpaqueDependenciesTests(unittest.TestCase):
    def test_appends_stub_inside_named_address_module_spec(self) -> None:
        source = """spec aptos_framework::sample {
    spec schema Applied { ensures true; }
    apply Applied to target;
}
"""

        updated, count = _append_opaque_stub(source, "sample", "target")

        self.assertEqual(1, count)
        self.assertIn("spec target {\n        pragma opaque = true;", updated)
        self.assertTrue(updated.rstrip().endswith("}"))

    def test_refuses_ambiguous_module_blocks(self) -> None:
        source = """spec std::sample {}
spec other::sample {}
"""

        updated, count = _append_opaque_stub(source, "sample", "target")

        self.assertEqual(0, count)
        self.assertEqual(source, updated)

    def test_appends_stub_inside_implementation_module(self) -> None:
        source = """module std::sample {
    spec schema Applied { ensures true; }
    apply Applied to target;
}
"""

        updated, count = _append_opaque_stub(source, "sample", "target")

        self.assertEqual(1, count)
        self.assertIn("spec target {\n        pragma opaque = true;", updated)

    def test_same_named_function_is_scoped_to_own_module(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            sources = package / "sources"
            sources.mkdir(parents=True)
            first = sources / "first.spec.move"
            second = sources / "second.spec.move"
            first.write_text(
                "spec std::first { spec shared { ensures true; } }\n",
                encoding="utf-8",
            )
            second.write_text(
                "spec std::second { spec shared { ensures true; } }\n",
                encoding="utf-8",
            )
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "modules": [
                            {
                                "module": "0x1::first",
                                "implementation": "sources/first.move",
                                "specifications": ["sources/first.spec.move"],
                            },
                            {
                                "module": "0x1::second",
                                "implementation": "sources/second.move",
                                "specifications": ["sources/second.spec.move"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            audit = root / "audit.json"
            audit.write_text(
                json.dumps(
                    {
                        "contracts": [
                            {
                                "function": "0x1::second::shared",
                                "module": "0x1::second",
                                "status": "nonopaque_contract",
                                "specification_paths": [
                                    "sources/first.spec.move",
                                    "sources/second.spec.move",
                                ],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            report = make_dependency_contracts_opaque(
                package, audit, root / "report.json"
            )

            self.assertEqual(1, report["changed_function_count"])
            self.assertNotIn("pragma opaque", first.read_text(encoding="utf-8"))
            self.assertIn("pragma opaque", second.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
