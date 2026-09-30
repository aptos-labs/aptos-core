import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from ci_actions.github import ActionError
from ci_actions import image_archives as archives


def docker_archive(path: Path, tag: str, *, extra=None, manifest=None):
    record = manifest or {"Config": "config.json", "RepoTags": [tag], "Layers": ["layer/layer.tar"]}
    content = {"manifest.json": json.dumps([record]).encode(), "config.json": b"{}",
               "layer/layer.tar": b"layer"}
    with tarfile.open(path, "w") as tar:
        for name, data in content.items():
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
        root.mkdir()
        hashes = {}
        for name, filename in archives.archive_names(mode).items():
            path = root / filename
            docker_archive(path, archives.source_tag(self.identity, name, mode))
            hashes[name] = archives.digest_file(path)
        (root / archives.METADATA_NAME).write_text(json.dumps({
            "version": 1, "type": mode, "identity": self.identity, "archives": hashes}))

    def test_validated_publish_uses_fixed_tags_and_skopeo_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            with patch.object(archives.subprocess, "run") as run:
                archives.publish_archives(root, self.identity)
            self.assertEqual(len(archives.IMAGE_NAMES), run.call_count)
            for call in run.call_args_list:
                command = call.args[0]
                self.assertEqual(["skopeo", "copy"], command[:2])
                self.assertIn("aptos-core/", command[2])
                self.assertIn(archives.image_tag(self.identity), command[3])

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
            self.assertEqual(len(archives.IMAGE_NAMES), len(commands))
            self.assertEqual(set(archives.archive_names("build").values()),
                             {Path(command[3]).name for command in commands})
            self.assertEqual(set(archives.IMAGE_NAMES), set(archives.validate_archives(root, self.identity, "build")))

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
            self.assertEqual(4, sum(command[:2] == ["docker", "pull"] for command in commands))
            self.assertIn(["docker", "pull", digest], commands)
            self.assertEqual(set(archives.E2E_ALIASES), set(archives.validate_archives(root, self.identity, "e2e")))
            with patch.dict("os.environ", PROTECTED_TOOLS_IMAGE="other/repo/tools@sha256:" + "c" * 64):
                with self.assertRaises(ActionError):
                    archives.export_archives(Path(tmp) / "invalid", self.identity, "e2e")

    def test_identity_hash_files_and_metadata_fail_closed_before_publish(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            with patch.object(archives.subprocess, "run") as run:
                for expected in [dict(self.identity, source_sha="c" * 40),
                                 dict(self.identity, base_sha="c" * 40),
                                 dict(self.identity, run_attempt="1")]:
                    with self.assertRaises(ActionError):
                        archives.publish_archives(root, expected)
                (root / "validator.tar").write_bytes(b"changed")
                with self.assertRaises(ActionError):
                    archives.publish_archives(root, self.identity)
                self.assertEqual(0, run.call_count)

    def test_prior_attempt_archive_can_be_reused_but_future_attempt_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            current = dict(self.identity, run_attempt="3")
            self.assertEqual(set(archives.IMAGE_NAMES), set(archives.validate_archives(root, current, "build")))
            stale = dict(self.identity, run_attempt="1")
            with self.assertRaises(ActionError):
                archives.validate_archives(root, stale, "build")

    def test_extra_alias_symlink_duplicate_metadata_and_traversal_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "bundle"
            self.make_bundle(root)
            (root / "extra.tar").write_bytes(b"evil")
            with self.assertRaises(ActionError):
                archives.validate_archives(root, self.identity, "build")
            (root / "extra.tar").unlink()
            path = root / "validator.tar"
            path.unlink()
            path.symlink_to(root / "tools.tar")
            with self.assertRaises(ActionError):
                archives.validate_archives(root, self.identity, "build")
            path.unlink()
            docker_archive(path, archives.source_tag(self.identity, "validator", "build"))
            metadata = root / archives.METADATA_NAME
            metadata.write_text('{"version":1,"version":1}')
            with self.assertRaises(ActionError):
                archives.validate_archives(root, self.identity, "build")
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
            self.assertEqual(4, run.call_count)
            self.assertTrue(all(call.args[0][:2] == ["docker", "load"] for call in run.call_args_list))
            self.assertEqual("PR_IMAGE_TAG=pr\nGCP_DOCKER_ARTIFACT_REPO=aptos-ci\n"
                             "APTOS_E2E_OFFLINE_IMAGES=true\n", env_file.read_text())


if __name__ == "__main__":
    unittest.main()
