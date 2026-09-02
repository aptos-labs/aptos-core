"""Add a human-inspectable catalog to an already prepared corpus."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
from typing import Any

from .artifacts import load_object, write_json
from .prepare import (
    _hardlink_tree,
    _require_absent,
    _root_name,
    _write_corpus_catalog_readme,
    _write_sample_catalog,
)


def build_catalog(corpus_path: Path, output_path: Path) -> dict[str, Any]:
    corpus = load_object(corpus_path)
    preparation = corpus.get("preparation", {})
    artifacts_root_value = preparation.get("artifacts_root")
    if not artifacts_root_value:
        raise ValueError("corpus has no preparation artifacts root")
    artifacts_root = (corpus_path.parent / artifacts_root_value).resolve()
    package_store = artifacts_root / "packages"
    samples_root = artifacts_root / "samples"
    _require_absent(package_store)
    _require_absent(samples_root)

    selected = [
        record
        for record in corpus["records"]
        if record["selection_status"] == "selected"
    ]
    if len(selected) != 30:
        raise ValueError(f"expected 30 selected records, got {len(selected)}")

    package_sources: dict[str, tuple[str, Path]] = {}
    dependency_closures: dict[str, dict[str, Any]] = {}
    for record in selected:
        source_root = record["source_root"]
        closure = record["dependency_closure"]
        dependency_closures.setdefault(source_root, {"packages": closure})
        pristine = artifacts_root / "pristine" / _root_name(source_root)
        for package in closure:
            name = package["package"]
            source_path = package["source_path"]
            source = pristine / package["snapshot_path"]
            previous = package_sources.get(name)
            if previous is not None:
                if previous[0] != source_path:
                    raise ValueError(
                        f"package `{name}` maps to both {previous[0]} and {source_path}"
                    )
                continue
            package_sources[name] = (source_path, source)

    for name, (_, source) in sorted(package_sources.items()):
        if not (source / "Move.toml").is_file():
            raise ValueError(f"shared package source has no Move.toml: {source}")
        _hardlink_tree(source, package_store / name)

    for record in selected:
        prepared = (corpus_path.parent / record["prepared_path"]).resolve()
        patch_path = (corpus_path.parent / record["preparation_patch"]).resolve()
        sample_dir = samples_root / record["task_id"]
        _write_sample_catalog(
            sample_dir,
            prepared,
            patch_path,
            record,
            corpus["source_commit"],
        )
        record["sample_path"] = os.path.relpath(sample_dir, output_path.parent)
        record["sample_readme"] = os.path.relpath(
            sample_dir / "README.md", output_path.parent
        )

    _write_corpus_catalog_readme(
        artifacts_root,
        selected,
        dependency_closures,
        corpus["source_commit"],
    )
    corpus["preparation"] = {
        **preparation,
        "package_store": os.path.relpath(package_store, output_path.parent),
        "sample_catalog": os.path.relpath(samples_root, output_path.parent),
        "shared_packages": sorted(package_sources),
    }
    corpus.pop("freeze_status", None)
    corpus["corpus_status"] = "screened"
    write_json(output_path, corpus)
    return corpus


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = build_catalog(args.corpus.resolve(), args.output.resolve())
    print(
        f"cataloged {result['preparation']['tasks_prepared']} samples under "
        f"{result['preparation']['sample_catalog']}"
    )


if __name__ == "__main__":
    main()
