from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from harness.restore_reference_contracts import (
    _function_spec_spans,
    restore_reference_contracts,
)


class RestoreReferenceContractsTests(unittest.TestCase):
    def test_finds_only_requested_function_block_with_nested_expression(self) -> None:
        source = """spec 0x1::sample {
    spec existing(x: u64) {
        ensures { let y = x; result == y };
    }

    spec generated() {
        pragma opaque = true;
        aborts_if false;
    }
}
"""

        spans = _function_spec_spans(source, "existing")

        self.assertEqual(1, len(spans))
        block = source[slice(*spans[0])]
        self.assertIn("spec existing", block)
        self.assertNotIn("spec generated", block)

    @patch("harness.restore_reference_contracts._git_show")
    def test_restores_blocks_in_reverse_source_order(self, git_show) -> None:
        original = """spec 0x1::sample {
    spec alpha() {
        aborts_if false;
    }

    spec zeta() {
        ensures result;
    }
}
"""
        generated = """spec 0x1::sample {
    spec alpha() {
        aborts_if false;
        ensures [inferred] result;
    }

    spec generated() {
        pragma opaque = true;
        aborts_if false;
    }

    spec zeta() {
        ensures result;
        aborts_if [inferred] false;
    }
}
"""
        git_show.return_value = original
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            source = package / "sources/sample.spec.move"
            source.parent.mkdir(parents=True)
            source.write_text(generated, encoding="utf-8")
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "source_path_map": {
                            "upstream/sample.spec.move": "sources/sample.spec.move"
                        },
                        "modules": [
                            {
                                "module": "0x1::sample",
                                "implementation": "sources/sample.move",
                                "specifications": ["sources/sample.spec.move"],
                            }
                        ],
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
                                "function": f"0x1::sample::{name}",
                                "module": "0x1::sample",
                                "condition_count": 1,
                                "specification_paths": ["sources/sample.spec.move"],
                            }
                            for name in ("alpha", "zeta")
                        ]
                    }
                ),
                encoding="utf-8",
            )

            report = restore_reference_contracts(
                package, root, "0" * 40, audit, root / "report.json"
            )

            restored = source.read_text(encoding="utf-8")
            self.assertEqual(2, report["restored_function_count"])
            self.assertNotIn("[inferred]", restored)
            self.assertIn("spec generated", restored)
            self.assertEqual(3, restored.count("pragma opaque = true;"))

    @patch("harness.restore_reference_contracts._git_show")
    def test_schema_applied_contract_is_non_textual_not_unresolved(self, git_show) -> None:
        source_text = """spec 0x1::sample {
    spec schema S { ensures true; }
    apply S to target;
}
"""
        git_show.return_value = source_text
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            source = package / "sources/sample.spec.move"
            source.parent.mkdir(parents=True)
            source.write_text(source_text, encoding="utf-8")
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "source_path_map": {
                            "upstream/sample.spec.move": "sources/sample.spec.move"
                        },
                        "modules": [
                            {
                                "module": "0x1::sample",
                                "implementation": "sources/sample.move",
                                "specifications": ["sources/sample.spec.move"],
                            }
                        ],
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
                                "function": "0x1::sample::target",
                                "module": "0x1::sample",
                                "condition_count": 1,
                                "specification_paths": ["sources/sample.spec.move"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            report = restore_reference_contracts(
                package, root, "0" * 40, audit, root / "report.json"
            )

            self.assertEqual([], report["unresolved"])
            self.assertEqual(1, report["non_textual_function_count"])

    @patch("harness.restore_reference_contracts._git_show")
    def test_schema_applied_contract_restores_opaque_stub(self, git_show) -> None:
        original = """spec 0x1::sample {
    spec schema S { ensures true; }
    apply S to target;
}
"""
        generated = """spec 0x1::sample {
    spec schema S { ensures true; }
    apply S to target;

    spec target {
        pragma opaque = true;
        ensures [inferred] false;
    }
}
"""
        git_show.return_value = original
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            source = package / "sources/sample.spec.move"
            source.parent.mkdir(parents=True)
            source.write_text(generated, encoding="utf-8")
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "source_path_map": {
                            "upstream/sample.spec.move": "sources/sample.spec.move"
                        },
                        "modules": [
                            {
                                "module": "0x1::sample",
                                "implementation": "sources/sample.move",
                                "specifications": ["sources/sample.spec.move"],
                            }
                        ],
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
                                "function": "0x1::sample::target",
                                "module": "0x1::sample",
                                "condition_count": 1,
                                "specification_paths": ["sources/sample.spec.move"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            report = restore_reference_contracts(
                package, root, "0" * 40, audit, root / "report.json"
            )

            restored = source.read_text(encoding="utf-8")
            self.assertEqual([], report["unresolved"])
            self.assertNotIn("[inferred]", restored)
            self.assertIn("spec target {\n        pragma opaque = true;", restored)


if __name__ == "__main__":
    unittest.main()
