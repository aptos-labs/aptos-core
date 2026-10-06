import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import call, patch

from hypothesis import given, strategies as st

from ci_actions.github import ActionError
from ci_actions import image_archives as archives
from ci_actions.protected_images import IMAGE_NAMES
from tests.property_support import IDENTITIES, configure_profiles

configure_profiles()

E2E_IMAGES = ("pr", "devnet", "testnet", "mainnet")
METADATA_NAME = "image-archives.json"
E2E_ENV = "PR_IMAGE_TAG=pr\nGCP_DOCKER_ARTIFACT_REPO=aptos-ci\nAPTOS_E2E_OFFLINE_IMAGES=true\n"
MODES = ("build", "e2e")
IDENTITY = dict(source_repository="contributor/aptos-core", source_sha="a" * 40,
                base_sha="b" * 40, pr_number="42", run_id="123", run_attempt="2",
                variant="release", artifact_repo="us-docker.pkg.dev/aptos-registry/docker")


def bundle_files(mode):
    if mode == "build":
        return {name: f"{name}.tar" for name in IMAGE_NAMES}
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
    def test_export_build_saves_fixed_local_references(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            commands = []

            def fake_run(command, **_kwargs):
                commands.append(command)
                if command[:2] == ["docker", "save"]:
                    docker_archive(Path(command[3]), command[4])

            with patch.object(archives.subprocess, "run", side_effect=fake_run):
                archives.export_archives(root, IDENTITY, "build")
            self.assertEqual([
                ["docker", "save", "--output", str(root / filename), local_tag(IDENTITY, name, "build")]
                for name, filename in bundle_files("build").items()
            ], commands)
            self.assertEqual(set(IMAGE_NAMES), set(archives.validate_archives(root, IDENTITY, "build")))
            metadata = json.loads((root / METADATA_NAME).read_text())
            self.assertEqual({name: hashlib.sha256((root / filename).read_bytes()).hexdigest()
                              for name, filename in bundle_files("build").items()}, metadata["archives"])

    def test_prepare_e2e_accepts_only_bound_digest_and_fixed_baselines(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            digest = IDENTITY["artifact_repo"] + "/tools@sha256:" + "c" * 64
            commands = []

            def fake_run(command, **_kwargs):
                commands.append(command)
                if command[:2] == ["docker", "save"]:
                    docker_archive(Path(command[3]), command[4])

            with patch.dict("os.environ", PROTECTED_TOOLS_IMAGE=digest), patch.object(
                archives.subprocess, "run", side_effect=fake_run
            ):
                archives.export_archives(root, IDENTITY, "e2e")
            expected_commands = []
            for name, filename in bundle_files("e2e").items():
                reference = digest if name == "pr" else f"{IDENTITY['artifact_repo']}/tools:{name}"
                tag = local_tag(IDENTITY, name, "e2e")
                expected_commands.extend([["docker", "pull", reference], ["docker", "tag", reference, tag],
                                          ["docker", "save", "--output", str(root / filename), tag]])
            self.assertEqual(expected_commands, commands)
            self.assertEqual(set(E2E_IMAGES), set(archives.validate_archives(root, IDENTITY, "e2e")))
            with patch.dict("os.environ", PROTECTED_TOOLS_IMAGE="other/repo/tools@sha256:" + "c" * 64):
                with self.assertRaises(ActionError):
                    archives.export_archives(Path(tmp) / "invalid", IDENTITY, "e2e")


def rewrite_metadata(change):
    def mutate(bundle):
        change(bundle)
        write_metadata(bundle.root, bundle.metadata)
    return mutate


def set_metadata(**fields):
    return rewrite_metadata(lambda bundle: bundle.metadata.update(fields))


def duplicate_metadata_key(key_value):
    def mutate(bundle):
        key, value = (json.dumps(part) for part in key_value(bundle))
        raw = json.dumps(bundle.metadata).replace(f"{key}:", f"{key}: {value}, {key}:", 1)
        (bundle.root / METADATA_NAME).write_text(raw)
    return mutate


def link_file(metadata, symbolic):
    def mutate(bundle):
        target = bundle.root / METADATA_NAME if metadata else bundle.path
        outside = bundle.root.parent / "linked-file"
        target.rename(outside)
        (target.symlink_to if symbolic else target.hardlink_to)(outside)
    return mutate


def link_directory(bundle):
    outside = bundle.root.parent / "real-bundle"
    bundle.root.rename(outside)
    bundle.root.symlink_to(outside, target_is_directory=True)


def rebuild_archive(change):
    """`change(bundle, record)` edits the manifest record or returns docker_archive options."""
    def mutate(bundle):
        record = {"Config": "config.json", "RepoTags": [bundle.tag], "Layers": ["layer/layer.tar"]}
        docker_archive(bundle.path, bundle.tag, manifest=record, **(change(bundle, record) or {}))
        # Keep the digest valid so this reaches Docker structure validation.
        bundle.metadata["archives"][bundle.name] = hashlib.sha256(bundle.path.read_bytes()).hexdigest()
        write_metadata(bundle.root, bundle.metadata)
    return mutate


def extra_member(name, kind=tarfile.REGTYPE):
    member = tarfile.TarInfo(name)
    member.type = kind
    if kind != tarfile.REGTYPE:
        member.linkname = "config.json"
    return rebuild_archive(lambda _bundle, _record: {"extra": member})


def manifest_bytes(encode):
    return rebuild_archive(lambda _bundle, record: {"manifest_bytes": encode(json.dumps([record]))})


def set_record(**fields):
    return rebuild_archive(lambda bundle, record: record.update({
        key: value(bundle) if callable(value) else value for key, value in fields.items()}))


def omit_member(name):
    return rebuild_archive(lambda _bundle, _record: {"omit": (name,)})


VERSION_OR_TYPE = "^Invalid image archive metadata version or type$"
DUPLICATE_KEY = "^Duplicate protected image manifest key$"
NOT_REGULAR = "^Image archive must be a regular unlinked file: "
UNSAFE_MEMBER = "^Unsafe or duplicate Docker archive path$"
WRONG_TAG = "^Docker archive source tag does not match expected image$"
MISSING_PART = "^Docker archive manifest references a missing file$"
INVALID_ARCHIVE = "^Invalid Docker archive: "
IDENTITY_MISMATCH = "^Image archive identity does not match approved build$"
# Each category damages the last archive and must fail with its own message.
REJECTIONS = {
    "digest-mismatch": (lambda bundle: bundle.path.write_bytes(bundle.path.read_bytes() + b"corruption"),
                        "^Image archive digest mismatch$"),
    "invalid-digest": (rewrite_metadata(lambda bundle: bundle.metadata["archives"].update({bundle.name: "g" * 64})),
                       "^Invalid image archive digest$"),
    "missing-file": (lambda bundle: bundle.path.unlink(), "^Image archive has missing or unexpected files$"),
    "extra-file": (lambda bundle: (bundle.root / "extra.tar").write_bytes(b"extra"),
                   "^Image archive has missing or unexpected files$"),
    "missing-digest": (rewrite_metadata(lambda bundle: bundle.metadata["archives"].pop(bundle.name)),
                       "^Image archive has missing or unexpected digests$"),
    "extra-digest": (rewrite_metadata(lambda bundle: bundle.metadata["archives"].update(extra="0" * 64)),
                     "^Image archive has missing or unexpected digests$"),
    "version-bool": (set_metadata(version=True), VERSION_OR_TYPE),
    "version-float": (set_metadata(version=1.0), VERSION_OR_TYPE),
    "version-zero": (set_metadata(version=0), VERSION_OR_TYPE),
    "version-future": (set_metadata(version=2), VERSION_OR_TYPE),
    "metadata-non-finite": (set_metadata(version=float("nan")),
                            "^Invalid image archive metadata: JSON number must be finite"),
    "type-opposite": (rewrite_metadata(lambda bundle: bundle.metadata.update(
        type="e2e" if bundle.mode == "build" else "build")), VERSION_OR_TYPE),
    "type-unknown": (set_metadata(type="unknown"), VERSION_OR_TYPE),
    "identity-not-object": (set_metadata(identity=list(IDENTITY.values())), "^Invalid image archive identity$"),
    "duplicate-metadata": (duplicate_metadata_key(lambda _bundle: ("version", 1)), DUPLICATE_KEY),
    "duplicate-identity": (duplicate_metadata_key(lambda _bundle: ("source_sha", IDENTITY["source_sha"])),
                           DUPLICATE_KEY),
    "duplicate-digest": (duplicate_metadata_key(lambda bundle: (bundle.name, bundle.metadata["archives"][bundle.name])),
                         DUPLICATE_KEY),
    "archive-symlink": (link_file(metadata=False, symbolic=True), NOT_REGULAR),
    "archive-hardlink": (link_file(metadata=False, symbolic=False), NOT_REGULAR),
    "metadata-symlink": (link_file(metadata=True, symbolic=True), NOT_REGULAR),
    "metadata-hardlink": (link_file(metadata=True, symbolic=False), NOT_REGULAR),
    "directory-symlink": (link_directory, "^Image archive directory must be a real directory$"),
    "unsafe-parent": (extra_member("../member"), UNSAFE_MEMBER),
    "unsafe-absolute": (extra_member("/member"), UNSAFE_MEMBER),
    "unsafe-dot": (extra_member("./member"), UNSAFE_MEMBER),
    "duplicate-member": (extra_member("config.json"), UNSAFE_MEMBER),
    "tar-symlink": (extra_member("link", tarfile.SYMTYPE), "^Docker archive has a link or special member$"),
    "tar-hardlink": (extra_member("link", tarfile.LNKTYPE), "^Docker archive has a link or special member$"),
    "wrong-tag": (set_record(RepoTags=lambda bundle: [bundle.tag + "-other"]), WRONG_TAG),
    "extra-tag": (set_record(RepoTags=lambda bundle: [bundle.tag, bundle.tag + "-extra"]), WRONG_TAG),
    "empty-tags": (set_record(RepoTags=[]), WRONG_TAG),
    "missing-config": (omit_member("config.json"), MISSING_PART),
    "missing-layer": (omit_member("layer/layer.tar"), MISSING_PART),
    "missing-manifest": (omit_member("manifest.json"), "^Missing or oversized Docker archive manifest$"),
    "manifest-traversal": (set_record(Layers=["../layer/layer.tar"]), "^Unsafe Docker archive manifest path$"),
    "duplicate-manifest-key": (manifest_bytes(lambda text: ('[{"Config":"config.json",' + text[2:]).encode()),
                               DUPLICATE_KEY),
    "manifest-bom": (manifest_bytes(lambda text: b"\xef\xbb\xbf" + text.encode()), INVALID_ARCHIVE),
    "manifest-utf16": (manifest_bytes(lambda text: text.encode("utf-16")), INVALID_ARCHIVE),
}


class ImageArchivePropertyTests(unittest.TestCase):
    def assert_rejected_without_effects(self, root, identity, mode, error, label):
        env_file = root.parent / "env"
        env_file.write_text("existing=value\n")
        consumer = archives.publish_archives if mode == "build" else archives.load_e2e
        with patch.object(archives.subprocess, "run") as run, patch.dict("os.environ", GITHUB_ENV=str(env_file)):
            with self.assertRaisesRegex(ActionError, error, msg=f"{mode}: {label}"):
                consumer(root, dict(identity))
            run.assert_not_called()
        self.assertEqual("existing=value\n", env_file.read_text(), f"{mode}: {label}")

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
                        self.assert_rejected_without_effects(root, expected, mode, "^Image archive is from a future attempt$",
                                                             "future attempt")
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
                    self.assert_rejected_without_effects(root, expected, mode, IDENTITY_MISMATCH, field)

    def test_each_rejection_category_precedes_all_side_effects(self):
        for mode in MODES:
            for category, (mutate, error) in REJECTIONS.items():
                with self.subTest(mode=mode, category=category), tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp) / "bundle"
                    metadata = make_bundle(root, IDENTITY, mode)
                    # Damage the last archive, so publishing or loading an earlier image would fail.
                    name, filename = list(bundle_files(mode).items())[-1]
                    mutate(SimpleNamespace(root=root, metadata=metadata, name=name, path=root / filename,
                                           tag=local_tag(IDENTITY, name, mode), mode=mode))
                    self.assert_rejected_without_effects(root, IDENTITY, mode, error, category)


if __name__ == "__main__":
    unittest.main()
