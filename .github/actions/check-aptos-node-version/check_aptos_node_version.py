#!/usr/bin/env python3

import tomllib
import os
import sys
import unittest
import tempfile


def main():
    required_envs = ["APTOS_CORE_BRANCH", "APTOS_NODE_CARGO_TOML"]
    for env in required_envs:
        if not os.environ.get(env):
            print(f"Required environment variable {env} not set")
            sys.exit(1)

    BRANCH = os.environ.get("APTOS_CORE_BRANCH")
    APTOS_NODE_TOML = os.environ.get("APTOS_NODE_CARGO_TOML")

    with open(APTOS_NODE_TOML, "rb") as f:
        toml = tomllib.load(f)
        package = toml.get("package")
        if not package:
            print("No package section in Cargo.toml")
            sys.exit(1)
        version = package.get("version")
        if not version:
            print("No version in Cargo.toml")
            sys.exit(1)

        minor_version = ".".join(version.split(".")[:2])
        if minor_version not in BRANCH:
            print(
                f"aptos-node version {minor_version} does not match release branch {BRANCH}"
            )
            sys.exit(1)

        print(
            f"SUCCESS: aptos-node version {minor_version} matches release branch {BRANCH}!"
        )


if __name__ == "__main__":
    main()


class CheckAptosNodeVersionTests(unittest.TestCase):
    def test_envs_not_specified(self):
        with self.assertRaises(SystemExit):
            os.environ["APTOS_CORE_BRANCH"] = ""
            os.environ["APTOS_NODE_CARGO_TOML"] = ""
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
        os.environ["APTOS_CORE_BRANCH"] = "release-1.0"
        os.environ["APTOS_NODE_CARGO_TOML"] = tmp.name
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
        os.environ["APTOS_CORE_BRANCH"] = "release-1.1"
        os.environ["APTOS_NODE_CARGO_TOML"] = tmp.name
        with self.assertRaises(SystemExit):
            main()  # this should not work
