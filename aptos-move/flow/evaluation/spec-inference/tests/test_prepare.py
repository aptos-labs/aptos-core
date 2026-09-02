from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness.prepare import (
    _blank_spec_blocks,
    _copy_standalone_package,
    _remove_target_references,
    _write_git_patch,
    _write_sample_catalog,
)


class PrepareTests(unittest.TestCase):
    def test_remove_target_finds_generated_experimental_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            source_dir = package / "sources/AptosExperimental/order_book"
            source_dir.mkdir(parents=True)
            (source_dir / "book.move").write_text(
                "module aptos_experimental::book {}\n", encoding="utf-8"
            )
            generated = source_dir / "book.spec.move"
            generated.write_text(
                """spec aptos_experimental::book {
    spec target() { pragma opaque = true; ensures true; }
}
""",
                encoding="utf-8",
            )
            removed = _remove_target_references(
                package,
                {
                    "task_id": "AX-book-001",
                    "module": "0x7::book",
                    "corpus_source_path": (
                        "sources/AptosExperimental/order_book/book.move"
                    ),
                    "corpus_reference_paths": [],
                    "reference_condition_count": 0,
                    "target_functions": ["target"],
                },
            )
            self.assertEqual(removed[0]["path"], generated.relative_to(package).as_posix())
            self.assertNotIn("pragma opaque", generated.read_text(encoding="utf-8"))

    def test_blank_spec_blocks_preserves_lines_and_other_specs(self) -> None:
        source = """spec sample::m {
    spec target(x: u64) {
        ensures result == x;
    }

    spec other() {
        ensures true;
    }
}
"""
        prepared, count = _blank_spec_blocks(source, "target")
        self.assertEqual(count, 1)
        self.assertEqual(source.count("\n"), prepared.count("\n"))
        self.assertNotIn("ensures result == x", prepared)
        self.assertIn("spec other()", prepared)

    def test_copy_resolves_transitive_local_dependencies_and_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary) / "repo"
            root = repo / "packages/root"
            dependency = repo / "packages/dependency"
            leaf = repo / "packages/leaf"
            for package in (root, dependency, leaf):
                (package / "sources").mkdir(parents=True)
                (package / "sources/module.move").write_text(
                    "module 0x1::module {}\n", encoding="utf-8"
                )
            (root / "Move.toml").write_text(
                """[package]
name = "Root"
[addresses]
root = "0x2"
[dependencies]
Dependency = { local = "../dependency" }
""",
                encoding="utf-8",
            )
            (dependency / "Move.toml").write_text(
                """[package]
name = "Dependency"
[addresses]
dependency = "0x3"
[dependencies]
Leaf = { local = "../leaf" }
""",
                encoding="utf-8",
            )
            (leaf / "Move.toml").write_text(
                """[package]
name = "Leaf"
[addresses]
leaf = "0x4"
""",
                encoding="utf-8",
            )
            destination = Path(temporary) / "snapshot"
            metadata = _copy_standalone_package(
                repo.resolve(), "packages/root", destination
            )
            self.assertEqual(
                [record["package"] for record in metadata["packages"]],
                ["Dependency", "Leaf", "Root"],
            )
            self.assertEqual(
                metadata["resolved_address_aliases"],
                {"dependency": "0x3", "leaf": "0x4", "root": "0x2"},
            )
            self.assertIn(
                'local = "deps/Dependency"',
                (destination / "Move.toml").read_text(encoding="utf-8"),
            )
            self.assertIn(
                'local = "../Leaf"',
                (destination / "deps/Dependency/Move.toml").read_text(
                    encoding="utf-8"
                ),
            )

    def test_shared_package_store_deduplicates_closures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary) / "repo"
            dependency = repo / "dependency"
            roots = [repo / "root-a", repo / "root-b"]
            for package in [dependency, *roots]:
                (package / "sources").mkdir(parents=True)
                (package / "sources/module.move").write_text(
                    "module 0x1::module {}\n", encoding="utf-8"
                )
            (dependency / "Move.toml").write_text(
                '[package]\nname = "Dependency"\n', encoding="utf-8"
            )
            for index, root in enumerate(roots):
                (root / "Move.toml").write_text(
                    f'''[package]
name = "Root{index}"
[dependencies]
Dependency = {{ local = "../dependency" }}
''',
                    encoding="utf-8",
                )

            package_store = Path(temporary) / "corpus-v1/packages"
            shared_packages: dict[str, Path] = {}
            destinations = []
            for index, root in enumerate(roots):
                destination = Path(temporary) / f"snapshot-{index}"
                _copy_standalone_package(
                    repo.resolve(),
                    root.relative_to(repo).as_posix(),
                    destination,
                    package_store=package_store,
                    shared_packages=shared_packages,
                )
                destinations.append(destination)

            self.assertEqual(
                sorted(path.name for path in package_store.iterdir()),
                ["Dependency", "Root0", "Root1"],
            )
            shared_source = package_store / "Dependency/sources/module.move"
            for destination in destinations:
                self.assertTrue(
                    shared_source.samefile(
                        destination / "deps/Dependency/sources/module.move"
                    )
                )

    def test_sample_catalog_explains_source_and_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prepared = root / "snapshots/task"
            prepared.mkdir(parents=True)
            (prepared / "Move.toml").write_text("[package]\nname='Root'\n")
            patch = root / "patches/task.patch"
            patch.parent.mkdir()
            patch.write_text("patch\n")
            record = {
                "task_id": "task",
                "package_module_target": "Root::module::target",
                "granularity": "function",
                "source_path": "packages/root/sources/module.move",
                "source_root": "packages/root",
                "corpus_source_path": "sources/Root/module.move",
                "shared_package_sha256": "c" * 64,
                "prepared_sha256": "a" * 64,
                "required_contract_categories": ["normal-result"],
                "target_functions": ["target"],
                "called_function_dependencies": ["0x1::dep::boundary"],
                "spec_function_dependencies": ["0x1::dep_specs::models_boundary"],
                "transitive_module_dependencies": ["0x1::dep"],
                "dependency_closure": [
                    {
                        "package": "Root",
                        "source_path": "packages/root",
                        "snapshot_path": ".",
                    }
                ],
                "resolved_address_aliases": {"root": "0x1"},
                "removed_reference_blocks": [],
                "allowed_edit_paths": ["sources/module.move"],
            }
            sample = root / "samples/task"
            _write_sample_catalog(sample, prepared, patch, record, "b" * 40)

            readme = (sample / "README.md").read_text()
            self.assertIn("Root::module::target", readme)
            self.assertIn("packages/root", readme)
            self.assertIn("0x1::dep::boundary", readme)
            self.assertIn("0x1::dep_specs::models_boundary", readme)
            self.assertIn("0x1::dep", readme)
            self.assertIn("resolved named addresses", " ".join(readme.split()))
            self.assertEqual((sample / "framework").resolve(), prepared.resolve())
            self.assertEqual((sample / "preparation.patch").resolve(), patch.resolve())

    def test_patch_header_does_not_leak_construction_path_for_new_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            pristine = root / "pristine"
            prepared = root / "snapshots/task"
            pristine.mkdir()
            prepared.mkdir(parents=True)
            (pristine / "Move.toml").write_text("[package]\nname='Root'\n")
            (prepared / "Move.toml").write_text("[package]\nname='Root'\n")
            (prepared / ".move-inference-task.json").write_text("{}\n")
            patch = root / "task.patch"

            _write_git_patch(pristine, prepared, patch)

            text = patch.read_text()
            self.assertIn(
                "diff --git a/.move-inference-task.json "
                "b/.move-inference-task.json",
                text,
            )
            self.assertNotIn(str(root), text)


if __name__ == "__main__":
    unittest.main()
