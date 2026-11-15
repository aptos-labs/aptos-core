from pathlib import Path
import subprocess
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]


class CacheContractTests(unittest.TestCase):
    def test_dependency_setup_preserves_toolchains(self):
        source = (ROOT / "scripts/dev_setup.sh").read_text()
        function = source.split("function install_rustup_components_and_nightly {", 1)[
            1
        ].split("function install_cargo_sort", 1)[0]
        result = subprocess.run(
            [
                "bash",
                "-c",
                "rustup(){ printf '%s\\n' \"$*\"; }; "
                "APTOS_PRESERVE_TOOLCHAINS=true; f(){" + function + "\nf",
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "show active-toolchain")

    def test_real_cache_service_uses_two_fresh_jobs_and_same_identity(self):
        workflow = yaml.safe_load(
            (ROOT / ".github/workflows/validate-rust-cache.yaml").read_text()
        )
        self.assertEqual(workflow["jobs"]["warm"]["needs"], "cold")
        caches = []
        for job in ("cold", "warm"):
            caches.append(
                next(
                    s for s in workflow["jobs"][job]["steps"] if s.get("id") == "cache"
                )
            )
        self.assertEqual(
            caches[0]["with"]["shared-key"], caches[1]["with"]["shared-key"]
        )
        self.assertEqual(caches[0]["with"]["env-vars"], "APTOS_BUILD_ENV")
        self.assertEqual(caches[1]["with"]["save-if"], "false")

    def test_docker_builder_invokes_source_and_abi_guard_in_all_cache_mounts(self):
        dockerfile = (ROOT / "docker/builder/builder.Dockerfile").read_text()
        self.assertEqual(
            dockerfile.count("source docker/builder/refresh-source-cache.sh &&"), 4
        )
        guard = (ROOT / "docker/builder/refresh-source-cache.sh").read_text()
        self.assertIn("export CARGO_TARGET_DIR=", guard)
        self.assertIn("abi-${abi}", guard)
        self.assertNotIn("rm -", guard)


if __name__ == "__main__":
    unittest.main()
