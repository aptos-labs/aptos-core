"""Audit contracts for the exact function-dependency union of a corpus."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from .artifacts import write_json


def audit_dependency_contracts(
    manifest_path: Path,
    contract_report_path: Path,
    output_path: Path,
    inventory_path: Path | None = None,
    trusted_boundaries_path: Path | None = None,
) -> dict[str, Any]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    report = json.loads(contract_report_path.read_text(encoding="utf-8"))
    contracts = {record["function"]: record for record in report["functions"]}
    trusted_boundaries = _load_trusted_boundaries(trusted_boundaries_path)
    spec_functions = {
        record["function"]: record for record in report.get("spec_functions", [])
    }
    candidates = {}
    if inventory_path is not None:
        inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
        candidates = {
            candidate["package_module_target"]: candidate
            for candidate in inventory.get("candidates", [])
            if candidate.get("package_module_target")
        }
    consumers: dict[str, set[str]] = defaultdict(set)
    spec_consumers: dict[str, set[str]] = defaultdict(set)
    for task in manifest["records"]:
        target_functions = {
            (
                task["package_module_target"]
                if task["granularity"] == "function"
                else f"{task['module']}::{function}"
            )
            for function in task["target_functions"]
        }
        for dependency in task["called_function_dependencies"]:
            if dependency in target_functions:
                continue
            consumers[dependency].add(task["task_id"])
        for dependency in task.get("spec_function_dependencies", []):
            spec_consumers[dependency].add(task["task_id"])

    _expand_contract_proof_closure(consumers, spec_consumers, candidates)

    unused_trusted_boundaries = sorted(set(trusted_boundaries) - set(consumers))
    if unused_trusted_boundaries:
        raise ValueError(
            "trusted boundaries are outside the dependency closure: "
            + ", ".join(unused_trusted_boundaries)
        )
    for function in sorted(trusted_boundaries):
        contract = contracts.get(function)
        if contract is None or not contract.get("verification_disabled", False):
            raise ValueError(
                f"trusted boundary does not have pragma verify = false: {function}"
            )

    records = []
    for function in sorted(consumers):
        contract = contracts.get(function)
        trusted_boundary = trusted_boundaries.get(function)
        global_resource_types = _global_resource_types(function, candidates)
        mutates_global_resource = bool(global_resource_types)
        declared_modifies_resource_types = (
            [] if contract is None else contract.get("modifies_resource_types", [])
        )
        missing_modifies_resource_types = _missing_modifies_resource_types(
            global_resource_types, declared_modifies_resource_types
        )
        status = _status(
            contract,
            mutates_global_resource,
            bool(missing_modifies_resource_types),
            None if trusted_boundary is None else trusted_boundary["basis"],
        )
        records.append(
            {
                "function": function,
                "module": function.rsplit("::", 1)[0],
                "status": status,
                "consumer_tasks": sorted(consumers[function]),
                "condition_count": 0 if contract is None else contract["condition_count"],
                "informative_condition_count": (
                    0
                    if contract is None
                    else contract.get(
                        "informative_condition_count", contract["condition_count"]
                    )
                ),
                "untrusted_inferred_condition_count": (
                    0
                    if contract is None
                    else contract.get("untrusted_inferred_condition_count", 0)
                ),
                "untrusted_inferred_condition_kinds": (
                    []
                    if contract is None
                    else contract.get("untrusted_inferred_condition_kinds", [])
                ),
                "has_partial_aborts_if": (
                    False
                    if contract is None
                    else contract.get("has_partial_aborts_if", False)
                ),
                "verification_disabled": (
                    False
                    if contract is None
                    else contract.get("verification_disabled", False)
                ),
                "trusted_boundary": trusted_boundary,
                "condition_kinds": [] if contract is None else contract["condition_kinds"],
                "has_abort_contract": (
                    False
                    if contract is None
                    else _has_abort_contract(contract["condition_kinds"])
                ),
                "global_resource_mutation": mutates_global_resource,
                "global_resource_types": global_resource_types,
                "has_modifies_clause": (
                    False
                    if contract is None
                    else contract.get("has_modifies_clause", False)
                ),
                "modifies_target_count": (
                    0
                    if contract is None
                    else contract.get("modifies_target_count", 0)
                ),
                "modifies_all": (
                    False if contract is None else contract.get("modifies_all", False)
                ),
                "declared_modifies_resource_types": declared_modifies_resource_types,
                "missing_modifies_resource_types": missing_modifies_resource_types,
                "specification_paths": (
                    [] if contract is None else contract["specification_paths"]
                ),
                "opaque": False if contract is None else contract["opaque"],
                "native": False if contract is None else contract["native"],
                "intrinsic": False if contract is None else contract.get("intrinsic", False),
            }
        )

    counts = Counter(record["status"] for record in records)
    modules_requiring_work = sorted(
        {
            record["module"]
            for record in records
            if record["status"]
            in {
                "missing_contract",
                "opaque_without_conditions",
                "opaque_without_informative_conditions",
                "opaque_with_untrusted_inferred_conditions",
                "opaque_with_partial_abort_contract",
                "opaque_with_verification_disabled",
                "opaque_without_abort_contract",
                "opaque_without_modifies_contract",
                "opaque_with_incomplete_modifies_contract",
                "nonopaque_contract",
            }
        }
    )
    spec_records = []
    for function in sorted(spec_consumers):
        declaration = spec_functions.get(function)
        if declaration is None:
            status = "absent_from_package"
        elif declaration["has_body"]:
            status = "defined"
        elif declaration["native"]:
            status = "native"
        elif declaration["uninterpreted"]:
            status = "uninterpreted"
        else:
            status = "bodyless_unmodeled"
        spec_records.append(
            {
                "function": function,
                "module": function.rsplit("::", 1)[0],
                "status": status,
                "consumer_tasks": sorted(spec_consumers[function]),
                "has_body": False if declaration is None else declaration["has_body"],
                "native": False if declaration is None else declaration["native"],
                "uninterpreted": (
                    False if declaration is None else declaration["uninterpreted"]
                ),
                "move_function_companion": (
                    False
                    if declaration is None
                    else declaration["move_function_companion"]
                ),
            }
        )
    spec_counts = Counter(record["status"] for record in spec_records)
    spec_modules_requiring_work = sorted(
        {
            record["module"]
            for record in spec_records
            if record["status"] in {"absent_from_package", "bodyless_unmodeled"}
        }
    )
    result = {
        "schema_version": 9,
        "manifest": str(manifest_path),
        "contract_report": str(contract_report_path),
        "inventory": None if inventory_path is None else str(inventory_path),
        "trusted_boundaries": (
            None if trusted_boundaries_path is None else str(trusted_boundaries_path)
        ),
        "trusted_boundary_count": len(trusted_boundaries),
        "dependency_function_count": len(records),
        "status_counts": dict(sorted(counts.items())),
        "modules_requiring_work": modules_requiring_work,
        "spec_function_count": len(spec_records),
        "spec_status_counts": dict(sorted(spec_counts.items())),
        "spec_modules_requiring_work": spec_modules_requiring_work,
        "ready": not modules_requiring_work and not spec_modules_requiring_work,
        "contracts": records,
        "spec_functions": spec_records,
    }
    write_json(output_path, result)
    return result


def _expand_contract_proof_closure(
    consumers: dict[str, set[str]],
    spec_consumers: dict[str, set[str]],
    candidates: dict[str, dict[str, Any]],
) -> None:
    """Add every direct callee needed to prove an opaque contract boundary."""
    pending = list(consumers)
    seen = set()
    while pending:
        function = pending.pop()
        if function in seen:
            continue
        seen.add(function)
        candidate = candidates.get(function)
        if candidate is None:
            continue
        task_ids = consumers[function]
        for spec_function in candidate.get("spec_function_dependencies", []):
            spec_consumers[spec_function].update(task_ids)
        for callee in candidate.get("called_function_dependencies", []):
            if callee == function:
                continue
            known = callee in consumers
            consumers[callee].update(task_ids)
            if not known:
                pending.append(callee)


def _global_resource_types(
    function: str, candidates: dict[str, dict[str, Any]]
) -> list[str]:
    """Return the global resource types an executable closure can mutate.

    The inventory records direct AST effects. Its exact post-inline call
    closure lets an opaque boundary inherit effects from transparent helpers,
    while excluding read-only global operations.
    """
    candidate = candidates.get(function)
    if candidate is None:
        return []
    types = set(candidate.get("global_resource_types", []))
    for callee in candidate.get("transitive_called_function_dependencies", []):
        types.update(candidates.get(callee, {}).get("global_resource_types", []))
    # Backward-compatible fallback for inventories produced before resource
    # types were recorded: keep enforcing the presence of `modifies`, but do
    # not pretend to know which type an old report omitted.
    if not types and candidate.get("global_resource_mutation", False):
        types.add("<unknown-global-resource>")
    return sorted(types)


def _missing_modifies_resource_types(
    required: list[str], declared: list[str]
) -> list[str]:
    """Return resource effects not covered by a declared modifies family.

    Inventory entries below a generic callee retain that callee's type
    parameters (for example ``CoinInfo<#0>``), while a caller's contract must
    name the concrete instantiation it can actually mutate (for example
    ``CoinInfo<AptosCoin>``). Treat a concrete declaration from the same
    resource family as covering a placeholder-bearing inventory effect. Keep
    concrete-to-concrete comparisons exact so a frame for one coin type does
    not silently cover another.
    """
    declared_set = set(declared)
    declared_families = {_resource_type_family(resource) for resource in declared}
    missing = []
    for resource in required:
        if resource in declared_set:
            continue
        if _has_type_parameter_placeholder(resource) and (
            _resource_type_family(resource) in declared_families
        ):
            continue
        missing.append(resource)
    return sorted(missing)


def _resource_type_family(resource: str) -> str:
    return resource.split("<", 1)[0]


def _has_type_parameter_placeholder(resource: str) -> bool:
    return "<" in resource and "#" in resource


def _status(
    contract: dict[str, Any] | None,
    mutates_global_resource: bool = False,
    missing_modifies_resource_types: bool = False,
    trusted_verification_basis: str | None = None,
) -> str:
    if contract is None:
        return "absent_from_package"
    if contract.get("intrinsic", False):
        # Prover intrinsics are modeled by the backend. They have no Move
        # contract boundary to make opaque or prove directly against its body.
        return "intrinsic_binding"
    if contract["native"]:
        return "native_binding"
    if contract["opaque"]:
        if contract.get("untrusted_inferred_condition_count", 0):
            return "opaque_with_untrusted_inferred_conditions"
        if contract.get("has_partial_aborts_if", False):
            return "opaque_with_partial_abort_contract"
        if contract.get("informative_condition_count", contract["condition_count"]):
            if not _has_abort_contract(contract["condition_kinds"]):
                # Without an explicit `aborts_if` clause, opaque callers cannot
                # consume a no-abort guarantee. `aborts_if false` is therefore
                # required even for otherwise total implementations.
                return "opaque_without_abort_contract"
            if mutates_global_resource and not contract.get("has_modifies_clause", False):
                # `modifies` is a frame specification rather than a regular
                # condition. Every opaque boundary that may change a global
                # resource must name that footprint; mutable parameters and
                # read-only global access do not trigger this requirement.
                return "opaque_without_modifies_contract"
            if missing_modifies_resource_types:
                return "opaque_with_incomplete_modifies_contract"
            if contract.get("verification_disabled", False):
                if trusted_verification_basis == "intrinsic_model":
                    return "opaque_intrinsic_model_boundary"
                if trusted_verification_basis is not None:
                    return "opaque_trusted_assumption"
                return "opaque_with_verification_disabled"
            return "opaque_contract"
        if contract["condition_count"]:
            return "opaque_without_informative_conditions"
        return "opaque_without_conditions"
    if contract["condition_count"]:
        return "nonopaque_contract"
    return "missing_contract"


def _load_trusted_boundaries(path: Path | None) -> dict[str, dict[str, Any]]:
    if path is None:
        return {}
    document = json.loads(path.read_text(encoding="utf-8"))
    boundaries: dict[str, dict[str, Any]] = {}
    for record in document.get("boundaries", []):
        function = record.get("function")
        basis = record.get("basis")
        if not isinstance(function, str) or not function:
            raise ValueError("trusted boundary is missing a function")
        if function in boundaries:
            raise ValueError(f"duplicate trusted boundary: {function}")
        if basis not in {
            "proved_at_timeout",
            "expert_assumption",
            "intrinsic_model",
        }:
            raise ValueError(f"invalid trusted-boundary basis for {function}: {basis}")
        if basis == "proved_at_timeout":
            timeout = record.get("timeout_seconds")
            artifact = record.get("artifact")
            if not isinstance(timeout, int) or timeout <= 0:
                raise ValueError(f"invalid proof timeout for {function}")
            if not isinstance(artifact, str) or not artifact:
                raise ValueError(f"missing proof artifact for {function}")
        rationale = record.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            raise ValueError(f"missing trusted-boundary rationale for {function}")
        boundaries[function] = record
    return boundaries


def _has_abort_contract(condition_kinds: list[str]) -> bool:
    return "AbortsIf" in condition_kinds


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--contract-report", type=Path, required=True)
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--trusted-boundaries", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_dependency_contracts(
        args.manifest.resolve(),
        args.contract_report.resolve(),
        args.output.resolve(),
        None if args.inventory is None else args.inventory.resolve(),
        None if args.trusted_boundaries is None else args.trusted_boundaries.resolve(),
    )
    print(
        json.dumps(
            {
                "dependency_function_count": result["dependency_function_count"],
                "modules_requiring_work": len(result["modules_requiring_work"]),
                "ready": result["ready"],
                "spec_function_count": result["spec_function_count"],
                "spec_modules_requiring_work": len(
                    result["spec_modules_requiring_work"]
                ),
                "status_counts": result["status_counts"],
                "spec_status_counts": result["spec_status_counts"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
