import json
import unittest
from unittest.mock import patch

from common import build_image_name


class ProtectedToolsImageTest(unittest.TestCase):
    def test_approved_and_baseline_tags(self):
        repository = "us-docker.pkg.dev/aptos-registry/docker/tools"
        digest = "sha256:" + "a" * 64
        with patch.dict("os.environ", {
            "PROTECTED_IMAGE_TAG": "approved",
            "PROTECTED_IMAGE_DIGESTS": json.dumps({repository: digest}),
        }):
            self.assertEqual(build_image_name(repository[:-5], "approved"), f"{repository}@{digest}")
            self.assertEqual(build_image_name(repository[:-5], "baseline"), f"{repository}:baseline")

    def test_without_protected_configuration_uses_tag(self):
        with patch.dict("os.environ", {}, clear=True):
            self.assertEqual(build_image_name("registry.example/repo", "baseline"), "registry.example/repo/tools:baseline")

    def test_missing_or_invalid_digest_fails_closed(self):
        repository = "us-docker.pkg.dev/aptos-registry/docker/tools"
        for mapping in ({}, {repository: "sha256:bad"}):
            with self.subTest(mapping=mapping), patch.dict("os.environ", {
                "PROTECTED_IMAGE_TAG": "approved",
                "PROTECTED_IMAGE_DIGESTS": json.dumps(mapping),
            }):
                with self.assertRaises(ValueError):
                    build_image_name("us-docker.pkg.dev/aptos-registry/docker", "approved")


if __name__ == "__main__":
    unittest.main()
