from test_framework.kubernetes import Kubernetes
from test_framework.logging import log


def stop_node_main(testnet_name: str, node_name: str, kubernetes: Kubernetes):
    """stop a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): the statefulset/node name
        kubernetes (Kubernetes): kubernetes abstraction
    """
    log.info(f"Stopping node {node_name} in testnet {testnet_name}")
    kubernetes.scale_stateful_set(testnet_name, node_name, 0)
    log.info(f"Node {node_name} in testnet {testnet_name} has been stopped!")
