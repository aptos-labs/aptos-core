# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

# This file contains functions for running the localnet.

import logging
import subprocess
import time

import requests
from common import NODE_PORT, Network, build_image_name

LOG = logging.getLogger(__name__)

# Run a localnet in a docker container. We choose to detach here and we'll
# stop running it later using the container name. For an explanation of these
# arguments, see the argument parser in main.py.
def run_node(network: Network, image_repo_with_project: str, external_test_dir: str):
    image_name = build_image_name(image_repo_with_project, network)
    container_name = f"local-testnet-{network}"
    internal_mount_path = "/mymount"
    LOG.info(f"Trying to run localnet from image: {image_name}")

    # Confirm that the Docker daemon is running.
    try:
        subprocess.run(
            ["docker", "container", "ls"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=True,
        )
    except:
        LOG.error("Failed to connect to Docker. Is it installed and running?")
        raise

    # First delete the existing container if there is one with the same name.
    subprocess.run(
        ["docker", "rm", "-f", container_name],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # Run the container.
    subprocess.check_output(
        [
            "docker",
            "run",
            "--pull",
            "always",
            "--name",
            container_name,
            "--detach",
            # Expose the API port.
            "-p",
            f"{NODE_PORT}:{NODE_PORT}",
            # Mount the external test directory into the container.
            "-v",
            f"{external_test_dir}:{internal_mount_path}",
            image_name,
            "velor",
            "node",
            "run-local-testnet",
            "--test-dir",
            internal_mount_path,
            "--no-faucet",
            "--no-txn-stream",
        ],
    )
    LOG.info(f"Running localnet from image: {image_name}")
    return container_name


# Stop running the detached node.
def stop_node(container_name: str):
    LOG.info(f"Stopping container: {container_name}")
    subprocess.check_output(["docker", "stop", container_name])
    LOG.info(f"Stopped container: {container_name}")


# Query the node until the API comes up, or we timeout.
def wait_for_startup(container_name: str, timeout: int):
    LOG.info(f"Waiting for node API for {container_name} to come up")
    count = 0
    api_response = None
    while True:
        try:
            api_response = requests.get(f"http://127.0.0.1:{NODE_PORT}/v1")
            if api_response.status_code != 200:
                raise RuntimeError(f"API not ready. API response: {api_response}")
            break
        except Exception:
            if count >= timeout:
                LOG.error(f"Timeout while waiting for node to come up")
                raise
            count += 1
            time.sleep(1)
    LOG.info(f"Node API for {container_name} came up")
