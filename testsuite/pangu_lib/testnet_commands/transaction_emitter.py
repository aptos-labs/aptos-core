from .create_testnet import SystemContext
import pangu_lib.util as util
from kubernetes import client
from test_framework.logging import log
from typing import List
import pangu_lib.util as util

import random
import string


def transaction_emitter_main(
    testnet_name: str, args: List[str], system_context: SystemContext
):
    #
    # Create command array
    command_array = ["aptos-transaction-emitter"]
    command_array.extend(args)

    #
    # Create pod name
    random_postfix: str = "".join(
        random.choices(string.ascii_lowercase + string.digits, k=8)
    )
    pod_name = f"{testnet_name}-tx-emitter-{random_postfix}"

    #
    # Create Pod
    log.info("Starting transaction emitter...")
    pod: client.V1Pod = create_transaction_emitter_pod(pod_name, command_array)
    system_context.kubernetes.create_resource(pod, testnet_name)
    log.info("Transaction emitter started...")

    #
    # Get logs
    command = ["kubectl", "logs", "-f", pod_name, "-n", testnet_name]
    while True:
        system_context.shell.run(command, stream_output=True)
        user_input = input(
            '-------------------------------------------------------\n-Press Enter to get the newest logs,\n-Ctrl + c to end looking at the logs...\n-Type "delete" to delete the transaction emitter pod\n-------------------------------------------------------\n'
        )
        if user_input == "delete":
            system_context.kubernetes.delete_resource(pod, testnet_name)
            break


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
            requests={"cpu": "1", "memory": "1Gi"},  # Check if too much/not enough
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
        metadata=client.V1ObjectMeta(name=pods_name),
        spec=pod_spec,
    )

    return pod
