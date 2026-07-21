#!/usr/bin/env python3

import os
import sys
import unittest
import tempfile
import re


VERSION_LINE_REGEX = re.compile(r"^version =")
# Tag looks like aptos-node-vX.Y.Z, optionally suffixed with -rc[.N]. The
# Cargo.toml version must match the tag's version portion EXACTLY (including
# any -rc suffix) so the build_info metric carries the same version string
# users see on the tag / docker image.
VERSION_IN_RELEASE_TAG_REGEX = r"v(\d+\.\d+\.\d+(?:-rc(?:\.\d+)?)?)$"


def get_release_number_from_release_tag(release_tag: str) -> str:
    """Get the X.Y.Z[-rc[.N]] version from the release tag."""
    result = re.search(VERSION_IN_RELEASE_TAG_REGEX, release_tag)
    if not result:
        print(f"Release tag {release_tag} does not match the expected format")
        sys.exit(1)
    return result.group(1)


def main():
    required_envs = ["RELEASE_TAG", "APTOS_NODE_CARGO_TOML"]
    for env in required_envs:
        if not os.environ.get(env):
            print(f"Required environment variable {env} not set")
            sys.exit(1)

    RELEASE_TAG = os.environ.get("RELEASE_TAG")
    APTOS_NODE_TOML = os.environ.get("APTOS_NODE_CARGO_TOML")

    new_lines = []  # construct the file again
    with open(APTOS_NODE_TOML, "r") as f:
        lines = f.readlines()
        for line in lines:
            new_line = line
            if VERSION_LINE_REGEX.match(line):
                new_line = (
                    f'version = "{get_release_number_from_release_tag(RELEASE_TAG)}"\n'
                )
            new_lines.append(new_line)

    with open(APTOS_NODE_TOML, "w") as f:
        f.writelines(new_lines)

if __name__ == "__main__":
    main()


class CheckAptosNodeVersionTests(unittest.TestCase):
    def test_envs_not_specified(self):
        with self.assertRaises(SystemExit):
            os.environ["RELEASE_TAG"] = ""
            os.environ["APTOS_NODE_CARGO_TOML"] = ""
            main()

    def _bump(self, tag: str, initial_version: str) -> str:
        tmp = tempfile.NamedTemporaryFile()
        with open(tmp.name, "w") as f:
            f.write(f'[package]\nversion = "{initial_version}"\n')
        os.environ["RELEASE_TAG"] = tag
        os.environ["APTOS_NODE_CARGO_TOML"] = tmp.name
        main()
        with open(tmp.name) as f:
            for line in f:
                if VERSION_LINE_REGEX.match(line):
                    return line.rstrip("\n")
        raise AssertionError("no version line found after bump")

    def test_bump_release(self):
        self.assertEqual(self._bump("aptos-node-v1.1.0", "0.0.0-main"), 'version = "1.1.0"')

    def test_bump_rc(self):
        self.assertEqual(self._bump("aptos-node-v1.1.0-rc", "0.0.0-main"), 'version = "1.1.0-rc"')

    def test_bump_rc_numbered(self):
        self.assertEqual(self._bump("aptos-node-v1.1.0-rc.2", "0.0.0-main"), 'version = "1.1.0-rc.2"')

    def test_release_number_from_tag(self):
        self.assertEqual(get_release_number_from_release_tag("aptos-node-v1.0.0"), "1.0.0")
        self.assertEqual(get_release_number_from_release_tag("aptos-node-v1.0.0-rc"), "1.0.0-rc")
        self.assertEqual(get_release_number_from_release_tag("aptos-node-v1.0.0-rc.3"), "1.0.0-rc.3")

    def test_bad_tag_format(self):
        with self.assertRaises(SystemExit):
            get_release_number_from_release_tag("aptos-node-1.0.0")
