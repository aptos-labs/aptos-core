#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This script is how we orchestrate running a localnet and then the faucet
integration tests against it.

Example invocation:
  python3 main.py --local-testnet-network devnet

This would run a localnet built from the devnet release branch and then run the
faucet integration tests against it.

The script confirms that pre-existing conditions are suitable, e.g. checking that a
Redis instance is alive.
"""

import argparse
import logging
import os
import platform
import shutil
import sys

from common import VALID_NETWORK_OPTIONS, Network, network_from_str
from local_testnet import run_node, stop_node, wait_for_startup
from prechecks import check_redis_is_running
from tests import run_faucet_integration_tests

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
        required=True,
        choices=VALID_NETWORK_OPTIONS,
        help=(
            "What branch the Aptos CLI used for the localnet should be built "
            'from. If "custom", --tag must be set.'
        ),
    )
    parser.add_argument(
        "--skip-node",
        action="store_true",
        help="Skip running the node. You must run it yourself and copy the mint key yourself.",
    )
    parser.add_argument(
        "--tag",
        help=(
            'If --base-network is set to "custom", this must be set to the image tag'
            "to use. Otherwise this has no effect."
        ),
    )
    parser.add_argument(
        "--base-startup-timeout",
        type=int,
        default=30,
        help="Timeout in seconds for waiting for node and faucet to start up",
    )
    parser.add_argument(
        "--external-test-dir",
        default="/tmp/testnet",
        help="Where to mount the test dir that the node is writing to",
    )
    args = parser.parse_args()
    return args


def main():
    args = parse_args()

    if args.debug:
        logging.getLogger().setLevel(logging.DEBUG)
        LOG.debug("Debug logging enabled")
    else:
        logging.getLogger().setLevel(logging.INFO)

    # If we're on Mac and DOCKER_DEFAULT_PLATFORM is not already set, set it to
    # linux/amd64 since we only publish images for that platform.
    if platform.system().lower() == "darwin" and platform.processor().lower().startswith("arm"):
        if not os.environ.get("DOCKER_DEFAULT_PLATFORM"):
            os.environ["DOCKER_DEFAULT_PLATFORM"] = "linux/amd64"
            LOG.info(
                "Detected ARM Mac and DOCKER_DEFAULT_PLATFORM was not set, setting it "
                "to linux/amd64"
            )

    # Build the Network.
    network = network_from_str(args.base_network, args.tag)

    # Verify that a local Redis instance is running. This is just a basic check that
    # something is listening at the expected port.
    check_redis_is_running()

    if not args.skip_node:
        # Run a node and wait for it to start up.
        container_name = run_node(
            network, args.image_repo_with_project, args.external_test_dir
        )
        wait_for_startup(container_name, args.base_startup_timeout)

        # Copy the mint key from the node to where the integration tests expect it to be.
        copy_mint_key(args.external_test_dir)

    # Build and run the faucet integration tests.
    run_faucet_integration_tests()

    if not args.skip_node:
        # Stop the localnet.
        stop_node(container_name)

    return True


def copy_mint_key(external_test_dir: str):
    key_name = "mint.key"
    source_path = os.path.join(external_test_dir, key_name)
    new_path = os.path.join("/tmp", key_name)
    try:
        shutil.copyfile(source_path, new_path)
    except FileNotFoundError as e:
        raise RuntimeError(
            f"Could not find mint key at expected source path: {source_path}"
        ) from e
    LOG.info(
        f"Copied mint key from {source_path} to the path the integration "
        f"tests expect: {new_path}"
    )


if __name__ == "__main__":
    if main():
        sys.exit(0)
    else:
        sys.exit(1)
