import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from package_hosted_shards import package, runtime_path
from test_verify_hosted_shards import inventory


class PackageTests(unittest.TestCase):
    def test_runtime_path_requires_target_prefix(self):
        self.assertEqual(
            runtime_path("/build/target/ci/test", "/build/target"),
            Path("target/ci/test"),
        )
        with self.assertRaises(ValueError):
            runtime_path("/outside/test", "/build/target")

    @unittest.skipUnless(
        shutil.which("zstd"), "zstd is required for archive contract tests"
    )
    def test_keeps_helpers_and_trims_only_unselected_tests(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "source"
            (source / "target/nextest").mkdir(parents=True)
            (source / "target/ci").mkdir()
            for name in ["a", "b", "helper"]:
                (source / f"target/ci/{name}").write_text(name)
            metadata = {
                "rust-build-meta": {"target-directory": "/build/target"},
                "rust-binaries": {
                    name: {"binary-path": f"/build/target/ci/{name}"}
                    for name in ["a", "b"]
                },
            }
            (source / "target/nextest/binaries-metadata.json").write_text(
                json.dumps(metadata)
            )
            (source / "target/nextest/cargo-metadata.json").write_text("{}")
            archive = root / "all.tar.zst"
            subprocess.run(
                ["tar", "--zstd", "-cf", str(archive), "-C", str(source), "target"],
                check=True,
            )
            paths = []
            for index, name in enumerate(["a", "b"], 1):
                data = inventory(["test"])
                data["rust-suites"][name] = data["rust-suites"].pop("binary")
                path = root / f"inventory-{index}.json"
                path.write_text(json.dumps(data))
                paths.append(path)
            output = root / "bundles"
            sizes = package(archive, paths, output, "false")
            self.assertTrue(all(size > 0 for size in sizes))
            for index, name in enumerate(["a", "b"], 1):
                extracted = root / f"extracted-{index}"
                extracted.mkdir()
                subprocess.run(
                    [
                        "tar",
                        "--zstd",
                        "-xf",
                        str(output / f"unit-tests-{index}.tar.zst"),
                        "-C",
                        str(extracted),
                    ],
                    check=True,
                )
                self.assertTrue((extracted / f"target/ci/{name}").exists())
                self.assertFalse(
                    (extracted / f'target/ci/{"b" if name == "a" else "a"}').exists()
                )
                self.assertTrue((extracted / "target/ci/helper").exists())
                trimmed = json.loads(
                    (extracted / "target/nextest/binaries-metadata.json").read_text()
                )
                self.assertEqual(set(trimmed["rust-binaries"]), {name})
            self.assertEqual(
                json.loads(
                    (source / "target/nextest/binaries-metadata.json").read_text()
                ),
                metadata,
            )


if __name__ == "__main__":
    unittest.main()
