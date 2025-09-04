from kubernetes import client
from test_framework.filesystem import Filesystem
import yaml  # might wanna switch this library -> rueml.yaml
from typing import Any, Callable, Tuple
from enum import Enum
from datetime import timedelta
from string import Formatter
import time
from test_framework.logging import log
from dataclasses import dataclass
from test_framework.shell import Shell
from test_framework.kubernetes import Kubernetes
import socket
from contextlib import closing
import os


class NodeType(Enum):
    VALIDATOR = "validator"
    VFN = "vfn"
    PFN = "pfn"


@dataclass
class SystemContext:
    """Data class for storing the external system_context"""

    shell: Shell
    filesystem: Filesystem
    kubernetes: Kubernetes


#
# Get the path of the current script file
script_dir = os.path.dirname(os.path.abspath(__file__))

#
# Constants
WAYPOINT_TXT: str = "waypoint.txt"
GENESIS_BLOB: str = "genesis.blob"
VALIDATOR_CONFIG_YAML: str = "validator.yaml"
VFN_CONFIG_YAML: str = "vfn.yaml"
PFN_CONFIG_YAML: str = "pfn.yaml"
GENESIS_ARTIFACTS_CONFIGMAP_NAME: str = "genesis-artifiact-configmap-pangu"
PANGU_WORKSPACE_NAME: str = "pangu_artifacts"
# TEMPLATE_DIRECTORY: str = "./pangu_lib/template_testnet_files"
TEMPLATE_DIRECTORY: str = os.path.join(script_dir, "template_testnet_files")
NODE_CONFIG_MOUNT_NAME: str = "velor-config"
NODE_CONFIG_MOUNT_PATH: str = "/opt/velor/etc"
GENESIS_ARTIFACTS_MOUNT_NAME: str = "genesis-config"
GENESIS_ARTIFACTS_MOUNT_PATH: str = "/opt/velor/genesis"
NODE_IDENTITY_MOUNT_NAME: str = "velor-secret"
NODE_IDENTITY_MOUNT_PATH: str = "/opt/velor/identites"
VALIDATOR_PORT: int = 6180
FULLNODE_HOST_PORT: int = 6182
API_PORT: int = 8080
METRICS_PORT: int = 9101
BACKUP_PORT: int = 6186
# Resource requests
CPU_REQUEST: str = "4"
MEMORY_REQUEST: str = "8Gi"

PLACEHOLDER_VFN_SEED: str = (
    "00000000000000000000000000000000d58bc7bb154b38039bc9096ce04e1237"
)
PLACEHOLDER_VFN_DNS4_VALUE: str = (
    "f0274c2774519281a8332d0bb9d8101bd58bc7bb154b38039bc9096ce04e1237"
)
DEFAULT_IMAGE: str = "velorlabs/validator:devnet@sha256:f0c62463b0e86acc9ad081c54be2d1823f143f780c73828b876caebc978c8947"
VELOR_DATA_NAME: str = "velor-data"
VELOR_DATA_DIR: str = "/opt/velor/data"
LEDGER_DB_NAME: str = "ledger_db"
STATE_MERKLE_DB_NAME: str = "state_merkle_db"
STATE_SYNC_DB_NAME: str = "state_sync_db"

DEFAULT_PERSISTENT_VOLUME_CLAIM_SIZE: str = "10Gi"
DEFAULT_TRANSACTION_EMITTER_IMAGE: str = "velorlabs/tools:devnet"
TX_EMITTER_TYPE: str = "tx_emitter"


def generate_labels(
    name: str, type: NodeType, custom_suffix: str = ""
) -> dict[str, str]:
    """Generates the labels for a kubernetes object

    Args:
        username (str): the name
        type (NodeType): the type of node to generate the labels for
        custom_suffix (str, optional): custom suffix to append to the name. Defaults to "".

    Returns:
        dict[str, str]: the labels
    """
    label = {
        "app.kubernetes.io/name": type.value,  # this is used by the role
        "app.kubernetes.io/instance": f"{type_specific_name(name, type, custom_suffix)}",
        "managed-by": "pangu",
        "type": type.value,
    }
    return label


def kubernetes_object_to_yaml(dump_path: str, kube_object: Any, filesystem: Filesystem):
    """Dumps a kubernetes object as yaml

    Args:
        dump_path (str): path to dump the yaml
        kube_object (Any): the object to dump
        filesystem (Filesystem): filesystem abstraction
    """
    api_client = client.ApiClient()  # type: ignore
    stateful_set_yaml = yaml.dump(api_client.sanitize_for_serialization(kube_object))  # type: ignore
    filesystem.write(f"{dump_path}", stateful_set_yaml.encode("utf-8"))


def strfdelta(tdelta: timedelta, fmt: str = "{D:02}d {H:02}h {M:02}m {S:02}s"):
    """converts timedelta to string

    Args:
        tdelta (timedelta): timedelta to convert
        fmt (_type_, optional): format to convert to. Defaults to '{D:02}d {H:02}h {M:02}m {S:02}s'.

    Returns:
        _type_: converted timedelta
    """
    remainder = int(tdelta.total_seconds())
    f = Formatter()
    desired_fields = [field_tuple[1] for field_tuple in f.parse(fmt)]
    possible_fields = ("W", "D", "H", "M", "S")
    constants = {"W": 604800, "D": 86400, "H": 3600, "M": 60, "S": 1}
    values = {}
    for field in possible_fields:
        if field in desired_fields and field in constants:
            values[field], remainder = divmod(remainder, constants[field])
    return f.format(fmt, **values)


def type_specific_name(username: str, type: NodeType, custom_suffix: str = "") -> str:
    """Creates a type specific name

    Args:
        username (str): the base username
        type (NodeType): the type of node

    Returns:
        str: the type specific username
    """
    if custom_suffix != "":
        custom_suffix = f"-{custom_suffix}"
    return f"{username}-{type.value}{custom_suffix}"


def pod_name(node_name: str) -> str:
    """Returns the name of the pod storing the node


    Args:
        node_name (str): the name of the node

    Returns:
        str: the name of the pod
    """
    return f"{node_name}-0"


def try_function_expo_backoff(
    function: Callable[[], object], *args: Tuple[Any], max_seconds: int
):
    """Tries the function until given seconds, using exponantial backoff

    Args:
        function (_type_): function to try
        seconds (int): the seconds time
    """
    elapsed_time = 0
    attempt = 1
    while elapsed_time < max_seconds:
        try:
            result = function(*args)
            return result
        except Exception as e:
            log.error(e)
            log.info(f"Attempt {attempt} failed. Retrying in {2 ** attempt} seconds...")
            time.sleep(min(2**attempt, max_seconds - elapsed_time + 1))
            elapsed_time += 2**attempt
    raise TimeoutError(
        f"Function '{function.__name__}' timed out after {max_seconds} seconds."
    )


def is_validator_name(node_name: str) -> bool:
    """Checks if a name is a validator name

    Args:
        name (str): the name to check

    Returns:
        bool: true if it is a validator name
    """
    return node_name.endswith("-validator")


def create_temp_vfn_config(
    system_context: SystemContext,
    temp_vfn_config_path: str,
    new_vfn_config_path: str,
    node_validator_name: str,
) -> str:
    """this function updates a vfn config file with the correct dns4 value. This is a workaround for the fact that the dns4 value is not known until the node is created.

    Args:
        system_context (SystemContext): the system abstractions
        temp_vfn_config_path (str): the path to the temp vfn config file
        new_vfn_config_path (str): the path to the new vfn config file
        node_validator_name (str): the name of the validator node

    Returns:
        str: the path to the temp vfn config file
    """
    system_context.filesystem.write(filename=temp_vfn_config_path, contents=b"")
    system_context.filesystem.copyfile(new_vfn_config_path, temp_vfn_config_path)

    #
    # modify vfn.yaml
    vfn_yaml = yaml.safe_load(system_context.filesystem.read(temp_vfn_config_path))

    #
    # Make sure the necessary fields exist within the template
    # vfn_template_yaml = yaml.safe_load(
    #     system_context.filesystem.read("./pangu_lib/template_testnet_files/vfn.yaml")
    # )
    vfn_template_yaml = yaml.safe_load(
        system_context.filesystem.read(f"{TEMPLATE_DIRECTORY}/vfn.yaml")
    )

    #
    # Hardcoding the vfn network seed/key etc, which connects the vfn and its validator...
    vfn_yaml["full_node_networks"] = vfn_template_yaml["full_node_networks"]
    vfn_yaml["full_node_networks"][1]["seeds"][PLACEHOLDER_VFN_SEED]["addresses"][
        0
    ] = f"/dns4/{node_validator_name}/tcp/6181/noise-ik/{PLACEHOLDER_VFN_DNS4_VALUE}/handshake/0"
    system_context.filesystem.write(
        temp_vfn_config_path, yaml.dump(vfn_yaml).encode("utf-8")
    )
    return temp_vfn_config_path


def find_free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(("", 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]
