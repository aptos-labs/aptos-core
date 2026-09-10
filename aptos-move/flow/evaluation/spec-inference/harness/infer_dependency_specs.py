"""Generate opaque WP contracts for dependency modules which lack coverage."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from .artifacts import write_json
from .prepare import _atomic_text
from .restore_reference_contracts import _function_spec_spans, restore_reference_contracts


FOUNDATION_ORDER = (
    "0x1::error",
    "0x1::cmp",
    "0x1::signer",
    "0x1::type_info",
    "0x1::string",
    "0x1::vector",
    "0x1::result",
    "0x1::from_bcs",
    "0x1::table",
    "0x1::table_with_length",
    "0x1::simple_map",
    "0x1::ordered_map",
    "0x1::storage_slots_allocator",
    "0x1::big_ordered_map",
    "0x1::features",
    "0x1::event",
    "0x1::function_info",
    "0x1::object",
    "0x1::aggregator_v2",
    "0x1::coin",
    "0x1::fungible_asset",
    "0x1::dispatchable_fungible_asset",
    "0x1::primary_fungible_store",
)


def infer_dependency_specs(
    cli: Path,
    package: Path,
    audit_path: Path,
    artifacts: Path,
    output_path: Path,
    timeout_seconds: int,
    function_level_modules: set[str] | None = None,
    reference_audit_path: Path | None = None,
) -> dict[str, Any]:
    audit = json.loads(audit_path.read_text(encoding="utf-8"))
    manifest_path = Path(audit["manifest"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    repo_root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=package,
            capture_output=True,
            text=True,
            timeout=30,
            check=True,
        ).stdout.strip()
    )
    work = set(audit["modules_requiring_work"])
    modules = [module for module in FOUNDATION_ORDER if module in work]
    modules.extend(sorted(work - set(modules)))
    artifacts.mkdir(parents=True, exist_ok=True)
    records = []
    stopped_early = False
    function_level_modules = function_level_modules or set()
    reference_audit_path = reference_audit_path or audit_path
    for module in modules:
        functions = [
            contract["function"]
            for contract in audit["contracts"]
            if contract["module"] == module
            and contract["status"]
            in {
                "missing_contract",
                "opaque_without_conditions",
                "nonopaque_contract",
            }
        ]
        if module in function_level_modules:
            record = {
                "target": module,
                "status": "timeout",
                "elapsed_seconds": 0.0,
                "timeout_seconds": timeout_seconds,
                "returncode": None,
                "report": None,
                "stdout": "",
                "stderr": "module inference skipped after a recorded timeout",
                "compile_check": None,
                "module_attempt_skipped": True,
            }
        else:
            record = _infer_target_transaction(
                cli,
                package,
                module,
                reference_audit_path,
                artifacts,
                repo_root,
                manifest["source_commit"],
                timeout_seconds,
                functions,
            )
        record["module"] = module
        if record["status"] != "passed":
            fallback = []
            for function in functions:
                attempt = _infer_target_transaction(
                    cli,
                    package,
                    function,
                    reference_audit_path,
                    artifacts,
                    repo_root,
                    manifest["source_commit"],
                    timeout_seconds,
                    [function],
                )
                fallback.append(attempt)
                print(
                    json.dumps(
                        {
                            "module": module,
                            "fallback_target": function,
                            "status": attempt["status"],
                            "elapsed_seconds": attempt["elapsed_seconds"],
                        },
                        sort_keys=True,
                    ),
                    flush=True,
                )
                if attempt["status"] != "passed":
                    break
            module_attempt = dict(record)
            record["module_attempt"] = module_attempt
            record["function_fallback"] = fallback
            record["elapsed_seconds"] = round(
                module_attempt["elapsed_seconds"]
                + sum(attempt["elapsed_seconds"] for attempt in fallback),
                3,
            )
            if len(fallback) == len(functions) and all(
                attempt["status"] == "passed" for attempt in fallback
            ):
                record["status"] = "passed"
                record["fallback_mode"] = "function_targets_after_module_timeout"
            elif fallback:
                record["status"] = fallback[-1]["status"]
        records.append(record)
        print(
            json.dumps(
                {
                    "module": module,
                    "status": record["status"],
                    "elapsed_seconds": record["elapsed_seconds"],
                },
                sort_keys=True,
            ),
            flush=True,
        )
        write_json(
            output_path,
            {
                "schema_version": 1,
                "package": str(package),
                "audit": str(audit_path),
                "reference_audit": str(reference_audit_path),
                "timeout_seconds": timeout_seconds,
                "complete": False,
                "modules": records,
            },
        )
        # Failed attempts are discarded transactionally. Stop at the first
        # unresolved target so its exact diagnostics remain easy to inspect.
        if record["status"] != "passed":
            stopped_early = True
            break
    result = {
        "schema_version": 1,
        "package": str(package),
        "audit": str(audit_path),
        "reference_audit": str(reference_audit_path),
        "timeout_seconds": timeout_seconds,
        "complete": not stopped_early,
        "stopped_early": stopped_early,
        "status_counts": {
            status: sum(record["status"] == status for record in records)
            for status in (
                "passed",
                "failed",
                "timeout",
                "generated_compile_failed",
                "generated_compile_timeout",
                "generated_contract_failed",
                "generated_contract_timeout",
                "generated_contract_missing",
                "reference_restore_failed",
            )
        },
        "modules": records,
    }
    write_json(output_path, result)
    return result


def _infer_target_transaction(
    cli: Path,
    package: Path,
    target: str,
    audit_path: Path,
    artifacts: Path,
    repo_root: Path,
    source_commit: str,
    timeout_seconds: int,
    required_functions: list[str],
) -> dict[str, Any]:
    stem = target.replace("::", "-")
    report = artifacts / f"{stem}.json"
    compile_report = artifacts / f"{stem}.compile.json"
    contract_report = artifacts / f"{stem}.contracts.json"
    restoration_report = artifacts / f"{stem}.restored-reference-contracts.json"
    started = time.monotonic()
    compile_check: dict[str, Any] | None = None
    contract_check: dict[str, Any] | None = None
    conservative_boundaries: list[dict[str, str]] = []
    with tempfile.TemporaryDirectory(
        prefix=f"move-inference-dependency-{stem}-"
    ) as temporary:
        working_package = Path(temporary) / "package"
        shutil.copytree(
            package,
            working_package,
            ignore=shutil.ignore_patterns("build"),
        )
        command = [
            str(cli),
            "experiment",
            "infer",
            "--package",
            str(working_package),
            "--target",
            target,
            "--write-inferred-specs",
            "--output",
            str(report),
        ]
        try:
            completed = subprocess.run(
                command,
                capture_output=True,
                text=True,
                timeout=timeout_seconds,
                check=False,
            )
            status = "passed" if completed.returncode == 0 else "failed"
            returncode = completed.returncode
            stderr = completed.stderr
            stdout = completed.stdout
        except subprocess.TimeoutExpired as error:
            status = "timeout"
            returncode = None
            stderr = _decode(error.stderr)
            stdout = _decode(error.stdout)
        if status == "passed":
            restoration = restore_reference_contracts(
                working_package,
                repo_root,
                source_commit,
                audit_path,
                restoration_report,
            )
            if restoration["unresolved"]:
                status = "reference_restore_failed"
            conservative_boundaries = _complete_empty_dependency_boundaries(
                working_package, required_functions
            )
            check_started = time.monotonic()
            check_command = [
                str(cli),
                "experiment",
                "check-package",
                "--package",
                str(working_package),
                "--output",
                str(compile_report),
            ]
            try:
                checked = subprocess.run(
                    check_command,
                    capture_output=True,
                    text=True,
                    timeout=timeout_seconds,
                    check=False,
                )
                compile_check = {
                    "status": "passed" if checked.returncode == 0 else "failed",
                    "elapsed_seconds": round(time.monotonic() - check_started, 3),
                    "returncode": checked.returncode,
                    "report": str(compile_report),
                    "stdout": checked.stdout,
                    "stderr": checked.stderr,
                }
                if checked.returncode != 0:
                    status = "generated_compile_failed"
            except subprocess.TimeoutExpired as error:
                compile_check = {
                    "status": "timeout",
                    "elapsed_seconds": round(time.monotonic() - check_started, 3),
                    "returncode": None,
                    "report": str(compile_report),
                    "stdout": _decode(error.stdout),
                    "stderr": _decode(error.stderr),
                }
                status = "generated_compile_timeout"
        if status == "passed":
            contract_started = time.monotonic()
            contract_command = [
                str(cli),
                "experiment",
                "contract-report",
                "--package",
                str(working_package),
                "--output",
                str(contract_report),
            ]
            try:
                checked = subprocess.run(
                    contract_command,
                    capture_output=True,
                    text=True,
                    timeout=timeout_seconds,
                    check=False,
                )
                contract_check = {
                    "status": "passed" if checked.returncode == 0 else "failed",
                    "elapsed_seconds": round(time.monotonic() - contract_started, 3),
                    "returncode": checked.returncode,
                    "report": str(contract_report),
                    "stdout": checked.stdout,
                    "stderr": checked.stderr,
                    "missing": [],
                }
                if checked.returncode != 0:
                    status = "generated_contract_failed"
                else:
                    reported = json.loads(contract_report.read_text(encoding="utf-8"))
                    by_function = {
                        function["function"]: function
                        for function in reported["functions"]
                    }
                    missing = [
                        function
                        for function in required_functions
                        if not _is_complete_opaque_contract(by_function.get(function))
                    ]
                    contract_check["missing"] = missing
                    if missing:
                        contract_check["status"] = "missing"
                        status = "generated_contract_missing"
            except subprocess.TimeoutExpired as error:
                contract_check = {
                    "status": "timeout",
                    "elapsed_seconds": round(time.monotonic() - contract_started, 3),
                    "returncode": None,
                    "report": str(contract_report),
                    "stdout": _decode(error.stdout),
                    "stderr": _decode(error.stderr),
                    "missing": required_functions,
                }
                status = "generated_contract_timeout"
        if status == "passed":
            shutil.copytree(
                working_package / "sources",
                package / "sources",
                dirs_exist_ok=True,
            )
    return {
        "target": target,
        "status": status,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "timeout_seconds": timeout_seconds,
        "returncode": returncode,
        "report": str(report),
        "stdout": stdout,
        "stderr": stderr,
        "compile_check": compile_check,
        "contract_check": contract_check,
        "conservative_boundaries": conservative_boundaries,
    }


def _is_complete_opaque_contract(function: dict[str, Any] | None) -> bool:
    return bool(
        function
        and function.get("opaque")
        and function.get("condition_count", 0) > 0
        and function.get("untrusted_inferred_condition_count", 0) == 0
    )


_CONDITION = re.compile(
    r"\b(?:requires|ensures|aborts_if|aborts_with|modifies|emits|invariant|decreases)\b"
)


def _complete_empty_dependency_boundaries(
    package: Path, required_functions: list[str]
) -> list[dict[str, str]]:
    """Give an empty inferred dependency contract an explicit sound result boundary."""
    catalog = json.loads((package / "corpus-modules.json").read_text(encoding="utf-8"))
    paths_by_module: dict[str, set[str]] = {}
    for module in catalog["modules"]:
        implementation = Path(module["implementation"])
        paths = set(module.get("specifications", []))
        paths.add(implementation.with_name(implementation.stem + ".spec.move").as_posix())
        paths_by_module[module["module"]] = paths

    completed = []
    for qualified in required_functions:
        module, function = qualified.rsplit("::", 1)
        matches: list[tuple[Path, int, int]] = []
        for relative in sorted(paths_by_module.get(module, set())):
            path = package / relative
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
            matches.extend((path, start, end) for start, end in _function_spec_spans(text, function))
        empty = []
        for path, start, end in matches:
            block = path.read_text(encoding="utf-8")[start:end]
            if "pragma opaque" in block and not _CONDITION.search(block):
                empty.append((path, start, end))
        if not empty:
            continue
        if len(empty) != 1:
            raise ValueError(f"expected one empty inferred block for {qualified}, got {len(empty)}")
        path, start, end = empty[0]
        text = path.read_text(encoding="utf-8")
        block = text[start:end]
        if "aborts_if_is_partial" not in block:
            raise ValueError(f"empty inferred dependency is not abort-partial: {qualified}")
        closing = block.rfind("}")
        line_start = text.rfind("\n", 0, start) + 1
        indent = text[line_start:start]
        addition = (
            f"{indent}    // Corpus dependency abstraction: WP emitted no usable condition.\n"
            f"{indent}    ensures true;\n"
        )
        updated = text[: start + closing] + addition + text[start + closing :]
        _atomic_text(path, updated)
        completed.append({"function": qualified, "path": path.relative_to(package).as_posix()})
    return completed


def _decode(value: bytes | str | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode(errors="replace")
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument(
        "--reference-audit",
        type=Path,
        help="immutable pre-generation audit used to identify handwritten contracts",
    )
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=int, default=60)
    parser.add_argument(
        "--function-level-module",
        action="append",
        default=[],
        help="skip module-wide WP and infer each missing dependency function",
    )
    args = parser.parse_args()
    result = infer_dependency_specs(
        args.cli.resolve(),
        args.package.resolve(),
        args.audit.resolve(),
        args.artifacts.resolve(),
        args.output.resolve(),
        args.timeout_seconds,
        set(args.function_level_module),
        args.reference_audit.resolve() if args.reference_audit else None,
    )
    print(json.dumps(result["status_counts"], sort_keys=True))


if __name__ == "__main__":
    main()
