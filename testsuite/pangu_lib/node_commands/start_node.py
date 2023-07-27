from test_framework.kubernetes import Kubernetes
from test_framework.logging import log


def start_node_main(testnet_name: str, node_name: str, kubernetes: Kubernetes):
    """start a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): the statefulset/node name
        kubernetes (Kubernetes): kubernetes abstraction
    """
    log.info(f"Starting node {node_name} in testnet {testnet_name}")
    kubernetes.scale_stateful_set(testnet_name, node_name, 1)
    log.info(f"Node {node_name} in testnet {testnet_name} has been started!")
