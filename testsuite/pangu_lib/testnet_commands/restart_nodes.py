from test_framework.kubernetes import Kubernetes, KubernetesResource
from typing import List
from test_framework.logging import log
from .get_testnet import get_singular_testnet
from pangu_lib.node_commands.restart_node import restart_node_main
from concurrent import futures


def restart_nodes_main(testnet_name: str, kubernetes: Kubernetes):
    """Restarts all nodes in a testnet

    Args:
        testnet_name (str): the namespace/testnet
        kubernetes (Kubernetes): kubernetes abstraction
    """
    log.info(f"Restarting all nodes in testnet {testnet_name}")

    nodes: List[KubernetesResource] = get_singular_testnet(
        testnet_name, kubernetes
    ).node_statefulsets
    with futures.ThreadPoolExecutor() as executor:
        node_futures = []
        for node in nodes:
            node_name: str = node.metadata.name  # type: ignore
            future = executor.submit(
                _restart_node_wrapper, testnet_name, node_name, kubernetes
            )
            node_futures.append(future)  # type: ignore
        futures.wait(node_futures)
        for future in node_futures:  # type: ignore
            if future.exception() is not None:  # type: ignore
                raise future.exception()  # type: ignore

    log.info(f"All nodes in testnet {testnet_name} have been restarted!")


def _restart_node_wrapper(testnet_name: str, node_name: str, kubernetes: Kubernetes):
    try:
        restart_node_main(testnet_name, node_name, kubernetes)
    except Exception as e:
        log.error(f"Failed to restart node {node_name} in testnet {testnet_name}")
        raise e
