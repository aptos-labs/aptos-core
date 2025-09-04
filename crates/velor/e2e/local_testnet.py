# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

# This file contains functions for running the localnet.

import logging
import subprocess
import time

import requests
from common import FAUCET_PORT, METRICS_PORT, NODE_PORT, Network, build_image_name

LOG = logging.getLogger(__name__)

# Run a localnet in a docker container. We choose to detach here and we'll
# stop running it later using the container name.
def run_node(network: Network, image_repo_with_project: str, pull=True):
    image_name = build_image_name(image_repo_with_project, network)
    container_name = f"velor-tools-{network}"
    LOG.info(f"Trying to run velor CLI localnet from image: {image_name}")

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

    # If debug logging is enabled show the output of the command to run the container.
    kwargs = {"check": True}
    if LOG.getEffectiveLevel() > 10:
        kwargs = {**kwargs, **{"stdout": subprocess.PIPE, "stderr": subprocess.PIPE}}

    args = [
        "docker",
        "run",
    ]

    if pull:
        args += ["--pull", "always"]

    args += [
        "--detach",
        "--name",
        container_name,
        "-p",
        f"{NODE_PORT}:{NODE_PORT}",
        "-p",
        f"{METRICS_PORT}:{METRICS_PORT}",
        "-p",
        f"{FAUCET_PORT}:{FAUCET_PORT}",
        image_name,
        "velor",
        "node",
        "run-local-testnet",
        "--with-faucet",
    ]

    # Run the container.
    LOG.debug("Running command: %s", " ".join(args))
    subprocess.run(
        args,
        **kwargs,
    )

    LOG.info(f"Running velor CLI localnet from image: {image_name}. Container name: {container_name}")
    return container_name


# Stop running the detached node.
def stop_node(container_name: str):
    LOG.info(f"Stopping container: {container_name}")
    subprocess.check_output(["docker", "stop", container_name])
    LOG.info(f"Stopped container: {container_name}")


# Query the node and faucet APIs until they start up or we timeout.
def wait_for_startup(container_name: str, timeout: int):
    LOG.info(f"Waiting for node and faucet APIs for {container_name} to come up")
    count = 0
    api_response = None
    faucet_response = None
    while True:
        try:
            api_response = requests.get(f"http://127.0.0.1:{NODE_PORT}/v1")
            # Try to query the legacy faucet health endpoint first. TODO: Remove this
            # once all localnet images we use have the new faucet in them.
            faucet_response = requests.get(f"http://127.0.0.1:{FAUCET_PORT}/health")
            if faucet_response.status_code == 404:
                # If that fails, try the new faucet health endpoint.
                faucet_response = requests.get(f"http://127.0.0.1:{FAUCET_PORT}/")
            if api_response.status_code != 200 or faucet_response.status_code != 200:
                raise RuntimeError(
                    f"API or faucet not ready. API response: {api_response}. "
                    f"Faucet response: {faucet_response}"
                )
            break
        except Exception:
            if count >= timeout:
                LOG.error(f"Timeout while waiting for node / faucet to come up")
                raise
            count += 1
            time.sleep(1)
    LOG.info(f"Node and faucet APIs for {container_name} came up")
