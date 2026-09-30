import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import call, patch

from hypothesis import given, strategies as st

from ci_actions.github import ActionError
from ci_actions import image_archives as archives
from tests.property_support import IDENTITIES, SAFE_COMPONENT, configure_profiles

configure_profiles()

BUILD_IMAGES = ("validator", "tools", "faucet", "forge", "telemetry-service",
                "keyless-pepper-service", "indexer-grpc", "validator-testing", "nft-metadata-crawler")
E2E_IMAGES = ("pr", "devnet", "testnet", "mainnet")
METADATA_NAME = "image-archives.json"
E2E_ENV = "PR_IMAGE_TAG=pr\nGCP_DOCKER_ARTIFACT_REPO=aptos-ci\nAPTOS_E2E_OFFLINE_IMAGES=true\n"


def bundle_files(mode):
    if mode == "build":
        return {name: f"{name}.tar" for name in BUILD_IMAGES}
    return {name: f"tools-{name}.tar" for name in E2E_IMAGES}


def local_tag(identity, name, mode):
    if mode == "build":
        return f"aptos-core/{name}:{identity['source_sha']}-from-local"
    return f"aptos-ci/tools:{name}"


def release_tag(identity):
    return f"pr-{identity['pr_number']}_r{identity['run_id']}-a{identity['run_attempt']}_{identity['source_sha']}"


def write_metadata(root, metadata):
    (root / METADATA_NAME).write_text(json.dumps(metadata))


def make_bundle(root, identity, mode="build"):
    root.mkdir()
    hashes = {}
    for name, filename in bundle_files(mode).items():
        path = root / filename
        docker_archive(path, local_tag(identity, name, mode))
        hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    metadata = {"version": 1, "type": mode, "identity": dict(identity), "archives": hashes}
    write_metadata(root, metadata)
    return metadata


def docker_archive(path: Path, tag: str, *, extra=None, manifest=None, manifest_bytes=None, omit=()):
    record = manifest if manifest is not None else {
        "Config": "config.json", "RepoTags": [tag], "Layers": ["layer/layer.tar"]}
    raw = json.dumps([record]).encode() if manifest_bytes is None else manifest_bytes
    content = {"manifest.json": raw, "config.json": b"{}",
               "layer/layer.tar": b"layer"}
    with tarfile.open(path, "w") as tar:
        for name, data in content.items():
            if name in omit:
                continue
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
        if extra:
            tar.addfile(extra)


class ImageArchiveTests(unittest.TestCase):
    def setUp(self):
        self.identity = dict(source_repository="contributor/aptos-core", source_sha="a" * 40,
                             base_sha="b" * 40, pr_number="42", run_id="123", run_attempt="2",
                             variant="release", artifact_repo="us-docker.pkg.dev/aptos-registry/docker")

    def make_bundle(self, root: Path, mode="build"):
        return make_bundle(root, self.identity, mode)

    def test_validated_publish_uses_fixed_tags_and_skopeo_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            with patch.object(archives.subprocess, "run") as run:
                archives.publish_archives(root, self.identity)
            self.assertEqual([
                call(["skopeo", "copy", f"docker-archive:{root / filename}:{local_tag(self.identity, name, 'build')}",
                      f"docker://{self.identity['artifact_repo']}/{name}:{release_tag(self.identity)}"],
                     check=True, timeout=1800)
                for name, filename in bundle_files("build").items()
            ], run.call_args_list)

    def test_export_build_saves_fixed_local_references(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            commands = []

            def fake_run(command, **_kwargs):
                commands.append(command)
                if command[:2] == ["docker", "save"]:
                    docker_archive(Path(command[3]), command[4])

            with patch.object(archives.subprocess, "run", side_effect=fake_run):
                archives.export_archives(root, self.identity, "build")
            self.assertEqual([
                ["docker", "save", "--output", str(root / filename), local_tag(self.identity, name, "build")]
                for name, filename in bundle_files("build").items()
            ], commands)
            self.assertEqual(set(BUILD_IMAGES), set(archives.validate_archives(root, self.identity, "build")))
            metadata = json.loads((root / METADATA_NAME).read_text())
            self.assertEqual({name: hashlib.sha256((root / filename).read_bytes()).hexdigest()
                              for name, filename in bundle_files("build").items()}, metadata["archives"])

    def test_prepare_e2e_accepts_only_bound_digest_and_fixed_baselines(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            digest = self.identity["artifact_repo"] + "/tools@sha256:" + "c" * 64
            commands = []

            def fake_run(command, **_kwargs):
                commands.append(command)
                if command[:2] == ["docker", "save"]:
                    docker_archive(Path(command[3]), command[4])

            with patch.dict("os.environ", PROTECTED_TOOLS_IMAGE=digest), patch.object(
                archives.subprocess, "run", side_effect=fake_run
            ):
                archives.export_archives(root, self.identity, "e2e")
            expected_commands = []
            for name, filename in bundle_files("e2e").items():
                reference = digest if name == "pr" else f"{self.identity['artifact_repo']}/tools:{name}"
                tag = local_tag(self.identity, name, "e2e")
                expected_commands.extend([["docker", "pull", reference], ["docker", "tag", reference, tag],
                                          ["docker", "save", "--output", str(root / filename), tag]])
            self.assertEqual(expected_commands, commands)
            self.assertEqual(set(E2E_IMAGES), set(archives.validate_archives(root, self.identity, "e2e")))
            with patch.dict("os.environ", PROTECTED_TOOLS_IMAGE="other/repo/tools@sha256:" + "c" * 64):
                with self.assertRaises(ActionError):
                    archives.export_archives(Path(tmp) / "invalid", self.identity, "e2e")

    def test_prior_attempt_archive_can_be_reused_but_future_attempt_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            current = dict(self.identity, run_attempt="3")
            self.assertEqual(set(BUILD_IMAGES), set(archives.validate_archives(root, current, "build")))
            stale = dict(self.identity, run_attempt="1")
            with self.assertRaises(ActionError):
                archives.validate_archives(root, stale, "build")

    def test_tar_paths_reject_parent_and_absolute_paths(self):
        self.assertFalse(archives.safe_tar_path("../layer.tar"))
        self.assertFalse(archives.safe_tar_path("/absolute"))

    def test_tar_links_bad_tags_and_manifest_traversal_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "image.tar"
            expected = "aptos-core/tools:" + "a" * 40 + "-from-local"
            docker_archive(path, expected)
            archives.validate_docker_archive(path, expected)
            with self.assertRaises(ActionError):
                archives.validate_docker_archive(path, expected + "-other")
            link = tarfile.TarInfo("escape")
            link.type = tarfile.SYMTYPE
            link.linkname = "../../etc/passwd"
            docker_archive(path, expected, extra=link)
            with self.assertRaises(ActionError):
                archives.validate_docker_archive(path, expected)
            docker_archive(path, expected, manifest={"Config": "../config.json", "RepoTags": [expected],
                                                     "Layers": ["layer/layer.tar"]})
            with self.assertRaises(ActionError):
                archives.validate_docker_archive(path, expected)

    def test_e2e_load_checks_all_images_then_sets_offline_aliases(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root, "e2e")
            env_file = Path(tmp) / "env"
            with patch.object(archives.subprocess, "run") as run, patch.dict("os.environ", GITHUB_ENV=str(env_file)):
                archives.load_e2e(root, self.identity)
            self.assertEqual([call(["docker", "load", "--input", str(root / filename)], check=True, timeout=1800)
                              for filename in bundle_files("e2e").values()], run.call_args_list)
            self.assertEqual(E2E_ENV, env_file.read_text())


class ImageArchivePropertyTests(unittest.TestCase):
    def assert_rejected_without_effects(self, root, identity, mode, category):
        env_file = root.parent / "env"
        env_file.write_text("existing=value\n")
        consumer = archives.publish_archives if mode == "build" else archives.load_e2e
        expected_error = "Invalid image archive digest" if category == "invalid-digest" else ".*"
        with patch.object(archives.subprocess, "run") as run, patch.dict("os.environ", GITHUB_ENV=str(env_file)):
            with self.assertRaisesRegex(ActionError, expected_error, msg=f"{mode}: {category}"):
                consumer(root, dict(identity))
            run.assert_not_called()
        self.assertEqual("existing=value\n", env_file.read_text(), f"{mode}: {category}")

    @given(identity=IDENTITIES)
    def test_generated_valid_build_and_e2e_bundles(self, identity):
        for mode in ("build", "e2e"):
            with tempfile.TemporaryDirectory() as tmp:
                expected = dict(identity)
                root = Path(tmp) / "bundle"
                metadata = make_bundle(root, expected, mode)
                files = bundle_files(mode)
                self.assertEqual({name: root / filename for name, filename in files.items()},
                                 archives.validate_archives(root, expected, mode))
                self.assertEqual(metadata, json.loads((root / METADATA_NAME).read_text()))
                env_file = Path(tmp) / "env"
                env_file.write_text("")
                with patch.object(archives.subprocess, "run") as run, patch.dict(
                    "os.environ", GITHUB_ENV=str(env_file)
                ):
                    if mode == "build":
                        archives.publish_archives(root, expected)
                        commands = [
                            call(["skopeo", "copy", f"docker-archive:{root / filename}:{local_tag(expected, name, mode)}",
                                  f"docker://{expected['artifact_repo']}/{name}:{release_tag(expected)}"],
                                 check=True, timeout=1800)
                            for name, filename in files.items()
                        ]
                    else:
                        archives.load_e2e(root, expected)
                        commands = [call(["docker", "load", "--input", str(root / filename)],
                                         check=True, timeout=1800) for filename in files.values()]
                    self.assertEqual(commands, run.call_args_list)
                self.assertEqual(E2E_ENV if mode == "e2e" else "", env_file.read_text())

    @given(identity=IDENTITIES, attempt=st.integers(min_value=2, max_value=999998))
    def test_generated_same_prior_and_future_attempts(self, identity, attempt):
        for mode in ("build", "e2e"):
            for produced_attempt in (attempt, attempt - 1, attempt + 1):
                with tempfile.TemporaryDirectory() as tmp:
                    expected = dict(identity, run_attempt=str(attempt))
                    produced = dict(expected, run_attempt=str(produced_attempt))
                    root = Path(tmp) / "bundle"
                    make_bundle(root, produced, mode)
                    if produced_attempt > attempt:
                        self.assert_rejected_without_effects(root, expected, mode, "future attempt")
                    else:
                        self.assertEqual(set(bundle_files(mode)),
                                         set(archives.validate_archives(root, expected, mode)))

    @given(identity=IDENTITIES)
    def test_generated_each_identity_mismatch_is_rejected(self, identity):
        replacements = {
            "source_repository": "other-" + identity["source_repository"],
            "source_sha": ("0" if identity["source_sha"][0] != "0" else "1") + identity["source_sha"][1:],
            "base_sha": ("0" if identity["base_sha"][0] != "0" else "1") + identity["base_sha"][1:],
            "pr_number": "1" if identity["pr_number"] != "1" else "2",
            "run_id": "1" if identity["run_id"] != "1" else "2",
            "variant": "performance",
            "artifact_repo": identity["artifact_repo"] + "-other",
        }
        self.assertEqual(set(identity) - {"run_attempt"}, set(replacements))
        for mode in ("build", "e2e"):
            for field, replacement in replacements.items():
                with tempfile.TemporaryDirectory() as tmp:
                    expected = dict(identity)
                    root = Path(tmp) / "bundle"
                    metadata = make_bundle(root, expected, mode)
                    metadata["identity"][field] = replacement
                    write_metadata(root, metadata)
                    self.assert_rejected_without_effects(root, expected, mode, field)

    @given(identity=IDENTITIES, component=SAFE_COMPONENT)
    def test_generated_rejection_categories_precede_all_side_effects(self, identity, component):
        # Every example exercises every category. The last archive is damaged so
        # publishing/loading any earlier validated image would fail this property.
        categories = (
            "digest-mismatch", "invalid-digest", "missing-file", "extra-file", "missing-digest", "extra-digest",
            "version-bool", "version-float", "version-zero", "version-future", "type-opposite", "type-unknown",
            "duplicate-metadata", "duplicate-identity", "duplicate-digest", "archive-symlink", "archive-hardlink",
            "metadata-symlink", "metadata-hardlink", "directory-symlink", "unsafe-parent", "unsafe-absolute",
            "unsafe-dot", "tar-symlink", "tar-hardlink", "duplicate-member", "wrong-tag", "extra-tag", "empty-tags",
            "missing-config", "missing-layer", "missing-manifest", "manifest-traversal", "duplicate-manifest-key",
        )
        for mode in ("build", "e2e"):
            for category in categories:
                with tempfile.TemporaryDirectory() as tmp:
                    expected = dict(identity)
                    root = Path(tmp) / "bundle"
                    metadata = make_bundle(root, expected, mode)
                    name, filename = list(bundle_files(mode).items())[-1]
                    path = root / filename
                    tag = local_tag(expected, name, mode)
                    if category == "digest-mismatch":
                        path.write_bytes(path.read_bytes() + b"corruption")
                    elif category == "invalid-digest":
                        metadata["archives"][name] = "g" * 64
                        write_metadata(root, metadata)
                    elif category == "missing-file":
                        path.unlink()
                    elif category == "extra-file":
                        (root / f"extra-{component}.tar").write_bytes(b"extra")
                    elif category == "missing-digest":
                        del metadata["archives"][name]
                        write_metadata(root, metadata)
                    elif category == "extra-digest":
                        metadata["archives"][f"extra-{component}"] = "0" * 64
                        write_metadata(root, metadata)
                    elif category.startswith("version-"):
                        metadata["version"] = {
                            "version-bool": True, "version-float": 1.0,
                            "version-zero": 0, "version-future": 2,
                        }[category]
                        write_metadata(root, metadata)
                    elif category.startswith("type-"):
                        opposite = "e2e" if mode == "build" else "build"
                        metadata["type"] = opposite if category == "type-opposite" else component + "-unknown"
                        write_metadata(root, metadata)
                    elif category.startswith("duplicate-") and category in (
                        "duplicate-metadata", "duplicate-identity", "duplicate-digest"
                    ):
                        raw = json.dumps(metadata)
                        if category == "duplicate-metadata":
                            raw = '{"version": 1, ' + raw[1:]
                        else:
                            key = "source_sha" if category == "duplicate-identity" else name
                            value = expected["source_sha"] if category == "duplicate-identity" else metadata["archives"][name]
                            raw = raw.replace(json.dumps(key) + ":", f"{json.dumps(key)}: {json.dumps(value)}, {json.dumps(key)}:", 1)
                        (root / METADATA_NAME).write_text(raw)
                    elif category in ("archive-symlink", "archive-hardlink", "metadata-symlink", "metadata-hardlink"):
                        target = root / METADATA_NAME if category.startswith("metadata") else path
                        outside = Path(tmp) / "linked-file"
                        target.rename(outside)
                        if category.endswith("symlink"):
                            target.symlink_to(outside)
                        else:
                            target.hardlink_to(outside)
                    elif category == "directory-symlink":
                        outside = Path(tmp) / "real-bundle"
                        root.rename(outside)
                        root.symlink_to(outside, target_is_directory=True)
                    else:
                        record = {"Config": "config.json", "RepoTags": [tag], "Layers": ["layer/layer.tar"]}
                        extra = None
                        omit = ()
                        raw = None
                        if category in ("unsafe-parent", "unsafe-absolute", "unsafe-dot"):
                            prefix = {"unsafe-parent": "../", "unsafe-absolute": "/", "unsafe-dot": "./"}[category]
                            extra = tarfile.TarInfo(prefix + component)
                        elif category in ("tar-symlink", "tar-hardlink"):
                            extra = tarfile.TarInfo("link-" + component)
                            extra.type = tarfile.SYMTYPE if category == "tar-symlink" else tarfile.LNKTYPE
                            extra.linkname = "config.json"
                        elif category == "duplicate-member":
                            extra = tarfile.TarInfo("config.json")
                        elif category == "wrong-tag":
                            record["RepoTags"] = [tag + "-other-" + component]
                        elif category == "extra-tag":
                            record["RepoTags"] = [tag, tag + "-extra"]
                        elif category == "empty-tags":
                            record["RepoTags"] = []
                        elif category in ("missing-config", "missing-layer", "missing-manifest"):
                            omit = ({"missing-config": "config.json", "missing-layer": "layer/layer.tar",
                                     "missing-manifest": "manifest.json"}[category],)
                        elif category == "manifest-traversal":
                            record["Layers"] = ["../" + component]
                        elif category == "duplicate-manifest-key":
                            raw = ('[{"Config":"config.json",' + json.dumps(record)[1:] + "]").encode()
                        else:
                            self.fail(f"Unhandled rejection category: {category}")
                        docker_archive(path, tag, extra=extra, manifest=record, manifest_bytes=raw, omit=omit)
                        # Keep the digest valid so this reaches Docker structure validation.
                        metadata["archives"][name] = hashlib.sha256(path.read_bytes()).hexdigest()
                        write_metadata(root, metadata)
                    self.assert_rejected_without_effects(root, expected, mode, category)


if __name__ == "__main__":
    unittest.main()
