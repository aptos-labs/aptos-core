from .create_testnet import SystemContext
import pangu_lib.util as util
from kubernetes import client
from test_framework.logging import log
from typing import List
import pangu_lib.util as util

import random
import string
import time


def transaction_emitter_main(
    testnet_name: str,
    dry_run: bool,
    workspace: str,
    args: List[str],
    system_context: SystemContext,
    timeout: int = 360,
    ask_for_delete: bool = True,
):
    #
    # Create command array
    command_array = ["velor-transaction-emitter"]
    command_array.extend(args)

    #
    # Create pod name
    random_postfix: str = "".join(
        random.choices(string.ascii_lowercase + string.digits, k=8)
    )
    pod_name = f"{testnet_name}-tx-emitter-{random_postfix}"

    #
    # Create Pod
    log.info("Creating a transaction emitter pod...")
    pod: client.V1Pod = create_transaction_emitter_pod(pod_name, command_array)

    #
    # If dry run, dump pod yaml and return
    if dry_run:
        util.kubernetes_object_to_yaml(
            f"{workspace}/{pod_name}.yaml",
            pod,
            system_context.filesystem,
        )
        log.info(
            f'Transaction emitter pod yaml dumped to "{workspace}/{pod_name}.yaml"...'
        )
        return

    #
    # Apply pod
    system_context.kubernetes.create_resource(pod, testnet_name)
    log.info("Transaction emitter pod created...")

    #
    # Get logs
    command = ["kubectl", "logs", "-f", pod_name, "-n", testnet_name]
    time_passed = 0
    while time_passed < timeout:
        log.info(
            f"Attempting to get logs from transaction emitter, time passed: {time_passed}..."
        )
        if system_context.shell.run(command, stream_output=True).succeeded():
            log.info("Successfully got logs from transaction emitter")
            break
        time_passed += 5
        time.sleep(5)

    #
    # Check if we timed out
    if time_passed == timeout:
        log.error("Failed to get logs from transaction emitter")

    #
    # Ask for delete flag for running Pangu without being able to use stdin (e.g. in CI/Forge)
    if ask_for_delete:
        #
        # Delete pod
        user_input = input(
            '-------------------------------------------------------\n- The transaction emitter logs are complete. \n- Type "delete" to delete the transaction emitter pod\n-------------------------------------------------------\n'
        )
        if user_input == "delete":
            system_context.kubernetes.delete_resource(pod, testnet_name)


def create_transaction_emitter_pod(
    pods_name: str, command_array: list[str]
) -> client.V1Pod:
    container: client.V1Container = client.V1Container(
        name=pods_name,
        image=util.DEFAULT_TRANSACTION_EMITTER_IMAGE,
        env=[
            client.V1EnvVar(name="RUST_BACKTRACE", value="1"),
            client.V1EnvVar(name="REUSE_ACC", value="1"),
        ],
        command=command_array,
        resources=client.V1ResourceRequirements(
            requests={"cpu": "15", "memory": "26Gi"},  # Check if too much/not enough
            limits={"cpu": "15", "memory": "26Gi"},  # Check if too much/not enough
        ),
    )

    pod_spec: client.V1PodSpec = client.V1PodSpec(
        restart_policy="Never",
        containers=[container],
    )

    pod: client.V1Pod = client.V1Pod(
        api_version="v1",
        kind="Pod",
        metadata=client.V1ObjectMeta(
            name=pods_name,
            labels={"type": util.TX_EMITTER_TYPE},
        ),
        spec=pod_spec,
    )

    return pod
