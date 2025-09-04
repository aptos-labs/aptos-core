from test_framework.kubernetes import Kubernetes
from test_framework.logging import log
import pangu_lib.util as util
from pangu_lib.node_commands import restart_node
from os import path


def wipe_node_main(testnet_name: str, node_name: str, kubernetes: Kubernetes):
    """Racing condition **

    Args:
        testnet_name (str): _description_
        node_name (str): _description_
        shell (Shell): _description_
        kubernetes (Kubernetes): _description_

    Returns:
        int: _description_
    """
    #
    # Get the pod name of the node.
    node_pod_name: str = util.pod_name(node_name)

    #
    # Get the node's data paths.
    ledger_db_path = path.join(util.VELOR_DATA_DIR, "db", util.LEDGER_DB_NAME)
    state_db_path = path.join(util.VELOR_DATA_DIR, "db", util.STATE_MERKLE_DB_NAME)
    state_sync_db_path = path.join(util.VELOR_DATA_DIR, "db", util.STATE_SYNC_DB_NAME)

    #
    # Deletion commands
    deletion_command = ["rm", "-rf", ledger_db_path, state_db_path, state_sync_db_path]
    log.info(
        f'Wiping node "{node_name}" in testnet "{testnet_name}" located in the pod "{node_pod_name}"'
    )
    try:
        kubernetes.exec_command(testnet_name, node_pod_name, deletion_command)
    except Exception as e:
        log.error(
            f'Error wiping node "{node_name}" in testnet "{testnet_name}" located in the pod "{node_pod_name}"'
        )
        log.error(f"Error: {e}")
        raise e

    log.info(f"Wipe complete, restarting node {node_name}")

    restart_node.restart_node_main(testnet_name, node_name, kubernetes)

    log.info("Node restarted successfully")
