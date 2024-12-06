import argparse
from google.cloud import compute_v1
from kubernetes import client, config
import logging
import concurrent.futures
import time
import yaml
from kubernetes.client.rest import ApiException


# Constants
DISK_COPIES = 1

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


def get_region_from_zone(zone):
    return zone.rsplit("-", 1)[0]


def get_kubectl_credentials(project_id, region, cluster_name):
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


def get_snapshot_source_pv_and_zone(project_id, region, cluster_id, namespace):
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
    snapshot_name,
    source_project,
    source_cluster,
    source_region,
    source_namespace,
    target_project,
):
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
    snapshot_name,
    source_project,
    source_volume,
    source_zone,
    target_project,
):
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
        del_res = delete_operation.result()
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


def delete_disk(disk_client, project, zone, disk_name):
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


def generate_snapshot_name(run_id, snapshot_name, pvc_id):
    return f"{run_id}-{snapshot_name}-{pvc_id}"


def create_pvcs_from_snapshot_with_sdk(
    run_id, snapshot_name, namespace, pvc_number, label
):
    disk_names = [
        generate_snapshot_name(run_id, snapshot_name, pvc_id)
        for pvc_id in range(pvc_number)
    ]
    create_disks_for_replay_verify(
        PROJECT, ZONE, CLUSTER_NAME, snapshot_name, disk_names
    )
    for disk_name in disk_names:
        create_persistent_volume(
            PROJECT, ZONE, disk_name, disk_name, disk_name, namespace, True, label
        )


def cleanup_disks(run_id, snapshot_name, pvc_number):
    # delete all the disks
    disk_names = [
        generate_snapshot_name(run_id, snapshot_name, pvc_id)
        for pvc_id in range(pvc_number)
    ]
    disk_client = compute_v1.DisksClient()

    for disk_name in disk_names:
        delete_disk(disk_client, PROJECT, ZONE, disk_name)


def create_disks_for_replay_verify(
    project, zone, cluster_name, snapshot_name, disk_names
):
    disk_client = compute_v1.DisksClient()
    snapshot_client = compute_v1.SnapshotsClient()
    # create first disk from snapshot
    create_disk_from_snapshot(
        disk_client,
        snapshot_client,
        project,
        zone,
        cluster_name,
        snapshot_name,
        disk_names[0],
    )
    # clone disk from the first created disk
    source_disk_name = f"projects/{project}/zones/{zone}/disks/{disk_names[0]}"
    for i in range(1, len(disk_names)):
        clone_disk_name = disk_names[i]
        delete_disk(disk_client, project, zone, clone_disk_name)
        disk_body = compute_v1.Disk(
            name=clone_disk_name,
            source_disk=source_disk_name,
            type=f"projects/{project}/zones/{zone}/diskTypes/pd-ssd",
        )
        operation = disk_client.insert(
            project=project, zone=zone, disk_resource=disk_body
        )
        wait_for_operation(
            project, zone, operation.name, compute_v1.ZoneOperationsClient()
        )
        logger.info(f"Disk {clone_disk_name} created from source disks")


def create_disk_from_snapshot(
    disk_client, snapshot_client, project, zone, snapshot_name, disk_name
):
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
    logger.info(f"Disk {disk_name} created from snapshot {snapshot_name}.")


# Creating disk from import snapshots
# require getting a hold of the kubectrl of the cluster
# eg: gcloud container clusters get-credentials replay-on-archive --region us-central1 --project replay-verify
def create_final_snapshot(
    project,
    zone,
    cluster_name,
    og_snapshot_name,
    snapshot_name,
    disk_name,
    pv_name,
    pvc_name,
    namespace,
):
    disk_client = compute_v1.DisksClient()
    snapshot_client = compute_v1.SnapshotsClient()
    create_disk_from_snapshot(
        disk_client,
        snapshot_client,
        project,
        zone,
        cluster_name,
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


def is_job_pod_cleanedup(namespace, job_name):
    config.load_kube_config()
    v1 = client.BatchV1Api()
    try:
        job = v1.read_namespaced_job(job_name, namespace)
        return False
    except Exception as e:
        if e.status == 404:
            return True
        raise


def wait_for_operation(project, zone, operation_name, zone_operations_client):
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


def create_persistent_volume(
    project, zone, disk_name, pv_name, pvc_name, namespace, read_only, label=""
):
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
        metadata=client.V1ObjectMeta(name=pv_name, labels={"run": f"{label}"}),
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
            storage_class_name="ssd-data-xfs",
        ),
    )

    # Create PersistentVolumeClaim
    pvc = client.V1PersistentVolumeClaim(
        api_version="v1",
        kind="PersistentVolumeClaim",
        metadata=client.V1ObjectMeta(
            name=pvc_name, namespace=namespace, labels={"run": f"{label}"}
        ),
        spec=client.V1PersistentVolumeClaimSpec(
            access_modes=[access_mode],
            resources=client.V1ResourceRequirements(requests={"storage": storage_size}),
            storage_class_name="ssd-data-xfs",
            volume_name=pv_name,
        ),
    )

    v1.create_persistent_volume(body=pv)
    v1.create_namespaced_persistent_volume_claim(namespace=namespace, body=pvc)


def create_repair_disk_and_its_snapshot(
    project, zone, cluster_name, og_snapshot_name, snapshot_name, prefix, namespace
):
    tasks = []

    for copy in range(DISK_COPIES):
        disk_name = f"{prefix}-{copy}"
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


def parse_args():
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("--network", required=True, choices=["testnet", "mainnet"])
    args = parser.parse_args()
    return args


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
