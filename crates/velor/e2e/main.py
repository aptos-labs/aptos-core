#!/usr/bin/env python3

# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This script is how we orchestrate running a localnet and then running CLI tests against it. There are two different CLIs used for this:

1. Base: For running the localnet. This is what the --base-network flag and all other flags starting with --base are for.
2. Test: The CLI that we're testing. This is what the --test-cli-tag / --test-cli-path and all other flags starting with --test are for.

Example (testing CLI in image):
  python3 main.py --base-network testnet --test-cli-tag mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3

Example (testing locally built CLI binary):
  python3 main.py --base-network devnet --test-cli-path ~/velor-core/target/release/velor

This means, run the CLI test suite using a CLI built from mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3 against a localnet built from the testnet branch of velor-core.

Example (using a different image repo):
  See ~/.github/workflows/cli-e2e-tests.yaml

When the test suite is complete, it will tell you which tests passed and which failed. To further debug a failed test, you can check the output in --working-directory, there will be files for each test containing the command run, stdout, stderr, and any exception.
"""

import argparse
import asyncio
import logging
import os
import pathlib
import platform
import shutil
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
from cases.init import test_velor_header_included, test_init, test_metrics_accessible
from cases.move import (
    test_move_compile,
    test_move_compile_script,
    test_move_publish,
    test_move_run,
    test_move_view,
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


def parse_args():
    # You'll notice there are two argument "prefixes", base and test. These refer to
    # cases 1 and 2 in the top-level comment.
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("-d", "--debug", action="store_true")
    parser.add_argument(
        "--image-repo-with-project",
        default="velorlabs",
        help=(
            "What docker image repo (+ project) to use for the localnet. "
            "By default we use Docker Hub: %(default)s (so, just velorlabs for the "
            "project since Docker Hub is the implied default repo). If you want to "
            "specify a different repo, it might look like this: "
            "docker.pkg.github.com/velorlabs/velor-core"
        ),
    )
    parser.add_argument(
        "--base-network",
        required=True,
        type=Network,
        choices=list(Network),
        help="What branch the Velor CLI used for the localnet should be built from",
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
        help="Path to CLI binary we want to test, e.g. /home/dport/velor-core/target/release/velor",
    )
    parser.add_argument(
        "--working-directory",
        default="/tmp/velor-cli-tests",
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

    # Make sure the velor-cli header is included on the original request
    test_velor_header_included(run_helper)

    # Run move subcommand group tests.
    test_move_compile(run_helper)
    test_move_compile_script(run_helper)
    test_move_publish(run_helper)
    test_move_run(run_helper)
    test_move_view(run_helper)

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
    container_name = run_node(
        args.base_network, args.image_repo_with_project, not args.no_pull_always
    )

    # We run these in a try finally so that if something goes wrong, such as the
    # localnet not starting up correctly or some unexpected error in the
    # test framework, we still stop the node + faucet.
    try:
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
        # Stop the node + faucet.
        stop_node(container_name)

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
