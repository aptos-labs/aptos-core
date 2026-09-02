"""Materialize reproducible, agent-visible packages for the selected corpus."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path
from typing import Any

from .artifacts import copy_snapshot, load_object, sha256_file, tree_hash, write_json
from .move_source import mask_comments_and_strings
from .shared_package import build_shared_package


FRAMEWORK = "aptos-move/framework/aptos-framework"
EXPERIMENTAL = "aptos-move/framework/aptos-experimental"
IGNORED_NAMES = {
    ".git",
    "build",
    "__pycache__",
    "tests",
    "benchmark_utils.move",
    "large_packages.move",
}


def prepare_corpus(
    provenance_path: Path,
    repo_root: Path,
    artifacts_root: Path,
    patches_dir: Path,
    output_path: Path,
) -> dict[str, Any]:
    provenance = load_object(provenance_path)
    commit = _source_commit(repo_root)
    if commit != provenance.get("source_commit"):
        raise ValueError(
            f"source checkout is {commit}, expected {provenance.get('source_commit')}"
        )
    selected = [
        record
        for record in provenance["records"]
        if record["selection_status"] == "selected"
    ]
    if len(selected) != 30:
        raise ValueError(f"expected 30 selected tasks, got {len(selected)}")

    shared = artifacts_root / "framework"
    shared_metadata = build_shared_package(repo_root, selected, shared)
    shared_hash = shared_metadata["tree_sha256"]
    source_path_map = shared_metadata["source_path_map"]

    patches_dir.mkdir(parents=True, exist_ok=True)
    prepared_records = []
    for record in selected:
        task_id = record["task_id"]
        record["corpus_source_path"] = source_path_map[record["source_path"]]
        record["corpus_reference_paths"] = [
            source_path_map[path]
            for path in record["reference_paths"]
            if path in source_path_map
        ]
        required_categories = _required_categories(record)
        patch_path = patches_dir / f"{task_id}.patch"
        _require_absent(patch_path)
        with tempfile.TemporaryDirectory(
            prefix=f"move-inference-{task_id}-"
        ) as temporary:
            prepared = Path(temporary) / "package"
            copy_snapshot(shared, prepared)
            removed_specs = _remove_target_references(prepared, record)
            descriptor = {
                "schema_version": 3,
                "task_id": task_id,
                "source_commit": commit,
                "source_path": record["corpus_source_path"],
                "package_module_target": record["package_module_target"],
                "granularity": record["granularity"],
                "target_functions": record["target_functions"],
                "transitive_function_dependencies": record[
                    "transitive_function_dependencies"
                ],
                "called_function_dependencies": record[
                    "called_function_dependencies"
                ],
                "spec_function_dependencies": record[
                    "spec_function_dependencies"
                ],
                "transitive_called_function_dependencies": record[
                    "transitive_called_function_dependencies"
                ],
                "transitive_module_dependencies": record[
                    "transitive_module_dependencies"
                ],
            }
            _atomic_text(
                prepared / ".move-inference-task.json",
                json.dumps(descriptor, indent=2, sort_keys=True) + "\n",
            )
            _write_git_patch(shared, prepared, patch_path)
            _verify_patch_reproduction(shared, prepared, patch_path, task_id)
            prepared_hash = tree_hash(prepared)

        for stale in (
            "dependency_closure",
            "prepared_path",
            "pristine_sha256",
            "resolved_address_aliases",
        ):
            record.pop(stale, None)
        record.update(
            {
                "prepared_sha256": prepared_hash,
                "shared_package_path": os.path.relpath(shared, output_path.parent),
                "shared_package_sha256": shared_hash,
                "package_relpath": ".",
                "allowed_edit_paths": _allowed_edit_paths(record),
                "required_contract_categories": required_categories,
                "preparation_patch": os.path.relpath(patch_path, output_path.parent),
                "preparation_patch_sha256": sha256_file(patch_path),
                "removed_reference_blocks": removed_specs,
                "snapshot_status": "shared_package_recipe",
            }
        )
        sample_dir = artifacts_root / "samples" / task_id
        _write_sample_catalog(
            sample_dir=sample_dir,
            shared=shared,
            patch_path=patch_path,
            record=record,
            source_commit=commit,
        )
        record["sample_path"] = os.path.relpath(sample_dir, output_path.parent)
        record["sample_readme"] = os.path.relpath(
            sample_dir / "README.md", output_path.parent
        )
        prepared_records.append(
            {
                "task_id": task_id,
                "shared_package_sha256": shared_hash,
                "prepared_sha256": prepared_hash,
                "preparation_patch_sha256": sha256_file(patch_path),
                "removed_reference_blocks": removed_specs,
                "required_contract_categories": required_categories,
            }
        )

    _write_corpus_catalog_readme(
        artifacts_root,
        selected,
        shared_metadata,
        commit,
    )

    result = {
        **provenance,
        "corpus_status": "prepared",
        "preparation": {
            "schema_version": 2,
            "source_commit": commit,
            "provenance_input_sha256": sha256_file(provenance_path),
            "artifacts_root": os.path.relpath(artifacts_root, output_path.parent),
            "shared_package": os.path.relpath(shared, output_path.parent),
            "shared_package_sha256": shared_hash,
            "module_count": shared_metadata["module_count"],
            "move_file_count": shared_metadata["move_file_count"],
            "sample_catalog": os.path.relpath(
                artifacts_root / "samples", output_path.parent
            ),
            "tasks_prepared": len(prepared_records),
            "patch_reproduction_verified": True,
            "records": prepared_records,
        },
        "records": provenance["records"],
    }
    write_json(output_path, result)
    return result


def _verify_patch_reproduction(
    pristine: Path, prepared: Path, patch: Path, task_id: str
) -> None:
    with tempfile.TemporaryDirectory(prefix="move-inference-prepare-") as temporary:
        rebuilt = Path(temporary) / "snapshot"
        copy_snapshot(pristine, rebuilt)
        result = subprocess.run(
            ["git", "apply", "--whitespace=nowarn", str(patch)],
            cwd=rebuilt,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            raise ValueError(
                f"preparation patch does not apply for {task_id}: "
                f"{result.stderr.strip()}"
            )
        if tree_hash(rebuilt) != tree_hash(prepared):
            raise ValueError(
                f"preparation patch does not reproduce prepared snapshot for {task_id}"
            )


def _copy_standalone_package(
    repo_root: Path,
    source_root: str,
    destination: Path,
    package_store: Path | None = None,
    shared_packages: dict[str, Path] | None = None,
) -> dict[str, Any]:
    source = (repo_root / source_root).resolve()
    if not (source / "Move.toml").is_file():
        raise ValueError(f"source package has no Move.toml: {source}")
    packages, edges = _resolve_dependency_closure(repo_root, source)
    destinations: dict[Path, Path] = {source: destination}
    names: dict[str, Path] = {}
    for package, manifest in packages.items():
        name = str(manifest["package"]["name"])
        if name in names and names[name] != package:
            raise ValueError(f"duplicate package name `{name}` in dependency closure")
        names[name] = package
        if package != source:
            destinations[package] = destination / "deps" / name
    for package in sorted(packages, key=lambda path: (path != source, path.as_posix())):
        if package_store is None:
            shutil.copytree(
                package,
                destinations[package],
                symlinks=True,
                ignore=_copy_ignore,
            )
            continue
        name = str(packages[package]["package"]["name"])
        stored = package_store / name
        if shared_packages is not None:
            previous = shared_packages.get(name)
            if previous is not None and previous != package:
                raise ValueError(
                    f"duplicate package name `{name}` in shared dependency store: "
                    f"{previous} versus {package}"
                )
            shared_packages[name] = package
        if not stored.exists():
            shutil.copytree(
                package,
                stored,
                symlinks=True,
                ignore=_copy_ignore,
            )
        _hardlink_tree(stored, destinations[package])
    for package, dependencies in edges.items():
        manifest_path = destinations[package] / "Move.toml"
        text = manifest_path.read_text(encoding="utf-8")
        for original, dependency in dependencies:
            relative = os.path.relpath(destinations[dependency], destinations[package])
            pattern = re.compile(
                rf"(local\s*=\s*['\"]){re.escape(original)}(['\"])"
            )
            text, count = pattern.subn(rf"\g<1>{relative}\g<2>", text)
            if count != 1:
                raise ValueError(
                    f"expected one local edge `{original}` in {package / 'Move.toml'}, got {count}"
                )
        _atomic_text(manifest_path, text)

    resolved_aliases: dict[str, str] = {}
    package_records = []
    for package, manifest in sorted(
        packages.items(), key=lambda item: str(item[1]["package"]["name"])
    ):
        aliases = {
            str(name): str(value)
            for name, value in manifest.get("addresses", {}).items()
        }
        dev_aliases = {
            str(name): str(value)
            for name, value in manifest.get("dev-addresses", {}).items()
        }
        for name, value in {**aliases, **dev_aliases}.items():
            if value == "_":
                continue
            previous = resolved_aliases.get(name)
            if previous is not None and previous.lower() != value.lower():
                raise ValueError(
                    f"address alias `{name}` resolves inconsistently: {previous} versus {value}"
                )
            resolved_aliases[name] = value
        copied_manifest = destinations[package] / "Move.toml"
        package_records.append(
            {
                "package": str(manifest["package"]["name"]),
                "source_path": package.relative_to(repo_root).as_posix(),
                "snapshot_path": destinations[package]
                .relative_to(destination)
                .as_posix(),
                "source_manifest_sha256": sha256_file(package / "Move.toml"),
                "snapshot_manifest_sha256": sha256_file(copied_manifest),
                "addresses": aliases,
                "dev_addresses": dev_aliases,
                "direct_local_dependencies": [
                    str(packages[dependency]["package"]["name"])
                    for _, dependency in edges[package]
                ],
            }
        )
    return {
        "packages": package_records,
        "resolved_address_aliases": dict(sorted(resolved_aliases.items())),
    }


def _write_sample_catalog(
    sample_dir: Path,
    shared: Path,
    patch_path: Path,
    record: dict[str, Any],
    source_commit: str,
) -> None:
    """Create human-facing metadata outside the agent-visible source tree."""
    _require_absent(sample_dir)
    sample_dir.mkdir(parents=True)
    framework_link = sample_dir / "framework"
    framework_link.symlink_to(
        os.path.relpath(shared, sample_dir), target_is_directory=True
    )
    patch_link = sample_dir / "preparation.patch"
    patch_link.symlink_to(os.path.relpath(patch_path, sample_dir))

    functions = "\n".join(
        f"- `{function}`" for function in record["target_functions"]
    )
    removed = record.get("removed_reference_blocks", [])
    removed_text = (
        "\n".join(
            f"- `{item['path']}`: `{item['function']}` ({item['blocks']} block(s))"
            for item in removed
        )
        if removed
        else "- None (this task has no embedded upstream target specification)."
    )
    categories = ", ".join(
        f"`{category}`" for category in record["required_contract_categories"]
    )
    allowed = "\n".join(f"- `{path}`" for path in record["allowed_edit_paths"])
    called_dependencies = record["called_function_dependencies"]
    called_text = (
        "\n".join(f"- `{function}`" for function in called_dependencies)
        if called_dependencies
        else "- None."
    )
    spec_dependencies = record["spec_function_dependencies"]
    spec_text = (
        "\n".join(f"- `{function}`" for function in spec_dependencies)
        if spec_dependencies
        else "- None."
    )
    module_dependencies = record["transitive_module_dependencies"]
    module_text = (
        "\n".join(f"- `{module}`" for module in module_dependencies)
        if module_dependencies
        else "- None."
    )
    readme = f"""# {record['task_id']}

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `{record['package_module_target']}`
- Granularity: `{record['granularity']}`
- Original source: `{record['source_path']}`
- Source inside the shared package: `{record['corpus_source_path']}`
- Source root: `{record['source_root']}`
- Aptos Core commit: `{source_commit}`
- Shared package SHA-256: `{record['shared_package_sha256']}`
- Prepared tree SHA-256: `{record['prepared_sha256']}`
- Required contract categories: {categories}

Target functions:

{functions}

## Compilation context

The shared package contains the union of the target modules and their complete
source-level transitive module dependencies. Its module/file map and resolved
named addresses are recorded in
[`framework/corpus-modules.json`](framework/corpus-modules.json). Modules other
than this sample's target are compilation context, not additional inference
targets.

Opaque/bodyless boundaries whose contracts are visible while proving this
target. This closure traverses transparent executable callees and behavioral
predicates referenced from reached contracts:

{called_text}

Transitive specification functions referenced by those boundary contracts:

{spec_text}

Transitive source modules required to compile the sample:

{module_text}

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

{removed_text}

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

{allowed}
"""
    _atomic_text(sample_dir / "README.md", readme)


def _write_corpus_catalog_readme(
    artifacts_root: Path,
    selected: list[dict[str, Any]],
    shared_metadata: dict[str, Any],
    source_commit: str,
) -> None:
    sample_rows = "\n".join(
        "| [`{task}`](samples/{task}/) | `{target}` | `{granularity}` | "
        "`{source}` |".format(
            task=record["task_id"],
            target=record["package_module_target"],
            granularity=record["granularity"],
            source=record["corpus_source_path"],
        )
        for record in sorted(selected, key=lambda item: item["task_id"])
    )
    readme = f"""# Move specification-inference corpus

This is the human-inspectable source catalog for the corpus prepared from Aptos
Core commit `{source_commit}`. Every experimental arm receives the same source
hash for a sample; treatment-specific skills and tools are stored separately.

## Metadata

- [`manifest.json`](manifest.json): the 30 prepared sample records and hashes;
  its `corpus_status` is authoritative for round readiness.
- [`metadata/candidate-inventory.json`](metadata/candidate-inventory.json): the
  complete compiler-AST source frame.
- [`metadata/selection.json`](metadata/selection.json): inclusion, exclusion,
  reserve, and replacement decisions.
- [`screening/ledger.json`](screening/ledger.json) and
  [`screening/results/`](screening/results/): compatibility evidence, valid for
  the current corpus only when its identity is recorded by `manifest.json`.

## Shared editable framework

[`framework/`](framework/) is the only Move package stored by the corpus. It
contains {shared_metadata['module_count']} modules and
{shared_metadata['move_file_count']} Move source/specification files: the union
of all targets and their source-level transitive dependencies. Named addresses,
original paths, and the exact module-to-file mapping are in
[`framework/corpus-modules.json`](framework/corpus-modules.json).

Every sample is a small overlay recipe. At run time the controller copies the
shared package and applies the sample's preparation patch, which removes only
that target's reference specification and adds its task descriptor. There are
no per-sample framework snapshots.

## Samples

Each sample README records provenance, dependency closure, address aliases,
preparation edits, allowed edit paths, required contract categories, and hashes.

| Sample | Target | Granularity | Target source in shared package |
| --- | --- | --- | --- |
{sample_rows}
"""
    _atomic_text(artifacts_root / "README.md", readme)


def _resolve_dependency_closure(
    repo_root: Path, root: Path
) -> tuple[dict[Path, dict[str, Any]], dict[Path, list[tuple[str, Path]]]]:
    packages: dict[Path, dict[str, Any]] = {}
    edges: dict[Path, list[tuple[str, Path]]] = {}
    pending = [root]
    while pending:
        package = pending.pop()
        if package in packages:
            continue
        if not package.is_relative_to(repo_root):
            raise ValueError(f"local dependency escapes pinned repository: {package}")
        manifest_path = package / "Move.toml"
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        if "package" not in manifest or "name" not in manifest["package"]:
            raise ValueError(f"Move manifest has no package name: {manifest_path}")
        packages[package] = manifest
        local_edges = []
        for section in ("dependencies", "dev-dependencies"):
            for dependency in manifest.get(section, {}).values():
                if not isinstance(dependency, dict) or "local" not in dependency:
                    continue
                original = str(dependency["local"])
                resolved = (package / original).resolve()
                if not (resolved / "Move.toml").is_file():
                    raise ValueError(
                        f"local dependency `{original}` has no Move.toml: {manifest_path}"
                    )
                local_edges.append((original, resolved))
                pending.append(resolved)
        edges[package] = local_edges
    return packages, edges


def _copy_ignore(_: str, names: list[str]) -> set[str]:
    ignored = {name for name in names if name in IGNORED_NAMES}
    ignored.update(name for name in names if name.startswith("test_") and name.endswith(".move"))
    return ignored


def _hardlink_tree(source: Path, destination: Path) -> None:
    shutil.copytree(source, destination, symlinks=True, copy_function=os.link)


def _remove_target_references(package: Path, record: dict[str, Any]) -> list[dict[str, Any]]:
    implementation = Path(record["corpus_source_path"])
    module_name = record["module"].rsplit("::", 1)[-1]
    sibling_spec = implementation.with_name(implementation.stem + ".spec.move")
    candidates = [sibling_spec]
    source_dir = package / implementation.parent
    if source_dir.is_dir():
        for path in sorted(source_dir.glob("*.spec.move")):
            if not _spec_declares_module(
                path.read_text(encoding="utf-8"), module_name
            ):
                continue
            relative = path.relative_to(package)
            if relative not in candidates:
                candidates.append(relative)
    for value in record["corpus_reference_paths"]:
        relative = Path(value)
        if relative not in candidates:
            candidates.append(relative)

    removed = []
    for function in record["target_functions"]:
        matches = []
        for relative in candidates:
            path = package / relative
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
            updated, count = _blank_spec_blocks(text, function)
            if count:
                _atomic_text(path, updated)
                matches.append({"path": relative.as_posix(), "function": function, "blocks": count})
        if not matches and record.get("reference_condition_count", 0) > 0:
            raise ValueError(
                f"could not locate explicit reference block for {record['task_id']}::{function}"
            )
        removed.extend(matches)
    return removed


def _spec_declares_module(text: str, module_name: str) -> bool:
    masked = mask_comments_and_strings(text)
    return re.search(
        rf"(?m)^\s*spec\s+(?:[A-Za-z_][A-Za-z0-9_]*|0x[0-9A-Fa-f]+)::"
        rf"{re.escape(module_name)}\s*\{{",
        masked,
    ) is not None


def _blank_spec_blocks(text: str, function: str) -> tuple[str, int]:
    masked = mask_comments_and_strings(text)
    pattern = re.compile(
        rf"(?m)^[ \t]*spec[ \t]+{re.escape(function)}\b(?![ \t]*:)",
    )
    spans = []
    for match in pattern.finditer(masked):
        brace = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if brace < 0 or (semicolon >= 0 and semicolon < brace):
            continue
        depth = 0
        end = None
        for index in range(brace, len(masked)):
            if masked[index] == "{":
                depth += 1
            elif masked[index] == "}":
                depth -= 1
                if depth == 0:
                    end = index + 1
                    break
        if end is None:
            raise ValueError(f"unterminated spec block for {function}")
        spans.append((match.start(), end))
    if not spans:
        return text, 0
    chars = list(text)
    for start, end in spans:
        for index in range(start, end):
            if chars[index] not in "\r\n":
                chars[index] = " "
    return "".join(chars), len(spans)



def _write_git_patch(pristine: Path, prepared: Path, output: Path) -> None:
    result = subprocess.run(
        [
            "git",
            "diff",
            "--no-index",
            "--binary",
            "--no-renames",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            str(pristine),
            str(prepared),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode not in (0, 1):
        raise ValueError(f"git diff failed: {result.stderr.strip()}")
    old_prefix = f"a/{pristine.as_posix().lstrip('/')}/"
    new_as_old_prefix = f"a/{prepared.as_posix().lstrip('/')}/"
    new_prefix = f"b/{prepared.as_posix().lstrip('/')}/"
    patch = (
        result.stdout.replace(old_prefix, "a/")
        .replace(new_as_old_prefix, "a/")
        .replace(new_prefix, "b/")
    )
    if not patch.strip():
        raise ValueError(f"empty preparation patch for {prepared.name}")
    _atomic_text(output, patch)


def _required_categories(record: dict[str, Any]) -> list[str]:
    features = set(record["feature_strata"])
    result = ["normal-result"]
    if "arithmetic-abort" in features:
        result.append("abort")
    if "global-state" in features or "mutable-reference" in features:
        result.append("state-transition")
    if "global-state" in features:
        result.append("frame")
    if "loop" in features:
        result.append("loop-invariant")
    return result


def _allowed_edit_paths(record: dict[str, Any]) -> list[str]:
    result = {record.get("corpus_source_path", _package_relative_source(record))}
    result.update(record.get("corpus_reference_paths", []))
    return sorted(result)


def _package_relative_source(record: dict[str, Any]) -> str:
    return Path(record["source_path"]).relative_to(record["source_root"]).as_posix()


def _source_commit(repo_root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        timeout=30,
        check=True,
    )
    return result.stdout.strip()


def _root_name(source_root: str) -> str:
    return Path(source_root).name


def _require_absent(path: Path) -> None:
    if path.exists():
        raise FileExistsError(f"refusing to overwrite generated artifact: {path}")


def _atomic_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(text, encoding="utf-8")
    temporary.replace(path)



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifacts-root", type=Path, required=True)
    parser.add_argument("--patches-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = prepare_corpus(
        args.provenance.resolve(),
        args.repo_root.resolve(),
        args.artifacts_root.resolve(),
        args.patches_dir.resolve(),
        args.output.resolve(),
    )
    print(
        json.dumps(
            {
                "patch_reproduction_verified": result["preparation"][
                    "patch_reproduction_verified"
                ],
                "tasks_prepared": result["preparation"]["tasks_prepared"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
