from test_framework.kubernetes import Kubernetes
from test_framework.logging import log
from pangu_lib.node_commands.start_node import start_node_main
from pangu_lib.node_commands.stop_node import stop_node_main


def restart_node_main(testnet_name: str, node_name: str, kubernetes: Kubernetes):
    """Restarts a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): the statefulset/node name
        kubernetes (Kubernetes): kubernetes abstraction
    """
    #
    # Scale statefulset function waits for scaling by default, so no need to sleep in between scaling.
    log.info(f"Restarting node {node_name} in testnet {testnet_name}")
    stop_node_main(testnet_name, node_name, kubernetes)
    start_node_main(testnet_name, node_name, kubernetes)
    log.info(f"Node {node_name} in testnet {testnet_name} has been restarted!")
