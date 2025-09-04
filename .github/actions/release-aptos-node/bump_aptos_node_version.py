#!/usr/bin/env python3

import os
import sys
import unittest
import tempfile
import re


VERSION_LINE_REGEX = re.compile(r"^version =")
VERSION_IN_RELEASE_TAG_REGEX = r"v(\d+)\.(\d+)\.(\d+)$"


def get_release_number_from_release_tag(release_tag: str) -> str:
    """Get the release number from the release tag. The release tag looks like velor-node-vX.Y.Z"""
    result = re.search(VERSION_IN_RELEASE_TAG_REGEX, release_tag)
    if not result:
        print(f"Release tag {release_tag} does not match the expected format")
        sys.exit(1)
    return ".".join(result.groups())


def main():
    required_envs = ["RELEASE_TAG", "VELOR_NODE_CARGO_TOML"]
    for env in required_envs:
        if not os.environ.get(env):
            print(f"Required environment variable {env} not set")
            sys.exit(1)

    RELEASE_TAG = os.environ.get("RELEASE_TAG")
    VELOR_NODE_TOML = os.environ.get("VELOR_NODE_CARGO_TOML")

    new_lines = []  # construct the file again
    with open(VELOR_NODE_TOML, "r") as f:
        lines = f.readlines()
        for line in lines:
            new_line = line
            if VERSION_LINE_REGEX.match(line):
                new_line = (
                    f'version = "{get_release_number_from_release_tag(RELEASE_TAG)}"\n'
                )
            new_lines.append(new_line)

    with open(VELOR_NODE_TOML, "w") as f:
        f.writelines(new_lines)

if __name__ == "__main__":
    main()


class CheckVelorNodeVersionTests(unittest.TestCase):
    def test_envs_not_specified(self):
        with self.assertRaises(SystemExit):
            os.environ["RELEASE_TAG"] = ""
            os.environ["VELOR_NODE_CARGO_TOML"] = ""
            main()

    def test_version_match(self):
        tmp = tempfile.NamedTemporaryFile()
        with open(tmp.name, "w") as f:
            f.write(
                """
                [package]
                version = "1.0.0"
                """
            )
        os.environ["RELEASE_TAG"] = "release-1.0"
        os.environ["VELOR_NODE_CARGO_TOML"] = tmp.name
        main()  # this should work

    def test_version_mismatch(self):
        tmp = tempfile.NamedTemporaryFile()
        with open(tmp.name, "w") as f:
            f.write(
                """
                [package]
                version = "1.0.0"
                """
            )
        os.environ["RELEASE_TAG"] = "release-v1.1.0"
        os.environ["VELOR_NODE_CARGO_TOML"] = tmp.name
        main()
        with open(tmp.name, "r") as f:
            lines = f.readlines()
            for line in lines:
                if VERSION_LINE_REGEX.match(line):
                    self.assertEqual(line, 'version = "1.1.0"\n')

    def test_release_number_from_tag(self):
        self.assertEqual(get_release_number_from_release_tag("release-v1.0.0"), "1.0.0")
