from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.dependency_contracts import (
    _missing_modifies_resource_types,
    audit_dependency_contracts,
)
from harness.opaque_dependencies import _insert_opaque_pragma


class DependencyContractTests(unittest.TestCase):
    def test_concrete_modifies_covers_generic_inventory_effect(self) -> None:
        self.assertEqual(
            _missing_modifies_resource_types(
                ["0x1::coin::CoinInfo<#0>"],
                ["0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>"],
            ),
            [],
        )
        self.assertEqual(
            _missing_modifies_resource_types(
                ["0x1::coin::CoinInfo<0x1::other::OtherCoin>"],
                ["0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>"],
            ),
            ["0x1::coin::CoinInfo<0x1::other::OtherCoin>"],
        )

    def test_insert_opaque_pragma_preserves_other_blocks(self) -> None:
        source = """spec sample::m {
    spec target(x: u64) {
        use std::vector;
        ensures result == x;
    }
    spec other() { ensures true; }
}
"""
        updated, count = _insert_opaque_pragma(source, "target")
        self.assertEqual(count, 1)
        self.assertIn("pragma opaque = true;", updated)
        self.assertLess(updated.index("use std::vector"), updated.index("pragma opaque"))
        self.assertIn("spec other() { ensures true; }", updated)

    def test_audits_exact_dependency_union(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": [
                                    "0x1::m::opaque",
                                    "0x1::m::missing",
                                ],
                            },
                            {
                                "task_id": "two",
                                "granularity": "module",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target",
                                "target_functions": ["two"],
                                "called_function_dependencies": [
                                    "0x1::m::opaque",
                                    "0x1::target::two",
                                ],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::opaque",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                            },
                            {
                                "function": "0x1::m::missing",
                                "condition_count": 0,
                                "informative_condition_count": 0,
                                "condition_kinds": [],
                                "specification_paths": [],
                                "opaque": False,
                                "native": False,
                                "intrinsic": False,
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            result = audit_dependency_contracts(manifest, report, output)
            self.assertFalse(result["ready"])
            self.assertEqual(result["dependency_function_count"], 2)
            self.assertEqual(result["status_counts"]["opaque_contract"], 1)
            self.assertEqual(result["status_counts"]["missing_contract"], 1)
            opaque = result["contracts"][1]
            self.assertEqual(opaque["consumer_tasks"], ["one", "two"])

    def test_rejects_opaque_contract_with_only_ensures_true(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::trivial"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::trivial",
                                "condition_count": 1,
                                "informative_condition_count": 0,
                                "condition_kinds": ["Ensures"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_without_informative_conditions",
            )

    def test_rejects_opaque_contract_without_an_explicit_abort_clause(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::total"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::total",
                                "condition_count": 1,
                                "informative_condition_count": 1,
                                "condition_kinds": ["Ensures"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_without_abort_contract",
            )

    def test_rejects_global_mutator_without_modifies_clause(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            inventory = root / "inventory.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::mutate"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::mutate",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "has_modifies_clause": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            inventory.write_text(
                json.dumps(
                    {
                        "candidates": [
                            {
                                "package_module_target": "0x1::m::mutate",
                                "global_resource_mutation": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output, inventory)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_without_modifies_contract",
            )

    def test_allows_read_only_global_opaque_contract_without_modifies_clause(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            inventory = root / "inventory.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::read"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::read",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "has_modifies_clause": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            inventory.write_text(
                json.dumps(
                    {
                        "candidates": [
                            {
                                "package_module_target": "0x1::m::read",
                                "global_resource_mutation": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output, inventory)

            self.assertTrue(result["ready"])
            self.assertEqual(result["contracts"][0]["status"], "opaque_contract")

    def test_rejects_modifies_clause_missing_a_mutated_resource_type(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            inventory = root / "inventory.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::mutate"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::mutate",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "has_modifies_clause": True,
                                "modifies_resource_types": ["0x1::m::Other"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            inventory.write_text(
                json.dumps(
                    {
                        "candidates": [
                            {
                                "package_module_target": "0x1::m::mutate",
                                "global_resource_types": ["0x1::m::Resource"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output, inventory)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_with_incomplete_modifies_contract",
            )
            self.assertEqual(
                result["contracts"][0]["missing_modifies_resource_types"],
                ["0x1::m::Resource"],
            )

    def test_rejects_untrusted_wp_inferred_contract_clause(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::loop"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::loop",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "untrusted_inferred_condition_count": 2,
                                "untrusted_inferred_condition_kinds": [
                                    "Ensures",
                                    "AbortsIf",
                                ],
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_with_untrusted_inferred_conditions",
            )

    def test_rejects_partial_abort_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::partial"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::partial",
                                "condition_count": 1,
                                "informative_condition_count": 1,
                                "condition_kinds": ["AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "has_partial_aborts_if": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_with_partial_abort_contract",
            )

    def test_rejects_opaque_contract_with_verification_disabled(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::disabled"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::disabled",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "verification_disabled": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertFalse(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_with_verification_disabled",
            )

    def test_accepts_documented_trusted_verification_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            trusted = root / "trusted.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::trusted"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::trusted",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "verification_disabled": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            trusted.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "boundaries": [
                            {
                                "function": "0x1::m::trusted",
                                "basis": "proved_at_timeout",
                                "timeout_seconds": 180,
                                "artifact": "metadata/trusted.json",
                                "rationale": "Transparent proof completed at the larger limit.",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(
                manifest, report, output, None, trusted
            )

            self.assertTrue(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_trusted_assumption",
            )
            self.assertEqual(
                result["contracts"][0]["trusted_boundary"]["timeout_seconds"],
                180,
            )

    def test_classifies_intrinsic_model_boundary_separately(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            trusted = root / "trusted.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::map::helper"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::map::helper",
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["map.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                                "verification_disabled": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            trusted.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "boundaries": [
                            {
                                "function": "0x1::map::helper",
                                "basis": "intrinsic_model",
                                "rationale": "Private representation helper for an intrinsic map.",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(
                manifest, report, output, None, trusted
            )

            self.assertTrue(result["ready"])
            self.assertEqual(
                result["contracts"][0]["status"],
                "opaque_intrinsic_model_boundary",
            )

    def test_expands_the_contract_proof_closure_from_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            inventory = root / "inventory.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::boundary"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": function,
                                "condition_count": 2,
                                "informative_condition_count": 2,
                                "condition_kinds": ["Ensures", "AbortsIf"],
                                "specification_paths": ["m.spec.move"],
                                "opaque": True,
                                "native": False,
                                "intrinsic": False,
                            }
                            for function in ["0x1::m::boundary", "0x1::m::callee"]
                        ]
                    }
                ),
                encoding="utf-8",
            )
            inventory.write_text(
                json.dumps(
                    {
                        "candidates": [
                            {
                                "package_module_target": "0x1::m::boundary",
                                "called_function_dependencies": ["0x1::m::callee"],
                                "spec_function_dependencies": [],
                            },
                            {
                                "package_module_target": "0x1::m::callee",
                                "called_function_dependencies": [],
                                "spec_function_dependencies": [],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output, inventory)

            self.assertEqual(
                ["0x1::m::boundary", "0x1::m::callee"],
                [record["function"] for record in result["contracts"]],
            )

    def test_intrinsic_binding_needs_no_opaque_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            report = root / "report.json"
            output = root / "audit.json"
            manifest.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "task_id": "one",
                                "granularity": "function",
                                "module": "0x1::target",
                                "package_module_target": "0x1::target::one",
                                "target_functions": ["one"],
                                "called_function_dependencies": ["0x1::m::intrinsic"],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "functions": [
                            {
                                "function": "0x1::m::intrinsic",
                                "condition_count": 0,
                                "informative_condition_count": 0,
                                "condition_kinds": [],
                                "specification_paths": [],
                                "opaque": False,
                                "native": False,
                                "intrinsic": True,
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            result = audit_dependency_contracts(manifest, report, output)

            self.assertTrue(result["ready"])
            self.assertEqual(result["contracts"][0]["status"], "intrinsic_binding")


if __name__ == "__main__":
    unittest.main()
