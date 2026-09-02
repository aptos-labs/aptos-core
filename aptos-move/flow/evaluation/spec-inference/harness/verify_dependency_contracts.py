"""Prove corpus dependency contracts with their call-site opacity intact.

``pragma opaque`` changes how callers reason about a function; it does not
disable verification of that function's own body. This tool therefore selects
each ordinary opaque function as the prover target without editing the package.
Its body is checked against its contract while calls to opaque dependencies
continue to use their contracts. Functions with ``pragma verify = false`` are
handled separately as explicit trusted boundaries.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .artifacts import sha256_file, tree_hash, write_json
from .dependency_contracts import audit_dependency_contracts


@dataclass(frozen=True)
class OpaqueContractTarget:
    """One authored opaque boundary, in dependency order."""

    name: str
    target: str
    specification: str
    function: str


def verify_dependency_contracts(
    package: Path,
    manifest: Path,
    move_flow: Path,
    output: Path,
    timeout_seconds: int,
    selection_policy: Path,
    trusted_boundaries: Path | None = None,
) -> dict[str, Any]:
    package = package.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    compile_output = output.parent / f"{output.stem}.package.json"
    compile_result = _run(
        [
            str(move_flow),
            "experiment",
            "check-package",
            "--package",
            str(package),
            "--output",
            str(compile_output),
        ]
    )
    checks: list[dict[str, Any]] = [
        {
            "kind": "package_compile",
            "stage": "check_package",
            "passed": compile_result.returncode == 0 and _stage_passed(compile_output),
            "output": str(compile_output),
            "stdout": compile_result.stdout,
            "stderr": compile_result.stderr,
        }
    ]
    # Keep the audit inputs as the canonical metadata siblings of the main
    # verification record.  A verifier rerun refreshes them rather than
    # accumulating output-name-specific duplicates.
    inventory_output = output.parent / "package-inventory.json"
    inventory_result = _run(
        [
            str(move_flow),
            "experiment",
            "inventory-package",
            "--package",
            str(package),
            "--output",
            str(inventory_output),
            "--selection-policy",
            str(selection_policy),
        ]
    )
    inventory = None
    if inventory_result.returncode == 0 and inventory_output.is_file():
        inventory = json.loads(inventory_output.read_text(encoding="utf-8"))
    else:
        checks.append(
            {
                "kind": "dependency_call_graph",
                "stage": "inventory_package",
                "passed": False,
                "output": str(inventory_output),
                "stdout": inventory_result.stdout,
                "stderr": inventory_result.stderr,
            }
        )

    contract_report_output = output.parent / "contract-report.json"
    contract_report_result = _run(
        [
            str(move_flow),
            "experiment",
            "contract-report",
            "--package",
            str(package),
            "--output",
            str(contract_report_output),
        ]
    )
    audit_output = output.parent / "dependency-contract-audit.json"
    audit = None
    if (
        inventory is not None
        and contract_report_result.returncode == 0
        and contract_report_output.is_file()
    ):
        audit = audit_dependency_contracts(
            manifest,
            contract_report_output,
            audit_output,
            inventory_output,
            trusted_boundaries,
        )
    audit_ready = audit is not None and audit["ready"]
    checks.append(
        {
            "kind": "dependency_contract_audit",
            "stage": "contract_report",
            "passed": audit_ready,
            "output": str(audit_output),
            "contract_report": str(contract_report_output),
            "stdout": contract_report_result.stdout,
            "stderr": contract_report_result.stderr,
            "blocking_status_counts": (
                {} if audit is None else audit["status_counts"]
            ),
        }
    )

    targets: list[OpaqueContractTarget] = []
    if audit_ready:
        if inventory is not None:
            targets = _bottom_up_targets(audit, inventory)

    if targets:
        # Building the model does not depend on the target and is the expensive
        # part, so every contract is proved against one build.
        summary_output = output.parent / f"{output.stem}.contracts.json"
        argv = [
            str(move_flow),
            "experiment",
            "prove",
            "--package",
            str(package),
            "--timeout",
            str(timeout_seconds),
            "--output",
            str(summary_output),
        ]
        for item in targets:
            argv += ["--target", item.target]
        result = _run(argv)
        for item in targets:
            prove_output = _target_report_path(summary_output, item.target)
            checks.append(
                {
                    "kind": "opaque_contract_proof",
                    "name": item.name,
                    "target": item.target,
                    "specification": item.specification,
                    "function": item.function,
                    "opaque_preserved": True,
                    "passed": _stage_passed(prove_output),
                    "output": str(prove_output),
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                }
            )
    result = {
        "schema_version": 3,
        "package": str(package),
        "manifest": str(manifest),
        "package_tree_sha256": tree_hash(package),
        "move_flow": str(move_flow),
        "move_flow_sha256": sha256_file(move_flow),
        "timeout_seconds": timeout_seconds,
        "contract_report": str(contract_report_output),
        "contract_audit": str(audit_output),
        "inventory": str(inventory_output),
        "trusted_boundaries": (
            None if trusted_boundaries is None else str(trusted_boundaries)
        ),
        "bottom_up_order": [item.target for item in targets],
        "checks": checks,
        "passed": all(check["passed"] for check in checks),
    }
    write_json(output, result)
    return result


def _bottom_up_targets(
    audit: dict[str, Any], inventory: dict[str, Any]
) -> list[OpaqueContractTarget]:
    """Order every selected non-intrinsic contract from callees to callers.

    `dependency_contracts` defines the exact function-dependency closure used
    by the corpus.  This routine deliberately refuses an incomplete audit:
    proving a caller against a `sathard`, vacuous, or partial-abort callee
    contract would not establish a bottom-up proof of the corpus boundary.
    """
    if not audit.get("ready"):
        blocking = [
            f"{record['function']} ({record['status']})"
            for record in audit.get("contracts", [])
            if record["status"]
            not in {
                "opaque_contract",
                "opaque_intrinsic_model_boundary",
                "opaque_trusted_assumption",
                "native_binding",
                "intrinsic_binding",
            }
        ]
        raise ValueError(
            "dependency contract audit is not ready: " + ", ".join(sorted(blocking))
        )
    opaque_records = [
        record
        for record in audit.get("contracts", [])
        if record["status"] == "opaque_contract"
    ]
    candidates = {
        candidate["package_module_target"]: candidate
        for candidate in inventory.get("candidates", [])
        if candidate.get("package_module_target")
    }
    selected = {record["function"]: record for record in opaque_records}
    missing = sorted(function for function in selected if function not in candidates)
    if missing:
        raise ValueError(
            "opaque dependency contracts missing from inventory: " + ", ".join(missing)
        )

    prerequisites = {
        function: {
            callee
            for callee in candidates[function].get("called_function_dependencies", [])
            if callee in selected and callee != function
        }
        for function in selected
    }
    callers: dict[str, set[str]] = defaultdict(set)
    for function, callees in prerequisites.items():
        for callee in callees:
            callers[callee].add(function)
    ready = sorted(function for function, callees in prerequisites.items() if not callees)
    ordered: list[str] = []
    while ready:
        function = ready.pop(0)
        ordered.append(function)
        for caller in sorted(callers[function]):
            prerequisites[caller].remove(function)
            if not prerequisites[caller]:
                ready.append(caller)
        ready.sort()
    cyclic = sorted(function for function, callees in prerequisites.items() if callees)
    if cyclic:
        raise ValueError(
            "recursive opaque dependency-contract cycle: " + ", ".join(cyclic)
        )

    targets = []
    for function in ordered:
        record = selected[function]
        specification_paths = record["specification_paths"]
        if len(specification_paths) != 1:
            raise ValueError(
                f"{function} has {len(specification_paths)} companion specification paths"
            )
        targets.append(
            OpaqueContractTarget(
                name=function.replace("::", "-").replace("0x", "address-"),
                target=function,
                specification=specification_paths[0],
                function=function.rsplit("::", 1)[1],
            )
        )
    return targets


def _run(arguments: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(arguments, capture_output=True, text=True, check=False)


def _target_report_path(summary_output: Path, target: str) -> Path:
    """Where `flow experiment prove` writes the report for one of its targets."""
    sanitized = "".join(
        character if character.isalnum() else "_" for character in target
    )
    return summary_output.with_name(f"{summary_output.stem}.{sanitized}.json")


def _stage_passed(path: Path) -> bool:
    if not path.is_file():
        return False
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return False
    return isinstance(value, dict) and value.get("passed") is True


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--move-flow", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int, default=40)
    parser.add_argument(
        "--selection-policy",
        type=Path,
        default=Path(__file__).resolve().parent.parent
        / "config"
        / "corpus-selection.json",
        help="corpus selection policy the inventory is taken under",
    )
    parser.add_argument("--trusted-boundaries", type=Path)
    args = parser.parse_args()
    result = verify_dependency_contracts(
        args.package,
        args.manifest.resolve(),
        args.move_flow.resolve(),
        args.output,
        args.timeout_seconds,
        args.selection_policy.resolve(),
        (
            None
            if args.trusted_boundaries is None
            else args.trusted_boundaries.resolve()
        ),
    )
    print(json.dumps({"checks": len(result["checks"]), "passed": result["passed"]}))
    if not result["passed"]:
        raise SystemExit("one or more dependency contracts did not verify")


if __name__ == "__main__":
    main()
