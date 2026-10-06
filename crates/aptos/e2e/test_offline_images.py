# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Docker image behavior for the credential-free protected E2E job."""

import json
import pathlib
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

from hypothesis import given

CI_DIR = next(parent / ".github" / "ci" for parent in pathlib.Path(__file__).resolve().parents
              if (parent / ".github" / "ci").is_dir())
if str(CI_DIR) in sys.path:
    sys.path.remove(str(CI_DIR))
sys.path.insert(0, str(CI_DIR))
from tests.harness_support import DIGESTS, TAGS, load_common, load_module
from tests.property_support import configure_profiles

configure_profiles()

CLI_DIR = pathlib.Path(__file__).resolve().parent
FAUCET_DIR = CLI_DIR.parent.parent / "aptos-faucet" / "integration-tests"


class OfflineImagesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cli_common = load_common("cli_common_offline_contract", CLI_DIR / "common.py")
        faucet_common = load_common("faucet_common_offline_contract", FAUCET_DIR / "common.py")
        requests = types.ModuleType("requests")
        sdk = types.ModuleType("aptos_sdk")
        async_client = types.ModuleType("aptos_sdk.async_client")
        async_client.RestClient = lambda url: url
        account_address = types.ModuleType("aptos_sdk.account_address")
        account_address.AccountAddress = cli_common.AccountInfo.__annotations__["account_address"]
        with patch.dict(sys.modules, {
            "common": cli_common,
            "requests": requests,
            "aptos_sdk": sdk,
            "aptos_sdk.async_client": async_client,
            "aptos_sdk.account_address": account_address,
        }):
            cls.cli_localnet = load_module("cli_localnet_offline_test", CLI_DIR / "local_testnet.py")
            with patch.dict(sys.modules, {"common": faucet_common}):
                cls.faucet_localnet = load_module("faucet_localnet_offline_test", FAUCET_DIR / "local_testnet.py")
            cls.helpers = load_module("cli_helpers_offline_test", CLI_DIR / "test_helpers.py")

    @given(TAGS, DIGESTS)
    def test_generated_offline_cli_inspects_digest_and_never_pulls(self, tag, digest):
        repository = "registry.example/repo/tools"
        image = f"{repository}@{digest}"
        for flag in (None, "true", "false", "TRUE", "1", ""):
            env = {"PROTECTED_IMAGE_TAG": tag, "PROTECTED_IMAGE_DIGESTS":
                   json.dumps({repository: digest})}
            if flag is not None:
                env["APTOS_E2E_OFFLINE_IMAGES"] = flag
            with tempfile.TemporaryDirectory() as directory, patch.dict("os.environ", env, clear=True):
                helper = self.helpers.RunHelper(directory, "registry.example/repo", tag, None, "devnet")
                expected = ["docker", "image", "inspect", image] if flag == "true" else ["docker", "pull", image]
                with patch.object(self.helpers.subprocess, "check_output", return_value=b"[]") as prepare:
                    helper.prepare_cli()
                    prepare.assert_called_once_with(expected)
                with patch.object(self.helpers.subprocess, "check_output",
                                  side_effect=subprocess.CalledProcessError(1, expected)) as missing:
                    with self.assertRaises(subprocess.CalledProcessError):
                        helper.prepare_cli()
                    missing.assert_called_once_with(expected)
                with patch.object(self.helpers.subprocess, "run",
                                  return_value=subprocess.CompletedProcess([], 0, stdout="", stderr="")) as run:
                    helper.run_command("version", ["aptos", "--version"])
                    command = run.call_args.args[0]
                    prefix = ["docker", "run", "-e", "HOME=/tmp", "--rm"]
                    if flag == "true":
                        prefix += ["--pull", "never"]
                    self.assertEqual(prefix + ["--network", "host", "-i", "-v", directory + ":/tmp",
                                              "--workdir", "/tmp", image, "aptos", "--version"], command)
                    self.assertTrue(run.call_args.kwargs["check"])

    @given(TAGS, DIGESTS)
    def test_generated_localnets_apply_every_flag_and_pull_combination(self, tag, digest):
        repository = "registry.example/repo/tools"
        image = f"{repository}@{digest}"
        for flag in (None, "true", "false", "TRUE", "1", ""):
            env = {"PROTECTED_IMAGE_TAG": tag, "PROTECTED_IMAGE_DIGESTS":
                   json.dumps({repository: digest})}
            if flag is not None:
                env["APTOS_E2E_OFFLINE_IMAGES"] = flag
            with patch.dict("os.environ", env, clear=True):
                for pull in ({}, {"pull": False}, {"pull": True}):
                    with patch.object(self.cli_localnet.subprocess, "run") as run:
                        name = self.cli_localnet.run_node(tag, "registry.example/repo", **pull)
                        policy = (["--pull", "never"] if flag == "true"
                                  else ["--pull", "always"] if pull.get("pull", True) else [])
                        expected = ["docker", "run", *policy, "--detach", "--name", "aptos-tools-" + tag,
                                    "-p", "8080:8080", "-p", "9101:9101", "-p", "8081:8081",
                                    image, "aptos", "node", "run-local-testnet", "--with-faucet"]
                        self.assertEqual("aptos-tools-" + tag, name)
                        self.assertEqual(expected, run.call_args.args[0])
                        self.assertTrue(run.call_args.kwargs["check"])
                        self.assertEqual(3, run.call_count)
                        self.assertEqual(["docker", "container", "ls"], run.call_args_list[0].args[0])
                        self.assertEqual(["docker", "rm", "-f", name], run.call_args_list[1].args[0])
                    with patch.object(self.cli_localnet.subprocess, "run",
                                      side_effect=[None, None, subprocess.CalledProcessError(1, expected)]):
                        with self.assertRaises(subprocess.CalledProcessError):
                            self.cli_localnet.run_node(tag, "registry.example/repo", **pull)
                with tempfile.TemporaryDirectory() as directory, patch.object(
                    self.faucet_localnet.subprocess, "run"
                ) as setup, patch.object(self.faucet_localnet.subprocess, "check_output", return_value=b"ok") as run:
                    name = self.faucet_localnet.run_node(tag, "registry.example/repo", directory)
                    expected = ["docker", "run", "--pull", "never" if flag == "true" else "always",
                                "--name", "local-testnet-" + tag, "--detach", "-p", "8080:8080",
                                "-v", directory + ":/mymount", image, "aptos", "node", "run-local-testnet",
                                "--test-dir", "/mymount", "--no-faucet", "--no-txn-stream"]
                    self.assertEqual("local-testnet-" + tag, name)
                    run.assert_called_once_with(expected)
                    self.assertEqual(["docker", "container", "ls"], setup.call_args_list[0].args[0])
                    self.assertEqual(["docker", "rm", "-f", name], setup.call_args_list[1].args[0])
                    run.side_effect = subprocess.CalledProcessError(1, expected)
                    with self.assertRaises(subprocess.CalledProcessError):
                        self.faucet_localnet.run_node(tag, "registry.example/repo", directory)


if __name__ == "__main__":
    unittest.main()
