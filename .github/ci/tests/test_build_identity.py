"""Tests for ci_actions.build_identity.BuildIdentity."""

import dataclasses
import os
import unittest
from unittest import mock

from tests.property_support import configure_profiles

configure_profiles()

from ci_actions.build_identity import BuildIdentity
from ci_actions.github import ActionError

BASE = dict(source_repository="contributor/aptos-core", source_sha="a" * 40, base_sha="d" * 40,
            pr_number="42", run_id="123", run_attempt="2", variant="release",
            artifact_repo="us-docker.pkg.dev/aptos-registry/docker")


class BuildIdentityTests(unittest.TestCase):
    def test_parse_rejects_wrong_shapes_with_the_fields_message(self):
        for value in (None, [], "x", {**BASE, "extra": "1"}, {k: v for k, v in BASE.items() if k != "variant"}):
            with self.subTest(value=value), self.assertRaisesRegex(ActionError, "Invalid protected image identity fields"):
                BuildIdentity.parse(value)

    def test_each_field_is_validated_in_order(self):
        bad = {
            "source_repository": "no-slash", "source_sha": "A" * 40, "base_sha": "a" * 39,
            "pr_number": "01", "run_id": "0", "run_attempt": "1234567",
            "variant": "Release", "artifact_repo": "ghcr.io/aptos/docker",
        }
        for key, value in bad.items():
            with self.subTest(key=key), self.assertRaisesRegex(ActionError, f"Invalid protected image {key}$"):
                BuildIdentity.parse({**BASE, key: value})
        with self.assertRaisesRegex(ActionError, "Invalid protected image source_repository"):
            BuildIdentity.parse({**BASE, "source_repository": 7, "source_sha": "bad"})

    def test_run_id_is_capped_at_the_safe_integer_limit(self):
        self.assertEqual(str(2**53 - 1), BuildIdentity.parse({**BASE, "run_id": str(2**53 - 1)}).run_id)
        with self.assertRaisesRegex(ActionError, "Invalid protected image run_id"):
            BuildIdentity.parse({**BASE, "run_id": str(2**53)})

    def test_unknown_variant_is_rejected(self):
        with self.assertRaisesRegex(ActionError, "Unknown protected image variant"):
            BuildIdentity.parse({**BASE, "variant": "nightly"})

    def test_every_construction_path_validates(self):
        identity = BuildIdentity.parse(dict(BASE))
        with self.assertRaisesRegex(ActionError, "Invalid protected image run_id"):
            dataclasses.replace(identity, run_id=str(2**53))
        with self.assertRaisesRegex(ActionError, "Invalid protected image source_sha"):
            BuildIdentity(**{**BASE, "source_sha": "bad"})
        with self.assertRaises(dataclasses.FrozenInstanceError):
            identity.run_id = "1"

    def test_tags_match_each_manifest_variant(self):
        sha = "a" * 40
        expected = {
            "release": f"pr-42_r123-a2_{sha}",
            "failpoints": f"pr-42_failpoints_r123-a2_{sha}",
            "performance": f"pr-42_performance_r123-a2_{sha}",
            "consensus": f"pr-42_consensus_only_perf_test_r123-a2_{sha}",
        }
        for variant, tag in expected.items():
            identity = BuildIdentity.parse({**BASE, "variant": variant})
            self.assertEqual(tag, identity.image_tag(), variant)
            self.assertEqual("controller-r123-a2_" + "d" * 40, identity.controller_tag())

    def test_check_same_build_rejects_changes_and_newer_attempts(self):
        expected = BuildIdentity.parse(dict(BASE))
        replacements = {
            "source_repository": "other/aptos-core", "source_sha": "b" * 40, "base_sha": "e" * 40,
            "pr_number": "43", "run_id": "124", "variant": "performance",
            "artifact_repo": "us-docker.pkg.dev/aptos-registry/other",
        }
        for key, value in replacements.items():
            with self.subTest(key=key), self.assertRaisesRegex(ActionError, "^mismatch$"):
                expected.check_same_build(BuildIdentity.parse({**BASE, key: value}), mismatch="mismatch", future="future")
        with self.assertRaisesRegex(ActionError, "^future$"):
            expected.check_same_build(BuildIdentity.parse({**BASE, "run_attempt": "3"}), mismatch="mismatch", future="future")
        for attempt in ("1", "2"):
            expected.check_same_build(BuildIdentity.parse({**BASE, "run_attempt": attempt}), mismatch="m", future="f")

    def test_from_env_reads_inputs_and_reports_the_first_missing_one(self):
        env = {"INPUT_" + key.upper(): value for key, value in BASE.items()}
        with mock.patch.dict(os.environ, env, clear=True):
            self.assertEqual(BASE, BuildIdentity.from_env().as_dict())
        partial = {k: v for k, v in env.items() if k not in ("INPUT_RUN_ID", "INPUT_ARTIFACT_REPO")}
        with mock.patch.dict(os.environ, partial, clear=True), self.assertRaisesRegex(ActionError, "INPUT_RUN_ID"):
            BuildIdentity.from_env()


if __name__ == "__main__":
    unittest.main()
