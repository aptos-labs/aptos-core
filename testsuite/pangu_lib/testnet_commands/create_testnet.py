import yaml  # might wanna switch this library -> rueml.yaml
from dataclasses import dataclass
from typing import Dict, Union, List, Coroutine, Optional
import os
import asyncio
from dacite import from_dict
from kubernetes import client
import base64
from test_framework.logging import log
import pangu_lib.util as util
from pangu_lib.util import SystemContext, create_temp_vfn_config


@dataclass
class GenesisNodeInformation:
    """Data class for storing the information needed to generate keys and configurations for a node"""

    vfn_image: str
    validator_image: str
    validator_storage_class_name: str
    vfn_storage_class_name: str
    vfn_validator_node_pair_name: str
    user_dir: str
    validator_host: str
    fullnode_host: str
    cur_stake_amount: int
    validator_config_path: str
    vfn_config_path: str
    persistent_volume_claim_size: str
    create_vfns: bool
    cpu: str
    memory: str


@dataclass
class PanguNodeBlueprint:
    """Data class for storing a single custom node configuration"""

    validator_config_path: str
    validator_image: str
    validator_storage_class_name: str
    vfn_config_path: str
    vfn_image: str
    vfn_storage_class_name: str
    nodes_persistent_volume_claim_size: str
    create_vfns: bool
    stake_amount: int
    count: int
    cpu: str
    memory: str


@dataclass
class CreateArgs:
    """Data class for storing the arguments to the create_testnet_main function"""

    pangu_node_configs_path: Optional[str]  #  path to the pangu node configs
    num_of_validators: int  #  number of validators if the user is using the pangu template testnet
    layout_path: Optional[str]  # path to the layout
    workspace: Optional[
        str
    ]  # workspace to create the validator keys & genesis artifacts. Will use a tempfile if not provided
    framework_path: str  # path to the compiled move framework #TODO make this more customizable
    velor_cli_path: str  # path to velor cli
    dry_run: bool  # whether it is a dry run or not
    name: str  # the namespace to create the testnet


@dataclass
class PanguNodeLayout:
    """Data class for storing all the custom node configurations per testnet run"""

    blueprints: Dict[str, PanguNodeBlueprint]


def create_testnet_main(args: CreateArgs, system_context: SystemContext):
    """Creates a testnet either through the pangu template, or using the provided layout & pangu node configs

    Args:
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the system_context

    """

    #
    # CREATING A WORKSPACE / SETTING IT UP
    #

    temp_workspace_flag: bool = False
    if args.workspace is None:
        temp_workspace_flag = True

    args.workspace = create_workspace(args, system_context)

    #
    # If it's a dry run, add folder for the YAML files
    if args.dry_run:
        dry_run_workspace = os.path.join(args.workspace, "dry_run")
        system_context.filesystem.mkdir(dry_run_workspace)

    #
    # CREATE NAMESPACE IF NOT DRY RUN
    #

    if not args.dry_run:
        log.info(f': Creating Kubernetes Namespace "{args.name}"')
        namespace_object = client.V1Namespace()
        namespace_object.metadata = client.V1ObjectMeta(name=args.name)
        namespace_object.metadata.labels = {"managed-by": "pangu"}
        system_context.kubernetes.create_resource(namespace_object)

    #
    # PROCESSING THE PANGU NODE CONFIGS
    #

    #
    # If we haven't defined the node configs, use the default
    default_node_configs_flag: bool = False

    if args.pangu_node_configs_path is None:
        default_node_configs_flag = True

    args.pangu_node_configs_path = (
        args.pangu_node_configs_path
        or f"{util.TEMPLATE_DIRECTORY}/pangu_node_config.yaml"
    )

    #
    # Load 'em up
    loaded_node_configs: PanguNodeLayout = parse_pangu_node_config(
        system_context,
        args.pangu_node_configs_path,
        args.num_of_validators,
        default_node_configs_flag,
    )

    #
    # PROCESSING & CREATING THE LAYOUT
    #

    temp_layout_creation(args, system_context, loaded_node_configs)

    #
    # GENERATING GENESIS ARTIFACTS
    #

    asyncio.run(
        generate_genesis(
            args,
            system_context,
            loaded_node_configs,
        )
    )

    #
    # DELETING THE WORKSPACE
    #

    if temp_workspace_flag:
        system_context.filesystem.rmtree(args.workspace)

    if not args.dry_run:
        log.info(
            f': Created a testnet named "{args.name}" in the Kubernetes Namespace "{args.name}"!'
        )


def create_workspace(args: CreateArgs, system_context: SystemContext) -> str:
    """Creates a workspace for the testnet

    Args:
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context

    Returns:
        str: path to the workspace
    """

    #
    # If the user is using default and temp directory...
    new_workspace: Optional[str] = args.workspace
    if args.workspace is None:
        new_workspace = system_context.filesystem.mkdtemp()
    else:
        try:
            base_workspace = os.path.join(args.workspace, util.PANGU_WORKSPACE_NAME)
            system_context.filesystem.mkdir(base_workspace)
        except:
            log.info(
                f"The pangu_artifacts directory already exists in {args.workspace}"
            )

        new_workspace = os.path.join(
            args.workspace, util.PANGU_WORKSPACE_NAME, args.name
        )
        log.info(f': Creating a Workpace in "{new_workspace}"')
        system_context.filesystem.mkdir(new_workspace)

    return new_workspace


def temp_layout_creation(
    args: CreateArgs,
    system_context: SystemContext,
    loaded_node_configs: PanguNodeLayout,
) -> str:
    """Creates a temp layout file from the provided layout file. If no layout file is provided, the default layout file is used.

    Args:
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context
        loaded_node_configs (PanguNodeLayout): the parsed pangu node layout object

    Raises:
        Exception: if the workspace is not defined

    Returns:
        str: path to the temp layout file
    """

    #
    # If we haven't defined the layout, use the default
    if args.layout_path is None:
        args.layout_path = f"{util.TEMPLATE_DIRECTORY}/layout.yaml"

    log.info(f': Creating a temp layout from the directory "{args.layout_path}"')

    #
    # Create a temp layout file, copy the layout provided
    temp_layout_file = f"{args.workspace}/layout.yaml"
    system_context.filesystem.write(filename=temp_layout_file, contents=b"")

    system_context.filesystem.copyfile(args.layout_path, temp_layout_file)

    #
    # Generate the usernames for layout.
    users: List[str] = []
    for pangu_node_blueprint in loaded_node_configs.blueprints:
        for idx in range(
            1, loaded_node_configs.blueprints[pangu_node_blueprint].count + 1
        ):
            curr_username: str = pangu_node_blueprint + f"-node-{idx}"
            users.append(curr_username)

    #
    # Update the users' names in the temp yaml file
    try:
        #
        # Load the contents of the temp_layout_file.
        data = {}
        data_bytes = system_context.filesystem.read(temp_layout_file)
        data_decoded = data_bytes.decode("utf-8")
        if data_decoded.strip() != "":
            data = yaml.safe_load(data_decoded)

        #
        # Replace the "users" field with the generated usernames
        data["users"] = users

        #
        # Save the modified data back to the temp_layout_file
        system_context.filesystem.write(
            temp_layout_file, yaml.dump(data).encode("utf-8")
        )

    except yaml.YAMLError as exc:
        #
        # Error traversing/finding file!
        log.error(exc)
        raise Exception(exc)

    return temp_layout_file


def parse_pangu_node_config(
    system_context: SystemContext,
    pangu_node_configs_path: str,
    num_of_validators: int,
    default_node_configs_flag: bool,
) -> PanguNodeLayout:
    """Parses the pangu node configs

    Args:
        system_context (SystemContext): the external system_context
        pangu_node_configs_path (str): the path to the pangu node configs
        num_of_validators (int): the number of validators if the user is using the pangu template testnet
        default_node_configs_flag (bool): whether the user is using the default node configs or not

    Returns:
        PanguNodeLayout: the parsed pangu node layout object
    """
    if not pangu_node_configs_path:
        raise Exception("No pangu node config path provided!")

    log.info(f': Loading the Pangu Node Configs from "{pangu_node_configs_path}"')

    #
    # Load up the loaded_node_configs
    loaded_node_configs: PanguNodeLayout = PanguNodeLayout(blueprints=dict())

    try:
        #
        # Get the raw data
        raw_node_blueprints: Dict[str, Dict[str, Union[str, int]]] = yaml.safe_load(
            system_context.filesystem.read(pangu_node_configs_path)
        )

        #
        # Traverse every blueprint, add it to blueprints dict
        raw_node_blueprints: Dict[str, Dict[str, Union[str, int]]] = yaml.safe_load(
            system_context.filesystem.read(pangu_node_configs_path)
        )
        loaded_node_configs = from_dict(
            data_class=PanguNodeLayout, data=raw_node_blueprints
        )

    except yaml.YAMLError as exc:
        #
        # Error traversing/finding file!
        log.error(exc)
        raise Exception(exc)

    #
    # Enforce bp naming conventions
    bp_set: set[str] = set([])
    for bp_name in loaded_node_configs.blueprints:
        if not bp_name.islower() or bp_name in bp_set:
            raise Exception("All blueprint names must be distinct and lowercase")
        bp_set.add(bp_name)

    #
    # Apply default values
    for bp_name in loaded_node_configs.blueprints:
        #
        # Apply default config paths
        if loaded_node_configs.blueprints[bp_name].validator_config_path == "":
            loaded_node_configs.blueprints[
                bp_name
            ].validator_config_path = f"{util.TEMPLATE_DIRECTORY}/validator.yaml"
        if loaded_node_configs.blueprints[bp_name].vfn_config_path == "":
            loaded_node_configs.blueprints[
                bp_name
            ].vfn_config_path = f"{util.TEMPLATE_DIRECTORY}/vfn.yaml"

        #
        # Apply default image
        if loaded_node_configs.blueprints[bp_name].validator_image == "":
            loaded_node_configs.blueprints[bp_name].validator_image = util.DEFAULT_IMAGE

        if loaded_node_configs.blueprints[bp_name].vfn_image == "":
            loaded_node_configs.blueprints[bp_name].vfn_image = util.DEFAULT_IMAGE

        #
        # Some default sane resource requests
        if loaded_node_configs.blueprints[bp_name].cpu == "":
            loaded_node_configs.blueprints[bp_name].cpu = util.CPU_REQUEST
        if loaded_node_configs.blueprints[bp_name].memory == "":
            loaded_node_configs.blueprints[bp_name].memory = util.MEMORY_REQUEST

    #
    # If the user is using default settings, set the counts acordingly
    if default_node_configs_flag:
        loaded_node_configs.blueprints["nodebp"].count = num_of_validators

    return loaded_node_configs


async def generate_genesis(
    args: CreateArgs,
    system_context: SystemContext,
    loaded_node_configs: PanguNodeLayout,
):
    """Generates the keys, configurations, and statefulsets for all nodes in parallel. Then, generates the genesis artifacts.

    Args:
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context
        loaded_node_configs (PanguNodeLayout): the parsed pangu node layout object

    Raises:
        Exception: if the workspace is not defined
    """
    if not args.workspace:
        raise Exception("Workspace not defined!")

    #
    # Collect for async
    coroutines: List[Coroutine[None, None, None]] = []

    log.info(
        f': Generating "{get_layout_node_count(loaded_node_configs)}" keys and configurations in parallel for all nodes.'
    )

    if args.dry_run:
        log.info(
            f": The generated keys, configurations, and statefulsets will be saved at \"{os.path.join(args.workspace, 'dry_run')}\""
        )
    else:
        log.info(
            f": The generated keys, configurations, and statefulsets are being applied to the current kubernetes cluster."
        )

    #
    # Run through all blue prints, and create keys for each
    for pangu_node_blueprint in loaded_node_configs.blueprints:
        count: int = loaded_node_configs.blueprints[pangu_node_blueprint].count
        stake_amount: int = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].stake_amount
        validator_config_path = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].validator_config_path
        vfn_config_path = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].vfn_config_path
        create_vfns = loaded_node_configs.blueprints[pangu_node_blueprint].create_vfns
        validator_storage_class_name = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].validator_storage_class_name
        cur_stake_amount: int = stake_amount
        vfn_image: str = loaded_node_configs.blueprints[pangu_node_blueprint].vfn_image
        validator_image: str = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].validator_image

        # unpack node resources
        vfn_storage_class_name = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].vfn_storage_class_name
        persistent_volume_claim_size: str = loaded_node_configs.blueprints[
            pangu_node_blueprint
        ].nodes_persistent_volume_claim_size
        cpu: str = loaded_node_configs.blueprints[pangu_node_blueprint].cpu
        memory: str = loaded_node_configs.blueprints[pangu_node_blueprint].memory

        for idx in range(1, count + 1):
            #
            # Set all node-specific vars
            vfn_validator_node_pair_name: str = pangu_node_blueprint + f"-node-{idx}"
            user_dir: str = f"{args.workspace}/{vfn_validator_node_pair_name}"
            validator_host: str = f"{util.type_specific_name(vfn_validator_node_pair_name, util.NodeType.VALIDATOR)}:{util.VALIDATOR_PORT}"  # -validator
            fullnode_host: str = f"{util.type_specific_name(vfn_validator_node_pair_name, util.NodeType.VFN)}:{util.FULLNODE_HOST_PORT}"  # -fullnode, DNS must equal to the name of the service of the full node

            coroutines.append(
                generate_keys_and_configuration(
                    GenesisNodeInformation(
                        vfn_image,
                        validator_image,
                        validator_storage_class_name,
                        vfn_storage_class_name,
                        vfn_validator_node_pair_name,
                        user_dir,
                        validator_host,
                        fullnode_host,
                        cur_stake_amount,
                        validator_config_path,
                        vfn_config_path,
                        persistent_volume_claim_size,
                        create_vfns,
                        cpu,
                        memory,
                    ),
                    args,
                    system_context,
                )
            )

    await asyncio.gather(*coroutines)

    #
    # Move the framework to the workspace
    # TODO Might need to find a better way to do this!
    system_context.shell.run(
        ["cp", args.framework_path, args.workspace + "/framework.mrb"],
        stream_output=False,
    )

    #
    # Run genesis
    system_context.shell.run(
        [
            args.velor_cli_path,
            "genesis",
            "generate-genesis",
            "--local-repository-dir",
            args.workspace,
            "--output-dir",
            args.workspace,
        ],
        stream_output=False,
    )

    #
    # Create the genesis artifacts configmap
    genesis_artifact_data = {
        util.WAYPOINT_TXT: system_context.filesystem.read(
            f"{args.workspace}/{util.WAYPOINT_TXT}"
        ).decode("utf-8")
    }

    #
    # Since it's binary, we encode it in base64
    genesis_artifact_binary_data = {
        util.GENESIS_BLOB: base64.b64encode(
            system_context.filesystem.read(f"{args.workspace}/{util.GENESIS_BLOB}")
        ).decode("utf-8")
    }

    genesis_artifact_metadata = client.V1ObjectMeta(
        name=util.GENESIS_ARTIFACTS_CONFIGMAP_NAME
    )

    genesis_artifact_config_map = client.V1ConfigMap(
        api_version="v1",
        kind="ConfigMap",
        metadata=genesis_artifact_metadata,
        data=genesis_artifact_data,
        binary_data=genesis_artifact_binary_data,
    )

    #
    # If dry run, don't apply. Just spit out the yaml.
    if args.dry_run:
        dry_run_workspace = os.path.join(args.workspace, "dry_run")
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/genesis_artifact_config_map.yaml",
            genesis_artifact_config_map,
            system_context.filesystem,
        )
    else:
        system_context.kubernetes.create_resource(
            genesis_artifact_config_map, args.name
        )


#
# Creating the keys & configs async significantly reduces the time this process takes
# Need this helper function since we need to make sure each key is made before its validator's config
async def generate_keys_and_configuration(
    node_info: GenesisNodeInformation,
    args: CreateArgs,
    system_context: SystemContext,
):
    """Creates the validator keys and configurations

    Args:
        node_info (GenesisNodeInformation): the node information
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context

    Raises:
        Exception: if the workspace is not defined
    """
    if not args.workspace:
        raise Exception("Workspace not defined!")

    #
    # Generate validator keys
    await system_context.shell.gen_run(
        [
            args.velor_cli_path,
            "genesis",
            "generate-keys",
            "--output-dir",
            node_info.user_dir,
        ],
        stream_output=False,
    )

    #
    # Generate validator identities
    await system_context.shell.gen_run(
        [
            args.velor_cli_path,
            "genesis",
            "set-validator-configuration",
            "--owner-public-identity-file",
            node_info.user_dir + "/public-keys.yaml",
            "--local-repository-dir",
            args.workspace,
            "--username",
            node_info.vfn_validator_node_pair_name,
            "--validator-host",
            node_info.validator_host,
            "--full-node-host",
            node_info.fullnode_host,
            "--stake-amount",
            str(node_info.cur_stake_amount),
        ],
        stream_output=False,
    )

    #
    # If dry run, create folder for this node.
    if args.dry_run:
        dry_run_workspace = os.path.join(
            args.workspace, "dry_run", node_info.vfn_validator_node_pair_name
        )
        system_context.filesystem.mkdir(dry_run_workspace)

    #
    # Create validator identity secrets and validator configurations
    await create_genesis_secrets_and_configmaps(
        node_info,
        args,
        system_context,
    )

    #
    # Create a list to store the coroutines
    coroutines: List[Coroutine[None, None, None]] = []

    #
    # Create the validator stateful sets
    coroutines.append(
        create_node_stateful_sets(
            args,
            system_context,
            util.NodeType.VALIDATOR,
            node_info.vfn_validator_node_pair_name,
            node_info.validator_image,
            node_info.validator_storage_class_name,
            node_info.persistent_volume_claim_size,
            node_info.cpu,
            node_info.memory,
        )
    )

    #
    # Create the vfn stateful sets if the user wants to create vfns
    if node_info.create_vfns:
        coroutines.append(
            create_node_stateful_sets(
                args,
                system_context,
                util.NodeType.VFN,
                node_info.vfn_validator_node_pair_name,
                node_info.vfn_image,
                node_info.vfn_storage_class_name,
                node_info.persistent_volume_claim_size,
                node_info.cpu,
                node_info.memory,
            )
        )

    await asyncio.gather(*coroutines)


async def create_genesis_secrets_and_configmaps(
    node_info: GenesisNodeInformation,
    args: CreateArgs,
    system_context: SystemContext,
):
    """Creates the validator configmap, vfn configmap, and validator identity secrets

    Args:
        node_info (GenesisNodeInformation): the node information
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context

    Raises:
        Exception: if the workspace is not defined
    """
    if not args.workspace:
        raise Exception("Workspace not defined!")

    #
    # Create the validator configmap
    validator_config_data = {
        util.VALIDATOR_CONFIG_YAML: system_context.filesystem.read(
            node_info.validator_config_path
        ).decode("utf-8")
    }
    validator_config_metadata = client.V1ObjectMeta(
        name=util.type_specific_name(
            node_info.vfn_validator_node_pair_name, util.NodeType.VALIDATOR, "configmap"
        )
    )
    validator_config_config_map = client.V1ConfigMap(
        api_version="v1",
        kind="ConfigMap",
        metadata=validator_config_metadata,
        data=validator_config_data,
    )

    #
    # If dry run, don't apply. Just spit out the yaml.
    if args.dry_run:
        dry_run_workspace = os.path.join(
            args.workspace, "dry_run", node_info.vfn_validator_node_pair_name
        )
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/validator_config_config_map.yaml",
            validator_config_config_map,
            system_context.filesystem,
        )
    else:
        system_context.kubernetes.create_resource(
            validator_config_config_map, args.name
        )

    #
    # Create the vfn configmap if the user wants to create vfns
    if node_info.create_vfns:
        #
        # Create a temp vfn config file, copy the vfn config provided
        temp_vfn_config_path = f"{args.workspace}/{node_info.vfn_validator_node_pair_name}/{util.VFN_CONFIG_YAML}"
        node_validator_name: str = util.type_specific_name(
            node_info.vfn_validator_node_pair_name, util.NodeType.VALIDATOR
        )

        create_temp_vfn_config(
            system_context,
            temp_vfn_config_path,
            node_info.vfn_config_path,
            node_validator_name,
        )

        vfn_config_data = {
            util.VFN_CONFIG_YAML: system_context.filesystem.read(
                temp_vfn_config_path
            ).decode("utf-8")
        }
        vfn_config_metadata = client.V1ObjectMeta(
            name=util.type_specific_name(
                node_info.vfn_validator_node_pair_name, util.NodeType.VFN, "configmap"
            )
        )
        vfn_config_config_map = client.V1ConfigMap(
            api_version="v1",
            kind="ConfigMap",
            metadata=vfn_config_metadata,
            data=vfn_config_data,
        )

        #
        # If dry run, don't apply. Just spit out the yaml.
        if args.dry_run:
            dry_run_workspace = os.path.join(
                args.workspace, "dry_run", node_info.vfn_validator_node_pair_name
            )
            util.kubernetes_object_to_yaml(
                f"{dry_run_workspace}/vfn_config_config_map.yaml",
                vfn_config_config_map,
                system_context.filesystem,
            )
        else:
            system_context.kubernetes.create_resource(vfn_config_config_map, args.name)

    #
    # Create both vfn & validator identity secrets regardless of create_vfn flag

    identity_data = {
        "validator-identity.yaml": system_context.filesystem.read(
            f"{node_info.user_dir}/validator-identity.yaml"
        ).decode("utf-8"),
        "validator-full-node-identity.yaml": system_context.filesystem.read(
            f"{node_info.user_dir}/validator-full-node-identity.yaml"
        ).decode("utf-8"),
    }

    identity_metadata = client.V1ObjectMeta(
        name=f"identity-secrets-{node_info.vfn_validator_node_pair_name}"
    )

    identity_secrets = client.V1Secret(
        api_version="v1",
        kind="Secret",
        metadata=identity_metadata,
        string_data=identity_data,
    )

    #
    # If dry run, don't apply. Just spit out the yaml.
    if args.dry_run:
        dry_run_workspace = os.path.join(
            args.workspace, "dry_run", node_info.vfn_validator_node_pair_name
        )
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/identity_secrets.yaml",
            identity_secrets,
            system_context.filesystem,
        )
    else:
        system_context.kubernetes.create_resource(identity_secrets, args.name)


async def create_node_stateful_sets(
    args: CreateArgs,
    system_context: SystemContext,
    type: util.NodeType,
    vfn_validator_node_pair_name: str,
    image: str,
    storage_clas_name: str = "",
    persistent_volume_claim_size: str = "",
    cpu: str = "",
    memory: str = "",
):
    """Creates the stateful sets for the nodes

    Args:
        args (CreateArgs): the arguments to the function
        system_context (SystemContext): the external system_context
        type (util.NodeType): the type of node
        vfn_validator_node_pair_name (str): the name of the node
        image (str): the image to use
        storage_clas_name (str, optional): the storage class name. Defaults to "".

    Raises:
        Exception: if the workspace is not defined
    """
    if not args.workspace:
        raise Exception("Workspace not defined!")

    #
    # Create PVC for the statefulset
    persistent_volume_claim: client.V1PersistentVolumeClaim = (
        create_persistent_volume_claim(
            vfn_validator_node_pair_name,
            type,
            storage_clas_name,
            persistent_volume_claim_size,
        )
    )

    #
    # Create ports for service
    service_port1: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="validator",
        port=util.VALIDATOR_PORT,
        target_port=util.VALIDATOR_PORT,
    )

    service_port2: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="api",
        port=8080,
        target_port=8080,
    )

    service_port3: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="vfn",
        port=6181,
        target_port=6181,
    )

    service_port4: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="metrics",
        port=9101,
        target_port=9101,
    )

    #
    # Create port 6182 for vfns so pfns can dial them
    ports = []
    if type == util.NodeType.VFN:
        service_port5: client.V1ServicePort = client.V1ServicePort(
            protocol="TCP",
            name="pfn",
            port=6182,
            target_port=6182,
        )
        ports = [
            service_port1,
            service_port2,
            service_port3,
            service_port4,
            service_port5,
        ]
    else:
        ports = [service_port1, service_port2, service_port3, service_port4]

    #
    # Create the service object
    service = client.V1Service(
        api_version="v1",
        kind="Service",
        metadata=client.V1ObjectMeta(
            name=util.type_specific_name(vfn_validator_node_pair_name, type),
            labels={"type": type.value},
        ),
        spec=client.V1ServiceSpec(
            selector=util.generate_labels(vfn_validator_node_pair_name, type),
            ports=ports,
        ),
    )

    #
    # Create the volumes for the statefulset
    volume1: client.V1Volume = client.V1Volume(
        name=util.NODE_CONFIG_MOUNT_NAME,
        config_map=client.V1ConfigMapVolumeSource(
            name=util.type_specific_name(
                vfn_validator_node_pair_name, type, "configmap"
            )
        ),
    )

    volume_mount1: client.V1VolumeMount = client.V1VolumeMount(
        name=util.NODE_CONFIG_MOUNT_NAME, mount_path=util.NODE_CONFIG_MOUNT_PATH
    )

    volume2: client.V1Volume = client.V1Volume(
        name=util.GENESIS_ARTIFACTS_MOUNT_NAME,
        config_map=client.V1ConfigMapVolumeSource(
            name=util.GENESIS_ARTIFACTS_CONFIGMAP_NAME
        ),
    )

    volume_mount2: client.V1VolumeMount = client.V1VolumeMount(
        name=util.GENESIS_ARTIFACTS_MOUNT_NAME,
        mount_path=util.GENESIS_ARTIFACTS_MOUNT_PATH,
    )

    volume3: client.V1Volume = client.V1Volume(
        name=util.NODE_IDENTITY_MOUNT_NAME,
        secret=client.V1SecretVolumeSource(
            secret_name=f"identity-secrets-{vfn_validator_node_pair_name}"
        ),
    )

    volume_mount3: client.V1VolumeMount = client.V1VolumeMount(
        name=util.NODE_IDENTITY_MOUNT_NAME, mount_path=util.NODE_IDENTITY_MOUNT_PATH
    )

    volume4: client.V1Volume = client.V1Volume(
        name=util.VELOR_DATA_NAME,
        persistent_volume_claim=client.V1PersistentVolumeClaimVolumeSource(
            claim_name=util.type_specific_name(
                vfn_validator_node_pair_name, type, "pvc"
            )
        ),
    )

    volume_mount4: client.V1VolumeMount = client.V1VolumeMount(
        name=util.VELOR_DATA_NAME, mount_path=util.VELOR_DATA_DIR
    )

    #
    # Create the container/spec for the statefulset
    spec_container: client.V1Container = client.V1Container(
        name=util.type_specific_name(vfn_validator_node_pair_name, type),
        image=image,
        command=[
            "/usr/local/bin/velor-node",
            "-f",
            f"/opt/velor/etc/{type.value}.yaml",
        ],
        volume_mounts=[
            volume_mount1,
            volume_mount2,
            volume_mount3,
            volume_mount4,
        ],
        ports=[
            client.V1ContainerPort(container_port=util.VALIDATOR_PORT),
            client.V1ContainerPort(container_port=8080),
            client.V1ContainerPort(container_port=6181),
            client.V1ContainerPort(container_port=9101),
        ],
        resources=client.V1ResourceRequirements(
            requests={"cpu": cpu, "memory": memory}
        ),
    )

    spec_pod: client.V1PodSpec = client.V1PodSpec(
        containers=[spec_container],
        volumes=[volume1, volume2, volume3, volume4],
    )

    #
    # Create the StatefulSet object
    stateful_set: client.V1StatefulSet = client.V1StatefulSet(
        api_version="apps/v1",
        kind="StatefulSet",
        metadata=client.V1ObjectMeta(
            name=util.type_specific_name(vfn_validator_node_pair_name, type),
            labels={"type": type.value},
        ),
        spec=client.V1StatefulSetSpec(
            replicas=1,
            selector=client.V1LabelSelector(
                match_labels=util.generate_labels(vfn_validator_node_pair_name, type)
            ),
            service_name=util.type_specific_name(vfn_validator_node_pair_name, type),
            template=client.V1PodTemplateSpec(
                metadata=client.V1ObjectMeta(
                    labels=util.generate_labels(vfn_validator_node_pair_name, type)
                ),
                spec=spec_pod,
            ),
        ),
    )

    #
    # If dry run, don't apply. Just spit out the yaml.
    if args.dry_run:
        dry_run_workspace = os.path.join(
            args.workspace, "dry_run", vfn_validator_node_pair_name
        )
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/{util.type_specific_name(vfn_validator_node_pair_name, type, 'statefulset')}.yaml",
            stateful_set,
            system_context.filesystem,
        )
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/{util.type_specific_name(vfn_validator_node_pair_name, type, 'service')}.yaml",
            service,
            system_context.filesystem,
        )
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/{util.type_specific_name(vfn_validator_node_pair_name, type, 'pvc')}.yaml",
            persistent_volume_claim,
            system_context.filesystem,
        )
    else:
        system_context.kubernetes.create_resource(persistent_volume_claim, args.name)
        system_context.kubernetes.create_resource(stateful_set, args.name)
        system_context.kubernetes.create_resource(service, args.name)


def create_persistent_volume_claim(
    name: str,
    type: util.NodeType,
    storage_class_name: str,
    persistent_volume_claim_size: str,
) -> client.V1PersistentVolumeClaim:
    """Create a persistent volume claim for a node

    Args:
        name (str): the name
        type (util.NodeType): the type of node
        storage_class_name (str, optional): the name of the storage class. Defaults to "".

    Returns:
        client.V1PersistentVolumeClaim: _description_
    """

    #
    # Define the name of the PVC
    pvc_name: str = util.type_specific_name(name, type, "pvc")

    #
    # Define the metadata for the PVC
    metadata = client.V1ObjectMeta(
        name=pvc_name,
        labels=util.generate_labels(name, type, "pvc"),  # TODO NOT SURE IF RIGHT!
    )

    #
    # Define the size of the PVC
    if persistent_volume_claim_size == "":
        persistent_volume_claim_size = util.DEFAULT_PERSISTENT_VOLUME_CLAIM_SIZE

    #
    # Define the spec for the PVC
    if storage_class_name == "":
        spec = client.V1PersistentVolumeClaimSpec(
            access_modes=["ReadWriteOnce"],
            resources=client.V1ResourceRequirements(
                requests={"storage": persistent_volume_claim_size}
            ),
        )
    else:
        spec = client.V1PersistentVolumeClaimSpec(
            access_modes=["ReadWriteOnce"],
            storage_class_name=storage_class_name,
            resources=client.V1ResourceRequirements(
                requests={"storage": persistent_volume_claim_size}
            ),
        )

    #
    # Create the PVC object
    pvc = client.V1PersistentVolumeClaim(
        api_version="v1", kind="PersistentVolumeClaim", metadata=metadata, spec=spec
    )

    #
    # Return the PVC
    return pvc


def get_layout_node_count(loaded_node_configs: PanguNodeLayout) -> int:
    """Gets the total number of nodes in a pangu node layout

    Args:
        loaded_node_configs (PanguNodeLayout): the loaded pangu node layout

    Returns:
        int: number of nodes
    """
    total_node_count: int = 0

    for pangu_node_blueprint in loaded_node_configs.blueprints:
        total_node_count += loaded_node_configs.blueprints[pangu_node_blueprint].count

    return total_node_count
