from .create_testnet import SystemContext
import pangu_lib.util as util
from kubernetes import client
from test_framework.logging import log


def transaction_emitter_main(testnet_name: str, system_context: SystemContext):
    pod: client.V1Pod = create_pod(testnet_name)
    system_context.kubernetes.create_resource(pod, testnet_name)


def create_pod(testnet_name: str) -> client.V1Pod:
    container: client.V1Container = client.V1Container(
        name=f"{testnet_name}-tx-emitter",
        image="aptoslabs/tools:devnet_performance",
        env=[
            client.V1EnvVar(name="RUST_BACKTRACE", value="1"),
            client.V1EnvVar(name="REUSE_ACC", value="1"),
        ],
        command=["aptos-transaction-emitter"],
        resources=client.V1ResourceRequirements(
            requests={"cpu": "1", "memory": "1Gi"},  # Check if too much/not enough
            limits={"cpu": "2", "memory": "2Gi"},  # Check if too much/not enough
        ),
    )

    pod_spec: client.V1PodSpec = client.V1PodSpec(
        restart_policy="Never",
        containers=[container],
    )

    pod: client.V1Pod = client.V1Pod(
        api_version="v1",
        kind="Pod",
        metadata=client.V1ObjectMeta(name=f"{testnet_name}-tx-emitter"),
        spec=pod_spec,
    )

    return pod
