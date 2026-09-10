import json
import tempfile
import unittest
from pathlib import Path

from harness.infer_dependency_specs import (
    _complete_empty_dependency_boundaries,
    _is_complete_opaque_contract,
)


class InferDependencySpecsTest(unittest.TestCase):
    def test_complete_opaque_contract(self) -> None:
        self.assertTrue(
            _is_complete_opaque_contract({"opaque": True, "condition_count": 2})
        )

    def test_rejects_missing_nonopaque_and_empty_contracts(self) -> None:
        self.assertFalse(_is_complete_opaque_contract(None))
        self.assertFalse(
            _is_complete_opaque_contract({"opaque": False, "condition_count": 2})
        )
        self.assertFalse(
            _is_complete_opaque_contract({"opaque": True, "condition_count": 0})
        )

    def test_completes_only_empty_partial_opaque_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            source = package / "sources" / "m.move"
            spec = package / "sources" / "m.spec.move"
            source.parent.mkdir(parents=True)
            source.write_text("module 0x42::m { fun f() {} }\n", encoding="utf-8")
            spec.write_text(
                "spec 0x42::m {\n"
                "    spec f {\n"
                "        pragma opaque = true, aborts_if_is_partial = true;\n"
                "    }\n"
                "}\n",
                encoding="utf-8",
            )
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "modules": [
                            {
                                "module": "0x42::m",
                                "implementation": "sources/m.move",
                                "specifications": ["sources/m.spec.move"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            completed = _complete_empty_dependency_boundaries(
                package, ["0x42::m::f"]
            )

            self.assertEqual(
                [{"function": "0x42::m::f", "path": "sources/m.spec.move"}],
                completed,
            )
            self.assertIn("ensures true;", spec.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
