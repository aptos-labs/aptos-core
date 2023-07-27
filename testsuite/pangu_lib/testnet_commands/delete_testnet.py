from test_framework.logging import log
from test_framework.kubernetes import Kubernetes


def delete_testnet_main(testnet_name: str, wait_deletion: bool, kubernetes: Kubernetes):
    """Main function of delete_testnet

    Args:
        testnet_name (str): name of the testnet to be deleted
        wait_deletion (bool): whether to wait for the deletion or not
        kubernetes (Kubernetes): kubernetes abstraction
    """
    if not testnet_name.startswith("pangu-"):
        raise Exception(f"{testnet_name} is not a valid testnet name!")

    log.info(f'The testnet "{testnet_name}" is being deleted!')

    result: bool = kubernetes.delete_namespace(testnet_name, wait_deletion)

    if not result:
        raise Exception(f'Failed to delete "{testnet_name}"!')

    log.info(f'Testnet "{testnet_name}" is deleted!')
