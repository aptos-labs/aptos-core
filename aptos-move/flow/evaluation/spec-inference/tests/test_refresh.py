from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.refresh import _refresh_dependency_closures, _refresh_shared_catalog


class RefreshTests(unittest.TestCase):
    def test_dependency_closures_follow_edited_package_inventory(self) -> None:
        manifest = {
            "records": [
                {
                    "task_id": "F-1",
                    "granularity": "function",
                    "module": "0x1::m",
                    "package_module_target": "0x1::m::f",
                    "target_functions": ["f"],
                    "transitive_function_dependencies": ["stale"],
                    "called_function_dependencies": ["stale-boundary"],
                    "spec_function_dependencies": ["stale-spec"],
                    "transitive_called_function_dependencies": ["stale-called"],
                    "transitive_module_dependencies": ["stale"],
                },
                {
                    "task_id": "M-1",
                    "granularity": "module",
                    "module": "0x1::m",
                    "package_module_target": "0x1::m",
                    "target_functions": ["f", "g"],
                    "transitive_function_dependencies": [],
                    "called_function_dependencies": [],
                    "spec_function_dependencies": [],
                    "transitive_called_function_dependencies": [],
                    "transitive_module_dependencies": [],
                },
            ]
        }
        inventory = {
            "candidates": [
                {
                    "granularity": "function",
                    "package_module_target": "0x1::m::f",
                    "transitive_function_dependencies": ["0x1::d::a"],
                    "called_function_dependencies": ["0x1::d::boundary_a"],
                    "spec_function_dependencies": ["0x1::d::spec_a"],
                    "transitive_called_function_dependencies": ["0x1::d::called_a"],
                    "transitive_module_dependencies": ["0x1::d"],
                },
                {
                    "granularity": "function",
                    "package_module_target": "0x1::m::g",
                    "transitive_function_dependencies": ["0x1::d::b"],
                    "called_function_dependencies": ["0x1::d::boundary_b"],
                    "spec_function_dependencies": ["0x1::d::spec_b"],
                    "transitive_called_function_dependencies": ["0x1::d::called_b"],
                    "transitive_module_dependencies": ["0x1::d"],
                },
            ]
        }

        _refresh_dependency_closures(manifest, inventory)

        self.assertEqual(
            ["0x1::d::a"], manifest["records"][0]["transitive_function_dependencies"]
        )
        self.assertEqual(
            ["0x1::d::a", "0x1::d::b"],
            manifest["records"][1]["transitive_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::boundary_a"],
            manifest["records"][0]["called_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::boundary_a", "0x1::d::boundary_b"],
            manifest["records"][1]["called_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::spec_a"],
            manifest["records"][0]["spec_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::spec_a", "0x1::d::spec_b"],
            manifest["records"][1]["spec_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::called_a"],
            manifest["records"][0]["transitive_called_function_dependencies"],
        )
        self.assertEqual(
            ["0x1::d::called_a", "0x1::d::called_b"],
            manifest["records"][1]["transitive_called_function_dependencies"],
        )

    def test_generated_companion_spec_is_added_to_module_catalog(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            sources = package / "sources/MoveStdlib"
            sources.mkdir(parents=True)
            (sources / "vector.move").write_text(
                "module std::vector {}\n", encoding="utf-8"
            )
            (sources / "vector.spec.move").write_text(
                "spec std::vector {}\n", encoding="utf-8"
            )
            (package / "corpus-modules.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "kind": "shared_editable_module_union",
                        "modules": [
                            {
                                "module": "0x1::vector",
                                "implementation": "sources/MoveStdlib/vector.move",
                                "specifications": [],
                            }
                        ],
                        "source_packages": [],
                        "resolved_address_aliases": {"std": "0x1"},
                        "source_path_map": {
                            "move-stdlib/sources/vector.move": (
                                "sources/MoveStdlib/vector.move"
                            )
                        },
                    }
                ),
                encoding="utf-8",
            )

            metadata = _refresh_shared_catalog(package)
            catalog = json.loads(
                (package / "corpus-modules.json").read_text(encoding="utf-8")
            )

            self.assertEqual(
                ["sources/MoveStdlib/vector.spec.move"],
                catalog["modules"][0]["specifications"],
            )
            self.assertEqual(
                ["sources/MoveStdlib/vector.spec.move"],
                catalog["generated_specifications"],
            )
            self.assertEqual(
                {"module_count": 1, "move_file_count": 2}, metadata
            )


if __name__ == "__main__":
    unittest.main()
