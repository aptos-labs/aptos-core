from .create_testnet import PanguNodeLayout, SystemContext, parse_pangu_node_config
import pangu_lib.util as util
from kubernetes import client
from pangu_lib.node_commands.restart_node import restart_node_main
from test_framework.logging import log
from pangu_lib.util import create_temp_vfn_config
from concurrent import futures


def update_nodes_main(
    testnet_name: str, pangu_node_configs_path: str, system_context: SystemContext
):
    """Update nodes using the pangu_node_config

    Args:
        testnet_name (str): the testnet to update
        pangu_node_configs_path (str): path to the pangunodeconfig
        system_context (SystemContext): the system abstractions

    """

    #
    # Load the node configs
    parsed_layout: PanguNodeLayout = parse_pangu_node_config(
        system_context, pangu_node_configs_path, -1, False
    )

    log.info("Parsed pangu node config.")

    #
    # Go through every blueprint
    for pangu_node_blueprint in parsed_layout.blueprints:
        count: int = parsed_layout.blueprints[pangu_node_blueprint].count
        new_validator_config_path = parsed_layout.blueprints[
            pangu_node_blueprint
        ].validator_config_path
        new_vfn_config_path = parsed_layout.blueprints[
            pangu_node_blueprint
        ].vfn_config_path

        vfns_enabled = parsed_layout.blueprints[pangu_node_blueprint].create_vfns

        with futures.ThreadPoolExecutor() as executor:
            node_futures = []

            #
            # And every node in a blueprint
            for idx in range(1, count + 1):
                vfn_validator_node_pair_name: str = (
                    pangu_node_blueprint + f"-node-{idx}"
                )
                validator_name = vfn_validator_node_pair_name + "-validator"
                validator_image: str = parsed_layout.blueprints[
                    pangu_node_blueprint
                ].validator_image
                if validator_image == "":
                    validator_image = util.DEFAULT_IMAGE

                log.info("Trying to update node: " + validator_name)
                future = executor.submit(
                    update_node,
                    testnet_name,
                    validator_name,
                    validator_image,
                    new_validator_config_path,
                    system_context,
                )

                node_futures.append(future)  # type: ignore

                if vfns_enabled:
                    vfn_name = vfn_validator_node_pair_name + "-vfn"
                    vfn_image: str = parsed_layout.blueprints[
                        pangu_node_blueprint
                    ].vfn_image
                    if vfn_image == "":
                        vfn_image = util.DEFAULT_IMAGE

                    log.info("Updating node: " + vfn_name)
                    future = executor.submit(
                        update_node,
                        testnet_name,
                        vfn_name,
                        vfn_image,
                        new_vfn_config_path,
                        system_context,
                    )
                    node_futures.append(future)  # type: ignore

            futures.wait(node_futures)

            for future in node_futures:  # type: ignore
                if future.exception() is not None:  # type: ignore
                    raise future.exception()  # type: ignore

    log.info(
        f'Testnet "{testnet_name}" has been updated successfully using the pangu config file "{pangu_node_configs_path}"!'
    )


def update_node(
    testnet_name: str,
    node_name: str,
    new_image: str,
    new_config: str,
    system_context: SystemContext,
):
    """Update a node

    Args:
        testnet_name (str): the testnet to update
        statefulsets (List[KubernetesResource]): the statefulsets
        node_name (str): the name of the node
        new_image (str): the new image
        new_config (str): the new config
        system_context (SystemContext): the system abstractions
    """
    #
    # Check if validator or vfn
    is_validator: bool = util.is_validator_name(node_name)

    #
    # Define the patch data for the statefulset
    patch_data_statefulset = [
        {
            "op": "replace",
            "path": "/spec/template/spec/containers/0/image",
            "value": new_image,
        }
    ]

    #
    # Patch the statefulset
    system_context.kubernetes.patch_resource(
        client.V1StatefulSet, node_name, patch_data_statefulset, testnet_name
    )

    if not is_validator:
        #
        # Create a temp vfn config file, copy the vfn config provided
        temp_vfn_config_path = f"/tmp/{util.VFN_CONFIG_YAML}"
        node_validator_name: str = node_name[:-4] + "-validator"
        new_config = create_temp_vfn_config(
            system_context, temp_vfn_config_path, new_config, node_validator_name
        )

    #
    # Define the patch data for the configmap
    name_of_node_config: str = (
        util.VALIDATOR_CONFIG_YAML if is_validator else util.VFN_CONFIG_YAML
    )

    patch_data_configmap = {
        "data": {
            name_of_node_config: system_context.filesystem.read(new_config).decode(
                "utf-8"
            )
        }
    }

    #
    # Patch the configmap
    configmap_name: str = f"{node_name}-configmap"
    system_context.kubernetes.patch_resource(
        client.V1ConfigMap, configmap_name, patch_data_configmap, testnet_name
    )

    if not is_validator:
        #
        # Delete the temp vfn config file
        try:
            system_context.filesystem.unlink(new_config)
        except FileNotFoundError:
            pass

    #
    # Restart the node
    restart_node_main(testnet_name, node_name, system_context.kubernetes)
