# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Docker image behavior for the credential-free protected E2E job."""

import importlib.util
import pathlib
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch


CLI_DIR = pathlib.Path(__file__).resolve().parent
FAUCET_DIR = CLI_DIR.parent.parent / "aptos-faucet" / "integration-tests"


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {name: module}):
        spec.loader.exec_module(module)
    return module


class OfflineImagesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        common = types.ModuleType("common")
        common.FAUCET_PORT = 8081
        common.METRICS_PORT = 9101
        common.NODE_PORT = 8080
        common.Network = str
        common.AccountInfo = object
        common.build_image_name = lambda repo, tag: f"{repo}/tools:{tag}"
        requests = types.ModuleType("requests")
        sdk = types.ModuleType("aptos_sdk")
        async_client = types.ModuleType("aptos_sdk.async_client")
        async_client.RestClient = lambda url: url
        account_address = types.ModuleType("aptos_sdk.account_address")
        account_address.AccountAddress = object
        with patch.dict(sys.modules, {
            "common": common,
            "requests": requests,
            "aptos_sdk": sdk,
            "aptos_sdk.async_client": async_client,
            "aptos_sdk.account_address": account_address,
        }):
            cls.cli_localnet = load_module("cli_localnet_offline_test", CLI_DIR / "local_testnet.py")
            cls.faucet_localnet = load_module("faucet_localnet_offline_test", FAUCET_DIR / "local_testnet.py")
            cls.helpers = load_module("cli_helpers_offline_test", CLI_DIR / "test_helpers.py")

    def test_cli_image_is_inspected_offline_and_missing_image_fails(self):
        helper = self.helpers.RunHelper("/tmp", "aptos-ci", "pr", None, "devnet")
        with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "true"}), patch.object(
            self.helpers.subprocess, "check_output", return_value=b"[]"
        ) as run:
            helper.prepare_cli()
            run.assert_called_once_with(["docker", "image", "inspect", "aptos-ci/tools:pr"])
            run.side_effect = subprocess.CalledProcessError(1, run.call_args.args[0])
            with self.assertRaises(subprocess.CalledProcessError):
                helper.prepare_cli()

    def test_cli_default_still_pulls_and_offline_run_never_pulls(self):
        with tempfile.TemporaryDirectory() as directory:
            helper = self.helpers.RunHelper(directory, "aptos-ci", "pr", None, "devnet")
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "false"}), patch.object(
                self.helpers.subprocess, "check_output", return_value=b"ok"
            ) as run:
                helper.prepare_cli()
                run.assert_called_once_with(["docker", "pull", "aptos-ci/tools:pr"])
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "true"}), patch.object(
                self.helpers.subprocess, "run"
            ) as run:
                run.return_value.stdout = ""
                run.return_value.stderr = ""
                helper.run_command("version", ["aptos", "--version"])
                command = run.call_args.args[0]
                self.assertEqual("never", command[command.index("--pull") + 1])

    def test_cli_localnet_offline_overrides_pull_flag_and_defaults_remain(self):
        with patch.object(self.cli_localnet.subprocess, "run") as run:
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "true"}):
                self.cli_localnet.run_node("devnet", "aptos-ci", pull=True)
                command = run.call_args.args[0]
                self.assertEqual("never", command[command.index("--pull") + 1])
                self.cli_localnet.run_node("devnet", "aptos-ci", pull=False)
                command = run.call_args.args[0]
                self.assertEqual("never", command[command.index("--pull") + 1])
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "false"}):
                self.cli_localnet.run_node("devnet", "aptos-ci")
                command = run.call_args.args[0]
                self.assertEqual("always", command[command.index("--pull") + 1])
                self.cli_localnet.run_node("devnet", "aptos-ci", pull=False)
                self.assertNotIn("--pull", run.call_args.args[0])

    def test_faucet_localnet_offline_never_pulls_and_default_always_pulls(self):
        with patch.object(self.faucet_localnet.subprocess, "run"), patch.object(
            self.faucet_localnet.subprocess, "check_output"
        ) as run:
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "true"}):
                self.faucet_localnet.run_node("pr", "aptos-ci", "/tmp")
                command = run.call_args.args[0]
                self.assertEqual("never", command[command.index("--pull") + 1])
                run.side_effect = subprocess.CalledProcessError(1, command)
                with self.assertRaises(subprocess.CalledProcessError):
                    self.faucet_localnet.run_node("pr", "aptos-ci", "/tmp")
            run.side_effect = None
            with patch.dict("os.environ", {"APTOS_E2E_OFFLINE_IMAGES": "false"}):
                self.faucet_localnet.run_node("pr", "aptos-ci", "/tmp")
                command = run.call_args.args[0]
                self.assertEqual("always", command[command.index("--pull") + 1])


if __name__ == "__main__":
    unittest.main()
