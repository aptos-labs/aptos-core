import click
from .restart_node import restart_node_main
from .start_node import start_node_main
from .stop_node import stop_node_main
from .wipe_node import wipe_node_main
from .profile_node import profile_node_main
from .add_pfn import add_pfn_main, AddPFNArgs
from test_framework.kubernetes import LiveKubernetes
from test_framework.filesystem import LocalFilesystem
from test_framework.shell import LocalShell
from pangu_lib.util import SystemContext
import pangu_lib.util as util


@click.command(help="Start a node by testnet and node name.")
@click.argument("testnet_name")
@click.argument("node_name")
def start(testnet_name: str, node_name: str):
    """stop a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): name of the node
    """
    start_node_main(testnet_name, node_name, LiveKubernetes())


@click.command(help="Stop a node by testnet and node name.")
@click.argument("testnet_name")
@click.argument("node_name")
def stop(testnet_name: str, node_name: str):
    """stop a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): name of the node
    """
    stop_node_main(testnet_name, node_name, LiveKubernetes())


@click.command(help="Profile a node by testnet and node name.")
@click.argument("testnet_name")
@click.argument("node_name")
def profile(testnet_name: str, node_name: str):
    """Profile a node"""
    profile_node_main(testnet_name, node_name, LiveKubernetes(), LocalShell())


@click.command(help="Restart a node by testnet and node name.")
@click.argument("testnet_name")
@click.argument("node_name")
def restart(testnet_name: str, node_name: str):
    """Restart a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): name of the node
    """
    restart_node_main(testnet_name, node_name, LiveKubernetes())


@click.command(help="Wipe a node by testnet and node name.")
@click.argument("testnet_name")
@click.argument("node_name")
def wipe(testnet_name: str, node_name: str):
    """Wipe a node

    Args:
        testnet_name (str): the namespace/testnet
        node_name (str): name of the node
    """
    wipe_node_main(testnet_name, node_name, LiveKubernetes())


@click.command(
    help="Add a PFN by providing the testnet name, PFN name, and the PFN config path."
)
@click.argument("testnet_name")
@click.argument("pfn_name")
@click.argument("pfn_config_path")
@click.option(
    "--image",
    default=util.DEFAULT_IMAGE,
    help="The image to use for the PFN. Defaults to the image defined in util.py.",
)
@click.option(
    "--workspace",
    default="",
    help="The workspace to save the deployment instructions to for a dry run. If not provided, the deployment instructions will be applied to the cluster",
)
@click.option(
    "--storage-class-name",
    default="",
    help="The storage class name to use for the PFN. Defaults to the standard storage class.",
)
@click.option(
    "--storage-size",
    default=util.DEFAULT_PERSISTENT_VOLUME_CLAIM_SIZE,
    help=f"The storage size to use for the PFN. Defaults to {util.DEFAULT_PERSISTENT_VOLUME_CLAIM_SIZE}.",
)
@click.option(
    "--cpu",
    default=util.CPU_REQUEST,
    help=f"The cpu request to use for the PFN. Defaults to {util.CPU_REQUEST}.",
)
@click.option(
    "--memory",
    default=util.MEMORY_REQUEST,
    help=f"The memory request to use for the PFN. Defaults to {util.MEMORY_REQUEST}.",
)
def add_pfn(
    testnet_name: str,
    pfn_name: str,
    pfn_config_path: str,
    image: str,
    workspace: str,
    storage_class_name: str,
    storage_size: str,
    cpu: str,
    memory: str,
):
    """Add a pfn"""
    add_pfn_main(
        AddPFNArgs(
            testnet_name,
            pfn_name,
            pfn_config_path,
            image,
            workspace,
            storage_class_name,
            storage_size,
            cpu,
            memory,
        ),
        SystemContext(LocalShell(), LocalFilesystem(), LiveKubernetes()),
    )
