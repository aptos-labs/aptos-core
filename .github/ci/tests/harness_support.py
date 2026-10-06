"""Test-only loading and image contracts shared by the external harness suites."""

import importlib.util
import json
import os
import sys
import types
from unittest.mock import patch

from hypothesis import example, given, strategies as st

from tests.property_support import configure_profiles

configure_profiles()

DIGESTS = st.integers(0, 2**256 - 1).map(lambda value: "sha256:" + format(value, "064x"))
TAGS = st.text(alphabet="abcdefghijklmnopqrstuvwxyz0123456789_-", min_size=1, max_size=100)
REPOS = st.sampled_from(["", "registry.example/repo", "registry.example/repo/",
                        "registry.example/repo///", "us-docker.pkg.dev/aptos-registry/docker"])


def load_module(name, path, imports=None):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {**(imports or {}), name: module}):
        spec.loader.exec_module(module)
    return module


def load_common(name, path):
    # AccountAddress is unrelated to image resolution. Only its import-time
    # from_str constructor is required by the real E2E common module.
    sdk = types.ModuleType("aptos_sdk")
    address_module = types.ModuleType("aptos_sdk.account_address")

    class AccountAddress:
        @classmethod
        def from_str(cls, value):
            return value

    address_module.AccountAddress = AccountAddress
    return load_module(name, path, {"aptos_sdk": sdk, "aptos_sdk.account_address": address_module})


def invalid_maps(repository):
    """Every finite malformed-map category runs for each generated example."""
    values = ["{bad JSON", "null", "[]", "true", "1", '"scalar"', "{}",
              json.dumps({repository + "-other": "sha256:" + "0" * 64})]
    for digest in (None, True, 1, [], {}, "latest", "sha256:bad", "sha256:" + "A" * 64,
                   "sha256:" + "a" * 63, "sha256:" + "a" * 65, "sha256:" + "a" * 64 + "\n"):
        values.append(json.dumps({repository: digest}))
    return values


class ToolsImageContract:
    """Mixin: the concrete test class supplies the real build_image_name."""

    @example(repo="us-docker.pkg.dev/aptos-registry/docker", tag="approved", digest="sha256:" + "a" * 64)
    @given(REPOS, TAGS, DIGESTS)
    def test_generated_tools_approved_digest_and_baseline_tag(self, repo, tag, digest):
        repository = (repo.rstrip("/") + "/" if repo.rstrip("/") else "") + "tools"
        baseline = tag + "-baseline"
        other = "baseline" if tag != "baseline" else "other-baseline"
        with patch.dict(os.environ, {
            "PROTECTED_IMAGE_TAG": tag,
            "PROTECTED_IMAGE_DIGESTS": json.dumps({repository: digest}),
        }, clear=True):
            self.assertEqual(f"{repository}@{digest}", self.build_image(repo, tag))
            self.assertEqual(f"{repository}:{baseline}", self.build_image(repo, baseline))
            self.assertEqual(f"{repository}:{other}", self.build_image(repo, other))
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(f"{repository}:{tag}", self.build_image(repo, tag))
            self.assertEqual(f"{repository}:{other}", self.build_image(repo, other))

    @example(repo="us-docker.pkg.dev/aptos-registry/docker", tag="approved")
    @given(REPOS, TAGS)
    def test_generated_tools_invalid_maps_fail_closed(self, repo, tag):
        repository = (repo.rstrip("/") + "/" if repo.rstrip("/") else "") + "tools"
        for raw in invalid_maps(repository):
            with patch.dict(os.environ, {
                "PROTECTED_IMAGE_TAG": tag,
                "PROTECTED_IMAGE_DIGESTS": raw,
            }, clear=True):
                with self.assertRaises(ValueError):
                    self.build_image(repo, tag)
                # Baseline images do not consume the approved image map.
                self.assertEqual(f"{repository}:{tag}-baseline", self.build_image(repo, tag + "-baseline"))

    @example(repo="us-docker.pkg.dev/aptos-registry/docker", tag="approved", digest="sha256:" + "a" * 64)
    @given(REPOS, TAGS, DIGESTS)
    def test_generated_tools_incomplete_configuration_fails_closed(self, repo, tag, digest):
        repository = (repo.rstrip("/") + "/" if repo.rstrip("/") else "") + "tools"
        mapping = json.dumps({repository: digest})
        for env in ({"PROTECTED_IMAGE_TAG": tag}, {"PROTECTED_IMAGE_DIGESTS": mapping},
                    {"PROTECTED_IMAGE_TAG": tag, "PROTECTED_IMAGE_DIGESTS": ""},
                    {"PROTECTED_IMAGE_TAG": "", "PROTECTED_IMAGE_DIGESTS": mapping}):
            with patch.dict(os.environ, env, clear=True):
                for requested in (tag, tag + "-baseline"):
                    try:
                        self.build_image(repo, requested)
                    except Exception as error:
                        self.assertIsInstance(error, ValueError)
                        self.assertRegex(
                            str(error),
                            "PROTECTED_IMAGE_TAG and PROTECTED_IMAGE_DIGESTS must be set together",
                        )
                    else:
                        self.fail("Incomplete protected image configuration was accepted")
