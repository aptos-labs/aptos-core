#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This script orchestrates running a localnet and then running CLI tests against it.

There are two modes for running tests:

## Docker Mode (Default)
Uses Docker containers for both the localnet and CLI testing. Requires two CLIs:
1. Base: For running the localnet (--base-network flag)
2. Test: The CLI being tested (--test-cli-tag or --test-cli-path)

Example (testing CLI in image):
  python3 main.py --base-network testnet --test-cli-tag mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3

Example (testing locally built CLI binary):
  python3 main.py --base-network devnet --test-cli-path ~/aptos-core/target/release/aptos

Example (using a different image repo):
  See ~/.github/workflows/cli-e2e-tests.yaml

## Local Testnet Mode (--use-local-testnet)
Skips Docker and uses a locally built CLI with a local testnet. This mode is recommended for:
- ARM Macs (avoids Docker x86 emulation issues)
- Fast iteration during development
- Testing CLI changes immediately without building Docker images

### Auto-start Mode (Default)
The framework automatically starts and stops the localnet with fresh state on each run:

Example (auto-start with local CLI):
  python3 main.py --use-local-testnet --test-cli-path ~/aptos-core/target/release/aptos

### Manual Mode (--no-auto-start)
For advanced users who want to manually manage the localnet (e.g., for debugging):

Example (manual mode):
  # Terminal 1: Start localnet
  ./target/release/aptos node run-local-testnet --with-faucet --assume-yes

  # Terminal 2: Run tests
  python3 main.py --use-local-testnet --no-auto-start --test-cli-path ~/aptos-core/target/release/aptos

Use manual mode when:
- You want to inspect localnet logs directly
- You're running many test iterations and want to keep localnet running
- You need to debug specific localnet behavior

Note: Manual mode requires cleanup between full test runs to avoid stale state errors.
Run `./reset_tests.sh` if tests fail with "Account has balance X, expected 0" errors.

## Debugging Failed Tests
When tests complete, the script shows which tests passed and which failed.
To debug failures, check the output files in --working-directory (default: /tmp/aptos-cli-tests/out):
- <test>.command - The command that was run
- <test>.stdout - Standard output from the command
- <test>.stderr - Standard error from the command
- <test>.exception - Any exception that was raised (if applicable)

Example:
  cat /tmp/aptos-cli-tests/out/001_test_init.stderr
  cat /tmp/aptos-cli-tests/out/001_test_init.stdout
"""

import argparse
import asyncio
import logging
import os
import pathlib
import platform
import requests
import shutil
import subprocess
import sys
import time

from cases.account import (
    test_account_create_and_transfer,
    test_account_fund_with_faucet,
    test_account_list,
    test_account_lookup_address,
    test_account_resource_account,
    test_account_rotate_key,
)
from cases.config import test_config_show_profiles
from cases.init import test_aptos_header_included, test_init, test_metrics_accessible
from cases.move import (
    test_move_compile,
    test_move_compile_script,
    test_move_publish,
    test_move_run,
    test_move_view,
)
from cases.struct_enum_args import (
    test_enum_simple_variant,
    test_enum_variant_with_fields,
    test_enum_with_nested_struct,
    test_option_legacy_format,
    test_option_variant_format,
    test_publish_struct_enum_module,
    test_struct_argument_nested,
    test_struct_argument_simple,
)
from cases.node import (
    test_node_show_validator_set,
    test_node_update_consensus_key,
    test_node_update_validator_network_address,
)
"""
from cases.stake import (
    test_stake_add_stake,
    test_stake_create_staking_contract,
    test_stake_increase_lockup,
    test_stake_initialize_stake_owner,
    test_stake_request_commission,
    test_stake_set_operator,
    test_stake_set_voter,
    test_stake_unlock_stake,
    test_stake_withdraw_stake_after_unlock,
    test_stake_withdraw_stake_before_unlock,
)
"""
from common import Network
from local_testnet import run_node, stop_node, wait_for_startup
from test_helpers import RunHelper
from test_results import test_results

logging.basicConfig(
    stream=sys.stderr,
    format="%(asctime)s - %(levelname)s - %(message)s",
    level=logging.INFO,
)

LOG = logging.getLogger(__name__)

# Local testnet configuration
LOCALHOST = "127.0.0.1"
API_PORT = 8080
FAUCET_PORT = 8081


def wait_for_service(url, check_fn, timeout, service_name):
    """
    Wait for a service to be ready with custom check function.

    Args:
        url: Service URL to check
        check_fn: Function that takes response and returns True if ready
        timeout: Maximum seconds to wait
        service_name: Name for logging

    Returns:
        True if service became ready, False if timeout
    """
    LOG.info(f"Waiting for {service_name} to start...")
    for i in range(timeout):
        try:
            response = requests.get(url, timeout=1)
            if check_fn(response):
                LOG.info(f"{service_name} is ready!")
                return True
        except (requests.RequestException, requests.Timeout):
            pass

        if i == timeout - 1:
            LOG.error(f"{service_name} failed to start within {timeout} seconds")
            return False
        time.sleep(1)


def parse_args():
    # You'll notice there are two argument "prefixes", base and test. These refer to
    # cases 1 and 2 in the top-level comment.
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("-d", "--debug", action="store_true")
    parser.add_argument(
        "--use-local-testnet",
        action="store_true",
        help=(
            "Skip Docker and use local CLI testnet instead. "
            "By default, will auto-start/stop the testnet. "
            "Use --no-auto-start to manually manage the testnet."
        ),
    )
    parser.add_argument(
        "--no-auto-start",
        action="store_true",
        help=(
            "When using --use-local-testnet, don't auto-start/stop the localnet. "
            "You must manually start it with: aptos node run-local-testnet --with-faucet"
        ),
    )
    parser.add_argument(
        "--image-repo-with-project",
        default="aptoslabs",
        help=(
            "What docker image repo (+ project) to use for the localnet. "
            "By default we use Docker Hub: %(default)s (so, just aptoslabs for the "
            "project since Docker Hub is the implied default repo). If you want to "
            "specify a different repo, it might look like this: "
            "docker.pkg.github.com/aptoslabs/aptos-core"
        ),
    )
    parser.add_argument(
        "--base-network",
        type=Network,
        choices=list(Network),
        default=Network.DEVNET,
        help=(
            "What branch the Aptos CLI used for the localnet should be built from. "
            "Not used with --use-local-testnet. Default: %(default)s"
        ),
    )
    parser.add_argument(
        "--base-startup-timeout",
        type=int,
        default=30,
        help="Timeout in seconds for waiting for node and faucet to start up",
    )
    test_cli_args = parser.add_mutually_exclusive_group(required=True)
    test_cli_args.add_argument(
        "--test-cli-tag",
        help="The image tag for the CLI we want to test, e.g. mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3",
    )
    test_cli_args.add_argument(
        "--test-cli-path",
        help="Path to CLI binary we want to test, e.g. /home/dport/aptos-core/target/release/aptos",
    )
    parser.add_argument(
        "--working-directory",
        default="/tmp/aptos-cli-tests",
        help="Where we'll run CLI commands from (in the host system). Default: %(default)s",
    )
    parser.add_argument(
        "--no-pull-always",
        action="store_true",
        help='If set, do not set "--pull always" when running the localnet. Necessary for using local images.',
    )
    args = parser.parse_args()
    return args


async def run_tests(run_helper):
    # Make sure the metrics port is accessible.
    test_metrics_accessible(run_helper)

    # Run init tests. We run these first to set up the CLI.
    test_init(run_helper)

    # Run config tests.
    test_config_show_profiles(run_helper)

    # Run account tests.
    await test_account_fund_with_faucet(run_helper)
    await test_account_create_and_transfer(run_helper)
    test_account_list(run_helper)
    test_account_lookup_address(run_helper)
    test_account_resource_account(run_helper)

    # Make sure the aptos-cli header is included on the original request
    test_aptos_header_included(run_helper)

    # Run move subcommand group tests.
    test_move_compile(run_helper)
    test_move_compile_script(run_helper)
    test_move_publish(run_helper)
    test_move_run(run_helper)
    test_move_view(run_helper)

    # Run struct/enum transaction argument tests.
    # First publish the struct-enum-args module
    test_publish_struct_enum_module(run_helper)
    test_struct_argument_simple(run_helper)
    test_struct_argument_nested(run_helper)
    test_option_variant_format(run_helper)
    test_option_legacy_format(run_helper)
    test_enum_simple_variant(run_helper)
    test_enum_variant_with_fields(run_helper)
    test_enum_with_nested_struct(run_helper)

    # Run stake subcommand group tests.
    """
    test_stake_initialize_stake_owner(run_helper)
    test_stake_add_stake(run_helper)
    test_stake_withdraw_stake_before_unlock(run_helper)
    test_stake_unlock_stake(run_helper)
    await test_stake_withdraw_stake_after_unlock(run_helper)
    test_stake_increase_lockup(run_helper)
    test_stake_set_operator(run_helper)
    test_stake_set_voter(run_helper)
    await test_stake_create_staking_contract(run_helper)
    test_stake_request_commission(run_helper)
    """

    # Run node subcommand group tests.
    time.sleep(5)
    test_node_show_validator_set(run_helper)
    test_node_update_consensus_key(run_helper)
    test_node_update_validator_network_address(run_helper)

    # WARNING: This has to stay at the end, else key will get rotated
    test_account_rotate_key(run_helper)


async def main():
    args = parse_args()

    if args.debug:
        logging.getLogger().setLevel(logging.DEBUG)
        LOG.debug("Debug logging enabled")
    else:
        logging.getLogger().setLevel(logging.INFO)

    # Create the dir the test CLI will run from.
    shutil.rmtree(args.working_directory, ignore_errors=True)
    pathlib.Path(args.working_directory).mkdir(parents=True, exist_ok=True)

    # If we're on Mac and DOCKER_DEFAULT_PLATFORM is not already set, set it to
    # linux/amd64 since we only publish images for that platform.
    if (
        platform.system().lower() == "darwin"
        and platform.processor().lower().startswith("arm")
    ):
        if not os.environ.get("DOCKER_DEFAULT_PLATFORM"):
            os.environ["DOCKER_DEFAULT_PLATFORM"] = "linux/amd64"
            LOG.info(
                "Detected ARM Mac and DOCKER_DEFAULT_PLATFORM was not set, setting it "
                "to linux/amd64"
            )

    # Run a node + faucet and wait for them to start up.
    localnet_process = None
    if args.use_local_testnet:
        # Skip Docker - use local CLI testnet
        container_name = None

        if not args.no_auto_start:
            # Auto-start the localnet with force-restart
            LOG.info("Auto-starting local testnet with --force-restart")

            # Determine CLI path
            if args.test_cli_path:
                cli_path = os.path.abspath(args.test_cli_path)
            else:
                # If testing a CLI from image, we can't use it for localnet
                LOG.error("Cannot auto-start localnet when using --test-cli-tag")
                LOG.error("Either use --test-cli-path or use --no-auto-start")
                return False

            # Start localnet in background
            localnet_process = subprocess.Popen(
                [cli_path, "node", "run-local-testnet", "--with-faucet", "--force-restart", "--assume-yes"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                universal_newlines=True,
            )
            LOG.info(f"Started localnet process (PID: {localnet_process.pid})")

            # Wait for services to start
            # First wait for API to respond
            if not wait_for_service(
                f"http://{LOCALHOST}:{API_PORT}/v1",
                lambda r: r.status_code == 200,
                60,
                "Local testnet API"
            ):
                if localnet_process.poll() is not None:
                    stdout, stderr = localnet_process.communicate()
                    LOG.error(f"Localnet stdout: {stdout}")
                    LOG.error(f"Localnet stderr: {stderr}")
                return False

            # Then wait for DB to finish bootstrapping
            if not wait_for_service(
                f"http://{LOCALHOST}:{API_PORT}/v1",
                lambda r: r.status_code == 200 and "Error" not in str(r.json()),
                60,
                "Database"
            ):
                return False

            # Finally, wait for faucet to be ready
            if not wait_for_service(
                f"http://{LOCALHOST}:{FAUCET_PORT}/",
                lambda r: r.status_code in [200, 404],
                30,
                "Faucet"
            ):
                return False
        else:
            # Manual mode - verify it's already running
            LOG.info("Using manually-started local testnet")
            try:
                requests.get(f"http://{LOCALHOST}:{API_PORT}/v1", timeout=5)
                LOG.info(f"Local testnet is running on port {API_PORT}")
            except Exception as e:
                LOG.error(f"Local testnet not running on port {API_PORT}: {e}")
                LOG.error("Please start it first with: aptos node run-local-testnet --with-faucet")
                return False
    else:
        # Use Docker
        container_name = run_node(
            args.base_network, args.image_repo_with_project, not args.no_pull_always
        )

    # We run these in a try finally so that if something goes wrong, such as the
    # localnet not starting up correctly or some unexpected error in the
    # test framework, we still stop the node + faucet.
    try:
        if not args.use_local_testnet:
            wait_for_startup(container_name, args.base_startup_timeout)

        # Build the RunHelper object.
        run_helper = RunHelper(
            host_working_directory=args.working_directory,
            image_repo_with_project=args.image_repo_with_project,
            image_tag=args.test_cli_tag,
            cli_path=args.test_cli_path,
            base_network=args.base_network,
        )

        # Prepare the run helper. This ensures in advance that everything needed is there.
        run_helper.prepare()

        # Run tests.
        await run_tests(run_helper)
    finally:
        # Stop the node + faucet (only if we started it).
        if container_name:
            stop_node(container_name)

        # Stop the localnet process if we started it
        if localnet_process:
            LOG.info("Stopping localnet process...")
            localnet_process.terminate()
            try:
                localnet_process.wait(timeout=10)
                LOG.info("Localnet stopped successfully")
            except subprocess.TimeoutExpired:
                LOG.warning("Localnet didn't stop gracefully, killing it...")
                localnet_process.kill()
                localnet_process.wait()

    # Print out the results.
    if test_results.passed:
        LOG.info("These tests passed:")
        for test_name in test_results.passed:
            LOG.info(test_name)

    if test_results.failed:
        LOG.error("These tests failed:")
        for test_name, exception in test_results.failed:
            LOG.error(f"{test_name}: {exception}")
        LOG.info("---")
        LOG.info(
            f"Debug these tests by checking the command, stdout, stderr, and any "
            f"exception information if relevant in {args.working_directory}/out"
        )
        return False

    LOG.info("All tests passed!")

    return True


if __name__ == "__main__":
    if asyncio.run(main()):
        sys.exit(0)
    else:
        sys.exit(1)
