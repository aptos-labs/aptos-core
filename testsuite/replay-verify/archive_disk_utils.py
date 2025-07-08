import argparse
from google.cloud import compute_v1
from kubernetes import client, config
import logging
import concurrent.futures
import time
import yaml
from kubernetes.client.rest import ApiException
from tenacity import (
    retry,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception_type,
)
from typing import Tuple, List, Optional, Any


# Constants
DISK_COPIES = 1
STORAGE_CLASS = "ssd-data-xfs-immediate"

# Logging configuration
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

import subprocess

TESTNET_SNAPSHOT_NAME = "testnet-archive"
MAINNET_SNAPSHOT_NAME = "mainnet-archive"

PROJECT = "aptos-devinfra-0"
REGION = "us-central1"
CLUSTER_NAME = "devinfra-usce1-0"
NAMESPACE = "replay-verify"
ZONE = "us-central1-a"


def get_region_from_zone(zone: str) -> str:
    return zone.rsplit("-", 1)[0]


def get_kubectl_credentials(project_id: str, region: str, cluster_name: str) -> None:
    try:
        # Command to get kubectl credentials for the cluster
        command = [
            "gcloud",
            "container",
            "clusters",
            "get-credentials",
            cluster_name,
            "--region",
            region,
            "--project",
            project_id,
        ]
        subprocess.check_call(command)
        logger.info(f"Successfully fetched credentials for cluster: {cluster_name}")
    except subprocess.CalledProcessError as e:
        logger.error(f"Error fetching kubectl credentials: {e}")


def get_snapshot_source_pv_and_zone(
    project_id: str, region: str, cluster_id: str, namespace: str
) -> Tuple[str, Optional[str]]:
    get_kubectl_credentials(project_id, region, cluster_id)

    # Use the Kubernetes API
    config.load_kube_config()
    v1 = client.CoreV1Api()
    logger.info("Listing PVCs:")
    volume_names = []
    pvc_names = []
    pvcs = v1.list_namespaced_persistent_volume_claim(namespace)
    for pvc in pvcs.items:
        pvc_names.append(pvc.metadata.name)
        volume_names.append(pvc.spec.volume_name)
    logger.info(f"PVC name: {pvc_names} Volume name: {volume_names}")
    assert len(volume_names) >= 1, "No PVCs found in the namespace"
    pv_name = volume_names[0]
    pvc_name = pvc_names[0]
    pv = v1.read_persistent_volume(pv_name)
    zone = None

    zone = None
    pod_list = v1.list_namespaced_pod(namespace)
    for pod in pod_list.items:
        for volume in pod.spec.volumes:
            if (
                volume.persistent_volume_claim
                and volume.persistent_volume_claim.claim_name == pvc_name
            ):
                node_name = pod.spec.node_name
                node = v1.read_node(node_name)
                if "topology.kubernetes.io/zone" in node.metadata.labels:
                    zone = node.metadata.labels["topology.kubernetes.io/zone"]
                elif "failure-domain.beta.kubernetes.io/zone" in node.metadata.labels:
                    zone = node.metadata.labels[
                        "failure-domain.beta.kubernetes.io/zone"
                    ]
                break
        if zone:
            break

    return pv_name, zone


def create_snapshot_from_backup_pods(
    snapshot_name: str,
    source_project: str,
    source_cluster: str,
    source_region: str,
    source_namespace: str,
    target_project: str,
) -> None:
    (volume_name, zone) = get_snapshot_source_pv_and_zone(
        source_project, source_region, source_cluster, source_namespace
    )
    create_snapshot_with_gcloud(
        snapshot_name,
        source_project,
        volume_name,
        zone,
        target_project,
    )


def create_snapshot_with_gcloud(
    snapshot_name: str,
    source_project: str,
    source_volume: str,
    source_zone: str,
    target_project: str,
) -> None:
    # delete the snapshot if it already exists
    snapshot_client = compute_v1.SnapshotsClient()
    try:
        snapshot_client.get(project=target_project, snapshot=snapshot_name)
        logger.info(
            f"Snapshot {target_project} {snapshot_name} already exists. Deleting it."
        )
        delete_operation = snapshot_client.delete(
            project=target_project, snapshot=snapshot_name
        )
        del_res = delete_operation.result(timeout=1800)
        logger.info(f"Snapshot {snapshot_name} {del_res}.")
    except Exception as e:
        logger.info(
            f"Snapshot {e} {target_project} {snapshot_name} does not exist. Creating a new one."
        )

    # Construct the gcloud command to create the snapshot in the target project
    source_disk_link = f"https://www.googleapis.com/compute/v1/projects/{source_project}/zones/{source_zone}/disks/{source_volume}"
    command = [
        "gcloud",
        "compute",
        "snapshots",
        "create",
        snapshot_name,
        "--source-disk",
        source_disk_link,
        "--project",
        target_project,
        "--storage-location",
        get_region_from_zone(source_zone),
    ]

    try:
        print(
            f"Creating snapshot '{snapshot_name}' in project '{target_project}' from disk '{source_disk_link}'..."
        )
        subprocess.run(command, check=True)
        print(
            f"Snapshot '{snapshot_name}' created successfully in project '{target_project}'!"
        )
    except subprocess.CalledProcessError as e:
        print(f"Error creating snapshot: {e}")
        raise Exception(f"Error creating snapshot: {e}")


def delete_disk(
    disk_client: compute_v1.DisksClient, project: str, zone: str, disk_name: str
) -> None:
    # Check if the disk already exists

    try:
        disk = disk_client.get(project=project, zone=zone, disk=disk_name)
        logger.info(f"Disk {disk_name} already exists. Deleting it.")
        # Delete the existing disk
        operation = disk_client.delete(project=project, zone=zone, disk=disk_name)
        wait_for_operation(
            project, zone, operation.name, compute_v1.ZoneOperationsClient()
        )
        logger.info(f"Disk {disk_name} deleted.")
    except Exception as e:
        logger.info(f"Disk {e} {disk_name} does not exist, no delete needed.")


def generate_disk_name(run_id: str, snapshot_name: str, pvc_id: int) -> str:
    return f"{snapshot_name}-{run_id}-{pvc_id}"


@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10),
    retry=retry_if_exception_type(ApiException),
    before_sleep=lambda retry_state: logger.warning(
        f"Retrying initial disk creation after error: {retry_state.outcome.exception()}"
    ),
)
def create_disk_from_snapshot(
    disk_client: compute_v1.DisksClient,
    snapshot_client: compute_v1.SnapshotsClient,
    project: str,
    zone: str,
    snapshot_name: str,
    disk_name: str,
) -> None:
    start_time = time.time()
    delete_disk(disk_client, project, zone, disk_name)

    # Create a new disk from the snapshot
    logger.info(f"Creating disk {disk_name} from snapshot {snapshot_name}.")
    snapshot = snapshot_client.get(project=project, snapshot=snapshot_name)
    disk_body = compute_v1.Disk(
        name=disk_name,
        source_snapshot=snapshot.self_link,
        type=f"projects/{project}/zones/{zone}/diskTypes/pd-ssd",
    )

    operation = disk_client.insert(project=project, zone=zone, disk_resource=disk_body)
    wait_for_operation(project, zone, operation.name, compute_v1.ZoneOperationsClient())
    duration = time.time() - start_time
    logger.info(
        f"Disk {disk_name} created from snapshot {snapshot_name} with {duration}."
    )


# Creating disk from import snapshots
# require getting a hold of the kubectrl of the cluster
# eg: gcloud container clusters get-credentials replay-on-archive --region us-central1 --project replay-verify
def create_final_snapshot(
    project: str,
    zone: str,
    cluster_name: str,
    og_snapshot_name: str,
    snapshot_name: str,
    disk_name: str,
    pv_name: str,
    pvc_name: str,
    namespace: str,
) -> None:
    disk_client = compute_v1.DisksClient()
    snapshot_client = compute_v1.SnapshotsClient()
    create_disk_from_snapshot(
        disk_client,
        snapshot_client,
        project,
        zone,
        og_snapshot_name,
        disk_name,
    )
    region_name = get_region_from_zone(zone)
    get_kubectl_credentials(project, region_name, cluster_name)
    # create_persistent_volume(disk_name, pv_name, pvc_name, namespace, True)
    # this is only for xfs replaying logs to repair the disk
    repair_pv = f"{pv_name}-repair"
    repair_pvc = f"{pvc_name}-repair"
    repair_job_name = f"xfs-repair-{pvc_name}"
    create_persistent_volume(
        project, zone, disk_name, repair_pv, repair_pvc, namespace, False
    )
    # start a pod to mount the disk and run simple task
    with open("xfs-disk-repair.yaml", "r") as f:
        pod_manifest = yaml.safe_load(f)
        pod_manifest["metadata"]["name"] = repair_job_name
        pod_manifest["spec"]["template"]["spec"]["volumes"][0]["persistentVolumeClaim"][
            "claimName"
        ] = repair_pvc
    # start a job
    try:
        config.load_kube_config()
        v1 = client.BatchV1Api()
        v1.create_namespaced_job(namespace, pod_manifest)
    except Exception as e:
        logger.error(f"Error creating disk repairing job: {e}")

    # wait till the pod clean up so that disk attachement is not changed during snapshot creation
    while not is_job_pod_cleanedup(namespace, repair_job_name):
        logger.info(f"Waiting for job {repair_job_name} to finish.")
        time.sleep(10)
    logger.info(f"creating final snapshot")
    create_snapshot_with_gcloud(snapshot_name, project, disk_name, zone, project)
    logger.info("deleting repair disks")
    # delete the disk used for repair
    delete_disk(disk_client, project, zone, disk_name)
    # delete the pv and pvc
    delete_pv_and_pvc(repair_pv, repair_pvc, namespace)


def is_job_pod_cleanedup(namespace: str, job_name: str) -> bool:
    config.load_kube_config()
    v1 = client.BatchV1Api()
    try:
        job = v1.read_namespaced_job(job_name, namespace)
        # Check if job has timed out (active for too long)
        if job.status.start_time:
            job_duration = time.time() - job.status.start_time.timestamp()
            if job_duration > 240:
                logger.error(
                    f"Job {job_name} has been running for over {job_duration:.0f} seconds"
                )
                return True
        return False
    except client.exceptions.ApiException as e:
        if e.status != 404:
            logger.error(f"Error checking job {job_name}: {e}")
        return True


def wait_for_operation(
    project: str,
    zone: str,
    operation_name: str,
    zone_operations_client: compute_v1.ZoneOperationsClient,
) -> Any:
    start_time = time.time()
    timeout = 3600  # 1 hour timeout

    while True:
        if time.time() - start_time > timeout:
            raise TimeoutError(
                f"Operation {operation_name} timed out after {timeout} seconds"
            )

        result = zone_operations_client.get(
            project=project, zone=zone, operation=operation_name
        )
        logger.info(f"Waiting for operation {operation_name} {result}")

        if result.status == compute_v1.Operation.Status.DONE:
            if hasattr(result, "error") and result.error:
                raise Exception(result.error)
            return result

        time.sleep(20)


def delete_pv_and_pvc(pv_name: str, pvc_name: str, namespace: str) -> None:
    config.load_kube_config()
    v1 = client.CoreV1Api()
    try:
        v1.delete_namespaced_persistent_volume_claim(name=pvc_name, namespace=namespace)
        logger.info(f"PVC {pvc_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            logger.error(f"Error deleting PVC {pvc_name}: {e}")
    try:
        v1.delete_persistent_volume(name=pv_name)
        logger.info(f"PV {pv_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            logger.error(f"Error deleting PV {pv_name}: {e}")


def create_persistent_volume(
    project: str,
    zone: str,
    disk_name: str,
    pv_name: str,
    pvc_name: str,
    namespace: str,
    read_only: bool,
    label: str = "",
) -> None:
    config.load_kube_config()
    v1 = client.CoreV1Api()
    access_mode = "ReadWriteOnce" if not read_only else "ReadOnlyMany"
    storage_size = "10Ti" if TESTNET_SNAPSHOT_NAME in disk_name else "8Ti"

    # Delete existing PVC if it exists
    try:
        v1.read_namespaced_persistent_volume_claim(name=pvc_name, namespace=namespace)
        logger.info(f"PVC {pvc_name} already exists. Deleting it.")
        v1.delete_namespaced_persistent_volume_claim(name=pvc_name, namespace=namespace)
        logger.info(f"PVC {pvc_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            raise

    # Delete existing PV if it exists
    try:
        v1.read_persistent_volume(name=pv_name)
        logger.info(f"PV {pv_name} already exists. Deleting it.")
        v1.delete_persistent_volume(name=pv_name)
        logger.info(f"PV {pv_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            raise

    # Create PersistentVolume
    volume_handle = f"projects/{project}/zones/{zone}/disks/{disk_name}"
    pv = client.V1PersistentVolume(
        api_version="v1",
        kind="PersistentVolume",
        metadata=client.V1ObjectMeta(
            name=pv_name,
            labels={
                "run": f"{label}",
                "topology.kubernetes.io/zone": zone,  # Add zone label to PV
            },
        ),
        spec=client.V1PersistentVolumeSpec(
            capacity={"storage": storage_size},
            access_modes=[access_mode],
            csi=client.V1CSIPersistentVolumeSource(
                driver="pd.csi.storage.gke.io",
                volume_handle=volume_handle,
                fs_type="xfs",
                read_only=read_only,
            ),
            persistent_volume_reclaim_policy="Delete",
            storage_class_name=STORAGE_CLASS,
        ),
    )

    # Create PersistentVolumeClaim
    pvc = client.V1PersistentVolumeClaim(
        api_version="v1",
        kind="PersistentVolumeClaim",
        metadata=client.V1ObjectMeta(
            name=pvc_name,
            namespace=namespace,
        ),
        spec=client.V1PersistentVolumeClaimSpec(
            access_modes=[access_mode],
            resources=client.V1ResourceRequirements(requests={"storage": storage_size}),
            storage_class_name=STORAGE_CLASS,
            volume_name=pv_name,
            # Remove the selector since we're using volume_name for direct binding
        ),
    )

    v1.create_persistent_volume(body=pv)
    v1.create_namespaced_persistent_volume_claim(namespace=namespace, body=pvc)


def create_repair_disk_and_its_snapshot(
    project: str,
    zone: str,
    cluster_name: str,
    og_snapshot_name: str,
    snapshot_name: str,
    prefix: str,
    namespace: str,
) -> None:
    tasks = []

    for copy in range(DISK_COPIES):
        disk_name = f"{prefix}-repair-{copy}"
        pv_name = f"{prefix}-{copy}"
        pvc_name = f"{prefix}-claim-{copy}"
        tasks.append(
            (
                project,
                zone,
                cluster_name,
                og_snapshot_name,
                snapshot_name,
                disk_name,
                pv_name,
                pvc_name,
                namespace,
            )
        )

    # Execute tasks in parallel
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [executor.submit(create_final_snapshot, *task) for task in tasks]
        for future in concurrent.futures.as_completed(futures, timeout=3600):
            try:
                result = future.result()
                logger.info(f"Task result: {result}")
            except Exception as e:
                logger.error(f"Task generated an exception: {e}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("--network", required=True, choices=["testnet", "mainnet"])
    args = parser.parse_args()
    return args


@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10),
    retry=retry_if_exception_type((ApiException, Exception)),
    before_sleep=lambda retry_state: logger.warning(
        f"Retrying creating pvc from snapshot after error: {retry_state.outcome.exception()}"
    ),
)
def create_one_pvc_from_snapshot(
    pvc_name: str, snapshot_name: str, namespace: str, label: str, ttl_secs: int
) -> str:
    config.load_kube_config()
    api_instance = client.CoreV1Api()
    # testnet and mainnet disk size could be different
    storage_size = "12Ti" if TESTNET_SNAPSHOT_NAME in snapshot_name else "12Ti"
    # Define the PVC manifest
    pvc_manifest = {
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {
            "name": f"{pvc_name}",
            "annotations": {
                "volume.kubernetes.io/storage-provisioner": "pd.csi.storage.gke.io",
                "k8s-ttl-controller.twin.sh/ttl": f"{ttl_secs}s",
            },
            "labels": {"run": f"{label}"},
        },
        "spec": {
            "accessModes": ["ReadWriteOnce"],
            "resources": {"requests": {"storage": storage_size}},
            "storageClassName": STORAGE_CLASS,
            "volumeMode": "Filesystem",
            "dataSource": {
                "name": f"{snapshot_name}",
                "kind": "VolumeSnapshot",
                "apiGroup": "snapshot.storage.k8s.io",
            },
        },
    }

    api_instance.create_namespaced_persistent_volume_claim(
        namespace=namespace, body=pvc_manifest
    )
    return pvc_name


def create_replay_verify_pvcs_from_snapshot(
    run_id: str,
    snapshot_name: str,
    namespace: str,
    pvc_num: int,
    label: str,
    ttl_secs: int,
) -> List[str]:
    config.load_kube_config()
    api_instance = client.CustomObjectsApi()

    volume_snapshot_content = {
        "apiVersion": "snapshot.storage.k8s.io/v1",
        "kind": "VolumeSnapshotContent",
        "metadata": {"name": f"{snapshot_name}"},
        "spec": {
            "deletionPolicy": "Retain",
            "driver": "pd.csi.storage.gke.io",
            "source": {
                "snapshotHandle": f"projects/aptos-devinfra-0/global/snapshots/{snapshot_name}"
            },
            "volumeSnapshotRef": {
                "kind": "VolumeSnapshot",
                "name": f"{snapshot_name}",
                "namespace": f"{namespace}",
            },
        },
    }

    # Define the VolumeSnapshot manifest
    volume_snapshot = {
        "apiVersion": "snapshot.storage.k8s.io/v1",
        "kind": "VolumeSnapshot",
        "metadata": {"name": f"{snapshot_name}"},
        "spec": {
            "volumeSnapshotClassName": "pd-data",
            "source": {"volumeSnapshotContentName": f"{snapshot_name}"},
        },
    }

    # Create VolumeSnapshotContent
    try:
        api_instance.create_cluster_custom_object(
            group="snapshot.storage.k8s.io",
            version="v1",
            plural="volumesnapshotcontents",
            body=volume_snapshot_content,
        )

        # Create VolumeSnapshot
        api_instance.create_namespaced_custom_object(
            group="snapshot.storage.k8s.io",
            version="v1",
            namespace=namespace,
            plural="volumesnapshots",
            body=volume_snapshot,
        )
    except ApiException as e:
        if e.status != 409:
            logger.error(f"Error creating new volumesnapshots: {e}")

    tasks = [
        (
            generate_disk_name(run_id, snapshot_name, pvc_id),
            snapshot_name,
            namespace,
            label,
            ttl_secs,
        )
        for pvc_id in range(pvc_num)
    ]
    res = []
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(create_one_pvc_from_snapshot, *task) for task in tasks
        ]
        for future in concurrent.futures.as_completed(futures):
            try:
                result = future.result()
                logger.info(f"Task result: {result}")
                res.append(result)
            except Exception as e:
                logger.error(f"Task generated an exception: {e}")
    return res


def create_one_pvc_from_existing(
    pvc_name: str, existing_pvc_name: str, namespace: str, label: str, ttl_secs: int
) -> str:
    config.load_kube_config()
    api_instance = client.CoreV1Api()
    # testnet and mainnet disk size could be different
    storage_size = "12Ti" if TESTNET_SNAPSHOT_NAME in existing_pvc_name else "12Ti"
    # Define the PVC manifest
    pvc_manifest = {
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {
            "name": f"{pvc_name}",
            "annotations": {
                "volume.kubernetes.io/storage-provisioner": "pd.csi.storage.gke.io",
                "k8s-ttl-controller.twin.sh/ttl": f"{ttl_secs}s",
            },
            "labels": {"run": f"{label}"},
        },
        "spec": {
            "accessModes": ["ReadWriteOnce"],
            "resources": {"requests": {"storage": storage_size}},
            "storageClassName": STORAGE_CLASS,
            "volumeMode": "Filesystem",
            "dataSource": {
                "name": f"{existing_pvc_name}",
                "kind": "PersistentVolumeClaim",
            },
        },
    }

    api_instance.create_namespaced_persistent_volume_claim(
        namespace=namespace, body=pvc_manifest
    )
    return pvc_name


def create_replay_verify_pvcs_from_existing(
    run_id: str,
    original_snapshot_name: str,
    existing_pvc: str,
    pvc_num: int,
    namespace: str,
    label: str,
    ttl_secs: int,
) -> List[str]:
    config.load_kube_config()
    api_instance = client.CoreV1Api()

    tasks = [
        (
            generate_disk_name(run_id, f"{original_snapshot_name}-clone", pvc_id),
            existing_pvc,
            namespace,
            label,
            ttl_secs,
        )
        for pvc_id in range(pvc_num)
    ]
    res = []
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(create_one_pvc_from_existing, *task) for task in tasks
        ]
        for future in concurrent.futures.as_completed(futures):
            try:
                result = future.result()
                logger.info(f"Task result: {result}")
                res.append(result)
            except Exception as e:
                logger.error(f"Task generated an exception: {e}")
    return res


if __name__ == "__main__":
    # check input arg network
    args = parse_args()
    network = args.network
    source_project_id = "aptos-platform-compute-0"
    region = REGION
    project_id = PROJECT
    target_namespace = NAMESPACE
    zone = ZONE
    cluster_name = CLUSTER_NAME

    if network == "testnet":
        source_cluster_id = "general-usce1-0"
        source_namespace = "testnet-pfn-usce1-backup"
        snapshot_name = TESTNET_SNAPSHOT_NAME
        new_pv_prefix = TESTNET_SNAPSHOT_NAME
    else:
        source_cluster_id = "mainnet-usce1-0"
        source_namespace = "mainnet-pfn-usce1-backup"
        snapshot_name = MAINNET_SNAPSHOT_NAME
        new_pv_prefix = MAINNET_SNAPSHOT_NAME
    # create OG snapshot
    og_snapshot_name = f"{snapshot_name}-og"
    create_snapshot_from_backup_pods(
        og_snapshot_name,
        source_project_id,
        source_cluster_id,
        region,
        source_namespace,
        project_id,
    )
    create_repair_disk_and_its_snapshot(
        project_id,
        zone,
        cluster_name,
        og_snapshot_name,
        snapshot_name,
        new_pv_prefix,
        target_namespace,
    )
