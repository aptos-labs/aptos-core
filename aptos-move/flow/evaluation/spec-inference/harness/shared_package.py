"""Build the corpus's single editable Move package from a module union."""

from __future__ import annotations

import shutil
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any
import re

from .artifacts import sha256_file, tree_hash, write_json
from .move_source import mask_comments_and_strings


SOURCE_PACKAGES = (
    "aptos-move/framework/move-stdlib",
    "aptos-move/framework/aptos-stdlib",
    "aptos-move/framework/aptos-framework",
    "aptos-move/framework/aptos-trading",
    "aptos-move/framework/aptos-experimental",
)

_MODULE = re.compile(
    r"(?m)^\s*module\s+([A-Za-z_][A-Za-z0-9_]*|0x[0-9A-Fa-f]+)::"
    r"([A-Za-z_][A-Za-z0-9_]*)"
)
_SPEC_MODULE = re.compile(
    r"(?m)^\s*spec\s+([A-Za-z_][A-Za-z0-9_]*|0x[0-9A-Fa-f]+)::"
    r"([A-Za-z_][A-Za-z0-9_]*)\s*\{"
)

# Companion specifications are separate source files in the flattened corpus
# package, so file-local aliases from their original implementation file are
# not available.  Qualify the small number of upstream references which rely
# on those aliases.  This changes specifications only, never executable Move.
_STANDALONE_SPEC_QUALIFICATIONS = {
    "sources/AptosFramework/dispatchable_fungible_asset.spec.move": (
        ("object::object_address", "aptos_framework::object::object_address"),
    ),
    "sources/AptosFramework/function_info.spec.move": (
        ("string::bytes", "std::string::bytes"),
    ),
}


@dataclass(frozen=True)
class SourceFile:
    package_path: Path
    package_name: str
    path: Path

    def corpus_path(self) -> Path:
        return Path("sources") / self.package_name / self.path.relative_to(
            self.package_path / "sources"
        )


def build_shared_package(
    repo_root: Path,
    selected: list[dict[str, Any]],
    destination: Path,
) -> dict[str, Any]:
    """Copy exactly the selected targets and their module closure into one package."""
    if destination.exists():
        raise FileExistsError(f"refusing to overwrite shared package: {destination}")
    modules = {record["module"] for record in selected}
    for record in selected:
        modules.update(record["transitive_module_dependencies"])

    implementations, specifications, aliases, package_records = _index_sources(
        repo_root
    )
    missing = sorted(modules - implementations.keys())
    if missing:
        raise ValueError(f"module closure has no source files: {missing}")

    destination.mkdir(parents=True)
    source_map: dict[str, str] = {}
    module_records = []
    copied: set[Path] = set()
    for module in sorted(modules):
        implementation = implementations[module]
        files = [implementation, *specifications.get(module, [])]
        corpus_files = []
        for source in files:
            target = destination / source.corpus_path()
            if target not in copied:
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source.path, target)
                copied.add(target)
            original = source.path.relative_to(repo_root).as_posix()
            relative = source.corpus_path().as_posix()
            source_map[original] = relative
            corpus_files.append(relative)
        module_records.append(
            {
                "module": module,
                "implementation": implementation.corpus_path().as_posix(),
                "specifications": corpus_files[1:],
            }
        )

    standalone_spec_qualifications = normalize_standalone_specs(destination)

    manifest = [
        "[package]",
        'name = "InferenceCorpusFramework"',
        'version = "1.0.0"',
        "",
        "[addresses]",
    ]
    manifest.extend(f'{name} = "{value}"' for name, value in sorted(aliases.items()))
    (destination / "Move.toml").write_text("\n".join(manifest) + "\n", encoding="utf-8")
    prover = repo_root / "aptos-move/framework/aptos-framework/Prover.toml"
    if prover.is_file():
        shutil.copy2(prover, destination / "Prover.toml")

    closure = {
        "schema_version": 1,
        "kind": "shared_editable_module_union",
        "modules": module_records,
        "source_packages": package_records,
        "resolved_address_aliases": dict(sorted(aliases.items())),
        "source_path_map": dict(sorted(source_map.items())),
        "standalone_spec_qualifications": standalone_spec_qualifications,
    }
    write_json(destination / "corpus-modules.json", closure)
    closure["tree_sha256"] = tree_hash(destination)
    closure["module_count"] = len(module_records)
    closure["move_file_count"] = len(copied)
    return closure


def normalize_standalone_specs(package: Path) -> list[dict[str, Any]]:
    """Fully qualify upstream spec references which depended on file aliases."""
    changed = []
    for relative, replacements in _STANDALONE_SPEC_QUALIFICATIONS.items():
        path = package / relative
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        original = text
        applied = []
        for shorthand, qualified in replacements:
            # Idempotence matters when dependency specs are reconstructed in
            # place after a failed generation pass.
            pattern = re.compile(rf"(?<![:A-Za-z0-9_]){re.escape(shorthand)}")
            text, count = pattern.subn(qualified, text)
            if count:
                applied.append(
                    {
                        "from": shorthand,
                        "to": qualified,
                        "count": count,
                    }
                )
        if text != original:
            path.write_text(text, encoding="utf-8")
            changed.append({"path": relative, "replacements": applied})
    return changed


def _index_sources(
    repo_root: Path,
) -> tuple[
    dict[str, SourceFile],
    dict[str, list[SourceFile]],
    dict[str, str],
    list[dict[str, Any]],
]:
    implementations: dict[str, SourceFile] = {}
    specifications: dict[str, list[SourceFile]] = {}
    aliases: dict[str, str] = {}
    packages = []
    for relative in SOURCE_PACKAGES:
        package = (repo_root / relative).resolve()
        manifest_path = package / "Move.toml"
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        package_name = str(manifest["package"]["name"])
        package_aliases = {
            str(name): str(value)
            for name, value in manifest.get("addresses", {}).items()
            if str(value) != "_"
        }
        for name, value in package_aliases.items():
            previous = aliases.get(name)
            if previous is not None and previous.lower() != value.lower():
                raise ValueError(
                    f"address alias `{name}` conflicts: {previous} versus {value}"
                )
            aliases[name] = value
        packages.append(
            {
                "package": package_name,
                "source_path": relative,
                "source_manifest_sha256": sha256_file(manifest_path),
            }
        )
        for path in sorted((package / "sources").rglob("*.move")):
            source = SourceFile(package, package_name, path)
            masked = mask_comments_and_strings(path.read_text(encoding="utf-8"))
            for match in _MODULE.finditer(masked):
                module = _module_id(match.group(1), match.group(2), package_aliases)
                previous = implementations.get(module)
                if previous is not None and previous.path != path:
                    raise ValueError(
                        f"module `{module}` has multiple sources: "
                        f"{previous.path} and {path}"
                    )
                implementations[module] = source
            for match in _SPEC_MODULE.finditer(masked):
                module = _module_id(match.group(1), match.group(2), package_aliases)
                values = specifications.setdefault(module, [])
                if source not in values:
                    values.append(source)
    return implementations, specifications, aliases, packages


def _module_id(address: str, name: str, aliases: dict[str, str]) -> str:
    value = aliases.get(address, address).lower()
    if value.startswith("0x"):
        value = "0x" + (value[2:].lstrip("0") or "0")
    return f"{value}::{name}"
