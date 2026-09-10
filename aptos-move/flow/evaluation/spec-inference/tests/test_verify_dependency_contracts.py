from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness.verify_dependency_contracts import (
    OpaqueContractTarget,
    _bottom_up_targets,
    verify_dependency_contracts,
)


class VerifyDependencyContractsTest(unittest.TestCase):
    def test_proves_original_package_without_removing_opaque(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            specification = package / "sources" / "example.spec.move"
            specification.parent.mkdir(parents=True)
            source = """spec 0x1::example {
    spec leaf {
        pragma opaque = true;
        aborts_if false;
    }
}
"""
            specification.write_text(source, encoding="utf-8")
            manifest = root / "manifest.json"
            manifest.write_text('{"records": []}', encoding="utf-8")
            move_flow = root / "move-flow"
            move_flow.write_bytes(b"test binary")
            output = root / "metadata" / "verification.json"
            prove_invocations: list[list[str]] = []

            def fake_run(arguments: list[str]) -> subprocess.CompletedProcess[str]:
                output_path = Path(arguments[arguments.index("--output") + 1])
                output_path.parent.mkdir(parents=True, exist_ok=True)
                if "inventory-package" in arguments:
                    payload = {"candidates": []}
                elif "contract-report" in arguments:
                    payload = {"functions": []}
                else:
                    payload = {"passed": True}
                output_path.write_text(json.dumps(payload), encoding="utf-8")
                if "prove" in arguments:
                    prove_invocations.append(arguments)
                return subprocess.CompletedProcess(arguments, 0, "", "")

            target = OpaqueContractTarget(
                name="address-1-example-leaf",
                target="0x1::example::leaf",
                specification="sources/example.spec.move",
                function="leaf",
            )
            with (
                mock.patch(
                    "harness.verify_dependency_contracts._run",
                    side_effect=fake_run,
                ),
                mock.patch(
                    "harness.verify_dependency_contracts.audit_dependency_contracts",
                    return_value={
                        "ready": True,
                        "contracts": [],
                        "status_counts": {},
                    },
                ),
                mock.patch(
                    "harness.verify_dependency_contracts._bottom_up_targets",
                    return_value=[target],
                ),
            ):
                result = verify_dependency_contracts(
                    package,
                    manifest,
                    move_flow,
                    output,
                    60,
                    Path(__file__).resolve().parent.parent
                    / "config"
                    / "corpus-selection.json",
                )

            self.assertEqual(source, specification.read_text(encoding="utf-8"))
            self.assertEqual(1, len(prove_invocations))
            invocation = prove_invocations[0]
            self.assertEqual(
                str(package.resolve()), invocation[invocation.index("--package") + 1]
            )
            proof = next(
                check
                for check in result["checks"]
                if check["kind"] == "opaque_contract_proof"
            )
            self.assertTrue(proof["opaque_preserved"])

    def test_orders_the_whole_dependency_closure_from_callee_to_caller(self) -> None:
        audit = {
            "ready": True,
            "contracts": [
                {
                    "function": "0x1::example::caller",
                    "status": "opaque_contract",
                    "specification_paths": ["sources/example.spec.move"],
                },
                {
                    "function": "0x1::example::leaf",
                    "status": "opaque_contract",
                    "specification_paths": ["sources/example.spec.move"],
                },
                {
                    "function": "0x1::vector::length",
                    "status": "intrinsic_binding",
                    "specification_paths": [],
                },
            ],
        }
        inventory = {
            "candidates": [
                {
                    "package_module_target": "0x1::example::caller",
                    "called_function_dependencies": [
                        "0x1::example::leaf",
                        "0x1::vector::length",
                    ],
                },
                {
                    "package_module_target": "0x1::example::leaf",
                    "called_function_dependencies": [],
                },
            ]
        }

        targets = _bottom_up_targets(audit, inventory)

        self.assertEqual(
            ["0x1::example::leaf", "0x1::example::caller"],
            [target.target for target in targets],
        )

    def test_rejects_audit_with_an_untrusted_contract(self) -> None:
        audit = {
            "ready": False,
            "contracts": [
                {
                    "function": "0x1::example::untrusted",
                    "status": "opaque_with_untrusted_inferred_conditions",
                    "specification_paths": ["sources/example.spec.move"],
                }
            ],
        }
        with self.assertRaisesRegex(ValueError, "untrusted"):
            _bottom_up_targets(audit, {"candidates": []})


if __name__ == "__main__":
    unittest.main()
