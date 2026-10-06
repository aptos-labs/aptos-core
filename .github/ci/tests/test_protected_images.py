import copy
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hypothesis import given, strategies as st

from tests.property_support import IDENTITIES, configure_profiles

configure_profiles()

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

    def test_size_limit_applies_to_valid_manifests(self):
        raw = json.dumps(self.manifest())
        images.validate(raw.ljust(images.MAX_BYTES), self.identity)
        with self.assertRaisesRegex(ActionError, "exceeds size limit"):
            images.validate(raw.ljust(images.MAX_BYTES + 1), self.identity)

    def test_non_finite_manifest_numbers_are_invalid(self):
        for value in (float("nan"), float("inf")):
            manifest = dict(self.manifest(), version=value)
            with self.subTest(value=value), self.assertRaisesRegex(
                    ActionError, "Invalid protected image manifest: JSON number must be finite"):
                images.validate(json.dumps(manifest), self.identity)

    def test_registry_inspect_output_is_parsed_strictly(self):
        digest = self.digest
        cases = (
            (json.dumps({"digest": digest}), None),
            ('{"digest": "%s", "digest": "%s"}' % (digest, digest),
             "Invalid registry inspect output: duplicate key: digest"),
            ('{"digest": NaN}', "Invalid registry inspect output: JSON number must be finite: NaN"),
        )
        for stdout, error in cases:
            completed = subprocess.CompletedProcess(args=[], returncode=0, stdout=stdout, stderr="")
            with self.subTest(stdout=stdout), mock.patch.object(images.subprocess, "run", return_value=completed):
                if error is None:
                    self.assertEqual(digest, images.registry_digest("repo/tools:tag"))
                else:
                    with self.assertRaisesRegex(ActionError, error):
                        images.registry_digest("repo/tools:tag")


# These are the handoff contract, independent of production IMAGE_NAMES.
REQUIRED_IMAGES = ("validator", "tools", "faucet", "forge", "telemetry-service",
                   "keyless-pepper-service", "indexer-grpc", "validator-testing", "nft-metadata-crawler")
VARIANT_PREFIXES = {"release": "", "performance": "performance_",
                    "consensus": "consensus_only_perf_test_", "failpoints": "failpoints_"}


def collected_fixture(identity, seed):
    digests = {name: "sha256:" + format(seed + i, "064x") for i, name in enumerate(REQUIRED_IMAGES)}
    controller = "sha256:" + format(seed + len(REQUIRED_IMAGES), "064x")
    tag = (f"pr-{identity['pr_number']}_{VARIANT_PREFIXES[identity['variant']]}"
           f"r{identity['run_id']}-a{identity['run_attempt']}_{identity['source_sha']}")
    controller_tag = f"controller-r{identity['run_id']}-a{identity['run_attempt']}_{identity['base_sha']}"
    expected_refs = {f"{identity['artifact_repo']}/{name}:{tag}": digest for name, digest in digests.items()}
    expected_refs[f"{identity['artifact_repo']}/forge:{controller_tag}"] = controller
    calls = []

    def resolve(reference):
        calls.append(reference)
        return expected_refs[reference]

    manifest = images.collect(identity, resolve)
    return manifest, digests, controller, tag, calls, list(expected_refs)


class ProtectedImagePropertyTests(unittest.TestCase):
    @given(IDENTITIES, st.integers(2, 999_998), st.integers(0, 2**128))
    def test_all_variants_resolve_exact_images_and_separate_controller(self, base, attempt, seed):
        for variant in VARIANT_PREFIXES:
            identity = {**base, "variant": variant, "run_attempt": str(attempt)}
            manifest, digests, controller, tag, calls, refs = collected_fixture(identity, seed)
            self.assertEqual(refs, calls)
            self.assertEqual({"version": 2, **identity, "image_tag": tag,
                              "images": digests, "controller_digest": controller}, manifest)
            for build_attempt in (attempt - 1, attempt, attempt + 1):
                producer = {**identity, "run_attempt": str(build_attempt)}
                candidate, expected_digests, trusted, built_tag, _, _ = collected_fixture(producer, seed)
                if build_attempt > attempt:
                    with self.assertRaises(ActionError):
                        images.validate(json.dumps(candidate), identity)
                else:
                    env = images.validate(json.dumps(candidate), identity)
                    repo = identity["artifact_repo"]
                    expected_map = {f"{repo}/{name}": digest for name, digest in expected_digests.items()}
                    self.assertEqual(expected_map, json.loads(env["PROTECTED_IMAGE_DIGESTS"]))
                    self.assertEqual(json.dumps(expected_map, separators=(",", ":"), sort_keys=True),
                                     env["PROTECTED_IMAGE_DIGESTS"])
                    self.assertEqual(built_tag, env["PR_IMAGE_TAG"])
                    self.assertEqual(built_tag, env["PROTECTED_IMAGE_TAG"])
                    self.assertEqual(f"{repo}/tools@{expected_digests['tools']}", env["PROTECTED_TOOLS_IMAGE"])
                    self.assertEqual(f"{repo}/forge@{trusted}", env["PROTECTED_FORGE_IMAGE"])
                    self.assertNotEqual(expected_digests["forge"], trusted)

    @given(IDENTITIES, st.integers(0, 2**128))
    def test_identity_substitution_fails_for_each_bound_field(self, base, seed):
        for variant in VARIANT_PREFIXES:
            identity = {**base, "variant": variant}
            replacements = {
                "source_repository": base["source_repository"] + "x",
                "source_sha": ("1" if base["source_sha"][0] == "0" else "0") + base["source_sha"][1:],
                "base_sha": ("1" if base["base_sha"][0] == "0" else "0") + base["base_sha"][1:],
                "pr_number": "2" if base["pr_number"] == "1" else "1",
                "run_id": "2" if base["run_id"] == "1" else "1",
                "variant": "performance" if variant == "release" else "release",
                "artifact_repo": "eu-docker.pkg.dev/other-project/other-repo",
            }
            for key, value in replacements.items():
                # Collect a self-consistent substitute so rejection must bind the consumer identity.
                candidate, _, _, _, _, _ = collected_fixture({**identity, key: value}, seed)
                with self.assertRaises(ActionError):
                    images.validate(json.dumps(candidate), identity)

    @given(IDENTITIES, st.integers(0, 2**128))
    def test_schema_types_duplicate_keys_and_exact_image_set_fail_closed(self, identity, seed):
        manifest, _, _, _, _, _ = collected_fixture(identity, seed)
        candidates = []
        for key in manifest:
            missing = copy.deepcopy(manifest)
            del missing[key]
            candidates.append(missing)
        candidates.append({**manifest, "extra": True})
        for bad in (True, "2", 1, 2.0, None):
            candidates.append({**manifest, "version": bad})
        for key in identity:
            for bad in (True, 1, None, [], {}):
                candidates.append({**manifest, key: bad})
        for name in REQUIRED_IMAGES:
            missing = copy.deepcopy(manifest)
            del missing["images"][name]
            candidates.append(missing)
            for bad in (None, True, 1, [], {}, "latest", "sha256:" + "A" * 64,
                        "sha256:" + "a" * 63, "sha256:" + "a" * 65, "sha256:" + "a" * 64 + "\n"):
                invalid = copy.deepcopy(manifest)
                invalid["images"][name] = bad
                candidates.append(invalid)
        candidates.extend([{**manifest, "images": {**manifest["images"], "extra": "sha256:" + "0" * 64}},
                           {**manifest, "images": []}, {**manifest, "image_tag": "baseline"}])
        for bad in (None, True, 1, "latest", "sha256:" + "a" * 63):
            candidates.append({**manifest, "controller_digest": bad})
        for candidate in candidates:
            with self.assertRaises(ActionError):
                images.validate(json.dumps(candidate), identity)
        raw = json.dumps(manifest)
        for key, value in (("version", 2), ("tools", manifest["images"]["tools"])):
            token = json.dumps(key) + ": " + json.dumps(value)
            duplicate = raw.replace(token, token + ", " + token, 1)
            with self.assertRaises(ActionError):
                images.validate(duplicate, identity)
        for raw in ("null", "[]", "true", "1", "{broken"):
            with self.assertRaises(ActionError):
                images.validate(raw, identity)

    @given(IDENTITIES, st.integers(0, 9))
    def test_invalid_resolver_digest_never_completes_collection(self, identity, failure_index):
        for invalid in (None, True, 1, {}, "latest", "sha256:" + "A" * 64,
                        "sha256:" + "a" * 63, "sha256:" + "a" * 64 + "\n"):
            calls = []

            def resolve(reference):
                calls.append(reference)
                return invalid if len(calls) - 1 == failure_index else "sha256:" + "a" * 64

            with self.assertRaises(ActionError):
                images.collect(identity, resolve)
            self.assertEqual(failure_index + 1, len(calls))


if __name__ == "__main__":
    unittest.main()
