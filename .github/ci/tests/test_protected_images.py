import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from ci_actions.github import ActionError
from ci_actions import protected_images as images


class ProtectedImagesTests(unittest.TestCase):
    def setUp(self):
        self.identity = dict(source_repository="contributor/aptos-core", source_sha="a" * 40, base_sha="d" * 40,
                             pr_number="42", run_id="123", run_attempt="2", variant="release",
                             artifact_repo="us-docker.pkg.dev/aptos-registry/docker")
        self.digest = "sha256:" + "b" * 64

    def manifest(self):
        return images.collect(self.identity, lambda ref: self.digest)

    def test_collect_resolves_every_required_image_and_scopes_tag_to_attempt(self):
        refs = []
        manifest = images.collect(self.identity, lambda ref: refs.append(ref) or self.digest)
        self.assertEqual(len(images.IMAGE_NAMES) + 1, len(refs))
        self.assertTrue(all(ref.endswith(":pr-42_r123-a2_" + "a" * 40) for ref in refs[:-1]))
        self.assertEqual(self.identity["artifact_repo"] + "/forge:controller-r123-a2_" + "d" * 40, refs[-1])
        self.assertEqual(set(images.IMAGE_NAMES), set(manifest["images"]))

    def test_validate_exports_digest_map_and_tools_reference(self):
        env = images.validate(json.dumps(self.manifest()), self.identity)
        self.assertEqual(self.identity["artifact_repo"] + "/tools@" + self.digest, env["PROTECTED_TOOLS_IMAGE"])
        self.assertEqual(env["PR_IMAGE_TAG"], env["PROTECTED_IMAGE_TAG"])
        self.assertEqual(self.digest, json.loads(env["PROTECTED_IMAGE_DIGESTS"])[self.identity["artifact_repo"] + "/forge"])
        self.assertEqual(self.identity["artifact_repo"] + "/forge@" + self.digest, env["PROTECTED_FORGE_IMAGE"])

    def test_controller_is_separate_from_the_pr_forge_image(self):
        manifest = self.manifest()
        controller_digest = "sha256:" + "e" * 64
        manifest["controller_digest"] = controller_digest
        env = images.validate(json.dumps(manifest), self.identity)
        self.assertEqual(self.identity["artifact_repo"] + "/forge@" + controller_digest, env["PROTECTED_FORGE_IMAGE"])
        self.assertEqual(self.digest, json.loads(env["PROTECTED_IMAGE_DIGESTS"])[self.identity["artifact_repo"] + "/forge"])
        for change in ({"base_sha": "e" * 40}, {"controller_digest": "latest"}, {"version": 1}):
            with self.subTest(change=change), self.assertRaises(ActionError):
                images.validate(json.dumps(dict(manifest, **change)), self.identity)
        del manifest["controller_digest"]
        with self.assertRaises(ActionError):
            images.validate(json.dumps(manifest), self.identity)

    def test_successful_build_from_earlier_attempt_can_be_reused_on_test_rerun(self):
        manifest = self.manifest()
        current = dict(self.identity, run_attempt="3")
        self.assertEqual(manifest["image_tag"], images.validate(json.dumps(manifest), current)["PR_IMAGE_TAG"])

    def test_identity_substitution_and_future_attempt_are_rejected(self):
        for key, value in {"source_sha": "c" * 40, "source_repository": "other/repo", "pr_number": "43",
                           "variant": "performance", "run_id": "124", "artifact_repo": "other.example/repo",
                           "run_attempt": "3"}.items():
            with self.subTest(key=key):
                manifest = self.manifest()
                manifest[key] = value
                with self.assertRaises(ActionError):
                    images.validate(json.dumps(manifest), self.identity)

    def test_missing_malformed_extra_and_duplicate_images_fail_closed(self):
        manifest = self.manifest()
        for mutate in (lambda m: m["images"].pop("forge"),
                       lambda m: m["images"].update(tools="latest"),
                       lambda m: m["images"].update(evil=self.digest),
                       lambda m: m.update(image_tag="main")):
            candidate = copy.deepcopy(manifest)
            mutate(candidate)
            with self.assertRaises(ActionError):
                images.validate(json.dumps(candidate), self.identity)
        raw = json.dumps(manifest).replace('"version": 2', '"version": 2, "version": 2')
        with self.assertRaises(ActionError):
            images.validate(raw, self.identity)

    def test_invalid_registry_digest_never_produces_a_manifest(self):
        with self.assertRaises(ActionError):
            images.collect(self.identity, lambda ref: "latest")

    def test_profiles_and_features_are_bound_to_variant(self):
        for variant, prefix in [("performance", "performance_"), ("consensus", "consensus_only_perf_test_"),
                                ("failpoints", "failpoints_")]:
            identity = dict(self.identity, variant=variant)
            manifest = images.collect(identity, lambda ref: self.digest)
            self.assertEqual(f"pr-42_{prefix}r123-a2_" + "a" * 40, manifest["image_tag"])
            images.validate(json.dumps(manifest), identity)

    def test_verify_command_writes_only_validated_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            output = Path(directory) / "env"
            env = dict(os.environ, INPUT_MODE="verify", INPUT_MANIFEST_PATH=str(path), GITHUB_ENV=str(output))
            env.update({"INPUT_" + key.upper(): value for key, value in self.identity.items()})
            launcher = Path(__file__).resolve().parents[1] / "run_action.py"
            path.write_text(json.dumps(self.manifest()))
            command = [sys.executable, "-I", str(launcher), "protected-image-manifest"]
            result = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(0, result.returncode, result.stderr)
            self.assertIn("PROTECTED_TOOLS_IMAGE=", output.read_text())
            output.unlink()
            manifest = self.manifest()
            manifest["source_sha"] = "c" * 40
            path.write_text(json.dumps(manifest))
            result = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertNotEqual(0, result.returncode)
            self.assertFalse(output.exists())

    def test_size_and_boolean_version_are_rejected(self):
        with self.assertRaises(ActionError):
            images.validate(" " * (images.MAX_BYTES + 1), self.identity)
        manifest = self.manifest()
        manifest["version"] = True
        with self.assertRaises(ActionError):
            images.validate(json.dumps(manifest), self.identity)


if __name__ == "__main__":
    unittest.main()
