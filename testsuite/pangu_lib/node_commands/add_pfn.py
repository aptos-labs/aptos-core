from pangu_lib.util import SystemContext
from test_framework.logging import log
from kubernetes import client
import pangu_lib.util as util
import os
from dataclasses import dataclass
from pangu_lib.testnet_commands.create_testnet import (
    create_persistent_volume_claim,
)


@dataclass
class AddPFNArgs:
    testnet_name: str  # testnet name
    pfn_name: str  # pfn name
    pfn_config_path: str  # pfn config path
    pfn_image: str  # pfn image
    pfn_workspace: str  # workspace
    pfn_storage_class: str  # storage class
    pfn_storage_size: str
    cpu: str
    memory: str


def add_pfn_main(
    args: AddPFNArgs,
    system_context: SystemContext,
):
    """Add a PFN to the testnet

    Args:
        args (AddPFNArgs): the arguments for the command
        system_context (SystemContext): the system context

    Raises:
        FileExistsError: if the workspace already exists

    """

    #
    # Create the kubernetes objects
    log.info(
        f'Creating the K8s objects for the PFN named "{args.pfn_name}" in the testnet "{args.testnet_name}"'
    )
    #
    # Create pvc
    persistent_volume_claim: client.V1PersistentVolumeClaim = (
        create_persistent_volume_claim(
            args.pfn_name,
            util.NodeType.PFN,
            args.pfn_storage_class,
            args.pfn_storage_size,
        )
    )
    pfn_config_config_map: client.V1ConfigMap = _create_pfn_configmap_object(
        args, system_context
    )
    pfn_statefulset: client.V1StatefulSet = _create_pfn_statefulset_object(
        args.pfn_name, args.pfn_image, args.cpu, args.memory
    )
    pfn_service: client.V1Service = _create_pfn_service_object(args.pfn_name)

    #
    # If dry run, don't apply. Just spit out the yaml.
    if args.pfn_workspace != "":
        log.info(
            f'Dry run. Saving the PFN deployment instructions to "{args.pfn_workspace}"'
        )
        #
        # Create the workspace
        try:
            system_context.filesystem.mkdir(
                os.path.join(args.pfn_workspace, "pfn_dry_run")
            )
        except FileExistsError:
            pass
        dry_run_workspace = os.path.join(
            args.pfn_workspace, "pfn_dry_run", args.pfn_name
        )
        try:
            system_context.filesystem.mkdir(dry_run_workspace)
        except FileExistsError:
            raise FileExistsError(
                f"The directory {dry_run_workspace} already exists. Please delete it and try again."
            )
        #
        # PVC
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/pfn_pvc.yaml",
            persistent_volume_claim,
            system_context.filesystem,
        )
        #
        # Configmap
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/pfn_config_config_map.yaml",
            pfn_config_config_map,
            system_context.filesystem,
        )
        #
        # Statefulset
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/pfn-statefulset.yaml",
            pfn_statefulset,
            system_context.filesystem,
        )
        #
        # Service
        util.kubernetes_object_to_yaml(
            f"{dry_run_workspace}/pfn-service.yaml",
            pfn_service,
            system_context.filesystem,
        )
        log.info(f"Done. The PFN deployment instructions are in {dry_run_workspace}")
    else:
        log.info(f'Creating the PFN for the testnet named "{args.testnet_name}"')
        #
        # PVC
        log.info(f"Applying the PVC...")
        system_context.kubernetes.create_resource(
            persistent_volume_claim, args.testnet_name
        )

        #
        # Configmap
        log.info(f"Applying the configmap...")
        system_context.kubernetes.create_resource(
            pfn_config_config_map, args.testnet_name
        )

        #
        # Statefulset
        log.info(f"Applying the statefulset...")
        system_context.kubernetes.create_resource(pfn_statefulset, args.testnet_name)

        #
        # Service
        log.info(f"Applying the service...")
        system_context.kubernetes.create_resource(pfn_service, args.testnet_name)

        log.info(
            f"Done. The PFN named {args.pfn_name}-pfn has been created in the testnet named {args.testnet_name}"
        )


def _create_pfn_configmap_object(
    args: AddPFNArgs,
    system_context: SystemContext,
) -> client.V1ConfigMap:
    """Create the configmap object for the PFN

    Args:
        args (AddPFNArgs): the arguments for the command
        system_context (SystemContext): the system context

    Returns:
        client.V1ConfigMap: the configmap object
    """
    #
    # Create the configmap object
    pfn_config_data = {
        util.PFN_CONFIG_YAML: system_context.filesystem.read(
            args.pfn_config_path
        ).decode("utf-8")
    }
    pfn_config_metadata = client.V1ObjectMeta(
        name=util.type_specific_name(args.pfn_name, util.NodeType.PFN, "configmap")
    )
    pfn_config_config_map = client.V1ConfigMap(
        api_version="v1",
        kind="ConfigMap",
        metadata=pfn_config_metadata,
        data=pfn_config_data,
    )

    return pfn_config_config_map


def _create_pfn_statefulset_object(
    pfn_name: str, pfn_image: str, pfn_cpu: str, pfn_memory: str
) -> client.V1StatefulSet:
    """Create the statefulset object for the PFN

    Args:
        pfn_name (str): the name of the PFN
        pfn_image (str): the image to use for the PFN

    Returns:
        client.V1StatefulSet: the statefulset object
    """
    #
    # Create the volumes for the statefulset
    volume1: client.V1Volume = client.V1Volume(
        name=util.NODE_CONFIG_MOUNT_NAME,
        config_map=client.V1ConfigMapVolumeSource(
            name=util.type_specific_name(pfn_name, util.NodeType.PFN, "configmap")
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
        name=util.VELOR_DATA_NAME,
        persistent_volume_claim=client.V1PersistentVolumeClaimVolumeSource(
            claim_name=util.type_specific_name(pfn_name, util.NodeType.PFN, "pvc")
        ),
    )

    volume_mount3: client.V1VolumeMount = client.V1VolumeMount(
        name=util.VELOR_DATA_NAME, mount_path=util.VELOR_DATA_DIR
    )

    #
    # Create the container/spec for the statefulset
    spec_container: client.V1Container = client.V1Container(
        name=f"{util.type_specific_name(pfn_name, util.NodeType.PFN)}",
        image=pfn_image,
        command=[
            "/usr/local/bin/velor-node",
            "-f",
            f"/opt/velor/etc/{util.NodeType.PFN.value}.yaml",
        ],
        volume_mounts=[volume_mount1, volume_mount2, volume_mount3],
        ports=[
            client.V1ContainerPort(container_port=util.FULLNODE_HOST_PORT),
            client.V1ContainerPort(container_port=6186),
            client.V1ContainerPort(container_port=8081),
            client.V1ContainerPort(container_port=util.API_PORT),
        ],
        resources=client.V1ResourceRequirements(
            requests={"cpu": pfn_cpu, "memory": pfn_memory},
        ),
    )

    spec_pod: client.V1PodSpec = client.V1PodSpec(
        containers=[spec_container],
        volumes=[volume1, volume2, volume3],
    )

    #
    # Create the StatefulSet object
    stateful_set: client.V1StatefulSet = client.V1StatefulSet(
        api_version="apps/v1",
        kind="StatefulSet",
        metadata=client.V1ObjectMeta(
            name=f"{util.type_specific_name(pfn_name, util.NodeType.PFN)}",
            labels={"type": util.NodeType.PFN.value},
        ),
        spec=client.V1StatefulSetSpec(
            replicas=1,
            selector=client.V1LabelSelector(
                match_labels=util.generate_labels(pfn_name, util.NodeType.PFN)
            ),
            service_name=f"{util.type_specific_name(pfn_name, util.NodeType.PFN)}",
            template=client.V1PodTemplateSpec(
                metadata=client.V1ObjectMeta(
                    labels=util.generate_labels(pfn_name, util.NodeType.PFN)
                ),
                spec=spec_pod,
            ),
        ),
    )

    return stateful_set


def _create_pfn_service_object(pfn_name: str) -> client.V1Service:
    """Create the service object for the PFN

    Args:
        pfn_name (str): the name of the PFN

    Returns:
        client.V1Service: the service object
    """
    service_port1: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="api",
        port=util.API_PORT,
        target_port=util.API_PORT,
    )

    service_port2: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="velornet",
        port=util.FULLNODE_HOST_PORT,
    )

    service_port3: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="backup",
        port=util.BACKUP_PORT,
    )

    service_port4: client.V1ServicePort = client.V1ServicePort(
        protocol="TCP",
        name="metrics",
        port=util.METRICS_PORT,
    )

    #
    # Create the service object
    service = client.V1Service(
        api_version="v1",
        kind="Service",
        metadata=client.V1ObjectMeta(
            name=f"{util.type_specific_name(pfn_name, util.NodeType.PFN)}",
            labels={"type": util.NodeType.PFN.value},
        ),
        spec=client.V1ServiceSpec(
            selector=util.generate_labels(pfn_name, util.NodeType.PFN),
            ports=[
                service_port1,
                service_port2,
                service_port3,
                service_port4,
            ],
        ),
    )

    return service
