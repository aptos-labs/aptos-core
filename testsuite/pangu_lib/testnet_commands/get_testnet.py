from test_framework.kubernetes import Kubernetes, KubernetesResource
from test_framework.logging import log
from typing import List, Any
from kubernetes import client
from tabulate import tabulate
from datetime import datetime, timezone
from pangu_lib.util import strfdelta
import json
import sys
from pangu_lib.util import NodeType
from pangu_lib.util import TX_EMITTER_TYPE


class PanguTestnet:
    name: str
    phase: str
    age: str
    namespace: KubernetesResource
    node_statefulsets: List[KubernetesResource]
    node_pods: List[KubernetesResource]
    num_validators: int
    num_validator_fullnodes: int
    num_public_fullnodes: int
    num_validators_active: int
    num_validator_fullnodes_active: int
    num_public_fullnodes_active: int


def get_testnet_main(testnet_name: str, output_format: str, kubernetes: Kubernetes):
    """get testnet main

    Args:
        testnet_name (str): testnet name
        json_flag (bool): whether to print in json or not
        kubernetes (Kubernetes): kubernetes abstraction
    """
    #
    # Get all testnets
    log.info("Getting testnet(s)...")
    if testnet_name == "":
        print_all_testnets(output_format, kubernetes)
    else:
        print_singular_testnet(testnet_name, output_format, kubernetes)


def print_all_testnets(output_format: str, kubernetes: Kubernetes):
    """print all testnets

    Args:
        kubernetes (Kubernetes): kubernetes abstraction
        output_format (str): whether to print in json or not
    """
    namespaces: List[KubernetesResource] = kubernetes.get_resources(
        type=client.V1Namespace
    )
    pangu_testnets: List[PanguTestnet] = []
    for namespace in namespaces:
        if namespace.metadata.name.startswith("pangu-"):  # type: ignore
            pangu_testnet: PanguTestnet = get_singular_testnet(namespace.metadata.name, kubernetes)  # type: ignore
            pangu_testnets.append(pangu_testnet)  # type: ignore

    live_testnets: List[tuple[str, str, str, str]] = [
        (
            pangu_testnet.name,
            pangu_testnet.phase,
            pangu_testnet.age,
            str(
                pangu_testnet.num_validators_active
                + pangu_testnet.num_validator_fullnodes_active
                + pangu_testnet.num_public_fullnodes_active
            )
            + "/"
            + str(
                pangu_testnet.num_validators
                + pangu_testnet.num_validator_fullnodes
                + pangu_testnet.num_public_fullnodes
            ),
        )
        for pangu_testnet in pangu_testnets
    ]

    table_headers = ["NAME", "STATUS", "AGE", "NODES"]
    if output_format == "json":
        data = {"headers": table_headers, "testnets": live_testnets}  # type: ignore
        print(json.dumps(data), file=sys.stdout)
    else:
        table = tabulate(live_testnets, headers=table_headers)  # type: ignore
        print("\n\n" + table + "\n")


def print_singular_testnet(
    testnet_name: str, output_format: str, kubernetes: Kubernetes
):
    """print testnet info

    Args:
        testnet_name (str): testnet name
        output_format (str): whether to print in json or not
        kubernetes (Kubernetes): kubernetes abstraction
    """
    try:
        pangu_testnet: PanguTestnet = get_singular_testnet(testnet_name, kubernetes)
    except Exception as e:
        raise e
    live_pods: Any = [
        (
            sts.metadata.name,  # type: ignore
            sts.spec.replicas,  # type: ignore
            strfdelta(datetime.now(timezone.utc) - sts.metadata.creation_timestamp),  # type: ignore
            sts.metadata.labels["type"],  # type: ignore
        )
        for sts in pangu_testnet.node_statefulsets
    ]
    table_headers = ["NAME", "READY", "AGE", "TYPE"]
    if output_format == "json":
        data: Any = {"headers": table_headers, "pods": live_pods}
        print(json.dumps(data), file=sys.stdout)
    else:
        table = tabulate(live_pods, headers=table_headers)
        print("\n\n" + table + "\n")


def get_singular_testnet(testnet_name: str, kubernetes: Kubernetes) -> PanguTestnet:
    """get singular testnet

    Args:
        testnet_name (str): testnet name
        kubernetes (Kubernetes): kubernetes abstraction

    Returns:
        PanguTestnet: PanguTestnet object
    """
    #
    # Get all namespaces
    namespaces: List[KubernetesResource] = kubernetes.get_resources(
        type=client.V1Namespace
    )
    #
    # Find the namespace that corrisponds to the testnet name
    namespace: client.V1Namespace = None  # type: ignore
    for curr_namespace in namespaces:
        if (
            curr_namespace.metadata is not None
            and curr_namespace.metadata.name == testnet_name
        ):
            namespace = curr_namespace  # type: ignore
            break
    #
    # If the namespace does not exist, or the namespace does not start with "pangu-", then the testnet does not exist
    if not namespace or not testnet_name.startswith("pangu-"):
        log.error(f"Testnet {testnet_name} does not exist")
        raise Exception(f"Testnet {testnet_name} does not exist")

    #
    # Create the PanguTestnet object
    pangu_testnet = PanguTestnet()
    pangu_testnet.namespace = namespace
    pangu_testnet.name = namespace.metadata.name  # type: ignore
    pangu_testnet.phase = namespace.status.phase  # type: ignore
    pangu_testnet.age = strfdelta(datetime.now(timezone.utc) - namespace.metadata.creation_timestamp)  # type: ignore
    pangu_testnet.num_validators = 0
    pangu_testnet.num_validator_fullnodes = 0
    pangu_testnet.num_public_fullnodes = 0
    pangu_testnet.num_validators_active = 0
    pangu_testnet.num_validator_fullnodes_active = 0
    pangu_testnet.num_public_fullnodes_active = 0

    #
    # Get all statefulsets, and count the number of each type of node
    sts_objects: List[KubernetesResource] = kubernetes.get_resources(
        type=client.V1StatefulSet, namespace=testnet_name
    )
    for sts in sts_objects:
        type: str = sts.metadata.labels["type"]  # type: ignore
        if type == NodeType.VALIDATOR.value:
            pangu_testnet.num_validators += 1
        elif type == NodeType.VFN.value:
            pangu_testnet.num_validator_fullnodes += 1
        elif type == NodeType.PFN.value:
            pangu_testnet.num_public_fullnodes += 1
        elif type == TX_EMITTER_TYPE:
            pass
        else:
            raise Exception(f"Unknown type: {type}")
    pangu_testnet.node_statefulsets = sts_objects

    #
    # Get all pods, and count the number of each type of node
    pod_objects: List[KubernetesResource] = kubernetes.get_resources(
        type=client.V1Pod, namespace=testnet_name
    )
    for pod in pod_objects:
        # type : str = pod.spec.template.metadata.labels["type"]  #type: ignore
        type: str = pod.metadata.labels["type"]  # type: ignore
        if type == NodeType.VALIDATOR.value:
            pangu_testnet.num_validators_active += 1
        elif type == NodeType.VFN.value:
            pangu_testnet.num_validator_fullnodes_active += 1
        elif type == NodeType.PFN.value:
            pangu_testnet.num_public_fullnodes_active += 1
        elif type == TX_EMITTER_TYPE:
            pass
        else:
            raise Exception(f"Unknown type: {type}")
    pangu_testnet.node_pods = pod_objects

    return pangu_testnet
