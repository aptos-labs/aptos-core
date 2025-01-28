import argparse
from google.cloud import compute_v1
from kubernetes import client, config
import time
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


# Creating disk from import snapshots
# require getting a hold of the kubectrl of the cluster
# eg: gcloud container clusters get-credentials replay-on-archive --region us-central1 --project replay-verify
def create_disk_pv_pvc_from_snapshot(
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
        logger.info(f"Disk {e} {disk_name} does not exist. Creating a new one.")

    # Create a new disk from the snapshot
    logger.info(f"Creating disk {disk_name} from snapshot {og_snapshot_name}.")
    snapshot = snapshot_client.get(project=project, snapshot=og_snapshot_name)
    disk_body = compute_v1.Disk(
        name=disk_name,
        source_snapshot=snapshot.self_link,
        type=f"projects/{project}/zones/{zone}/diskTypes/pd-ssd",
    )

    operation = disk_client.insert(project=project, zone=zone, disk_resource=disk_body)
    wait_for_operation(project, zone, operation.name, compute_v1.ZoneOperationsClient())
    logger.info(f"Disk {disk_name} created from snapshot {og_snapshot_name}.")

    region_name = zone.rsplit("-", 1)[0]
    get_kubectl_credentials(project, region_name, cluster_name)
    # create_persistent_volume(disk_name, pv_name, pvc_name, namespace, True)
    # this is only for xfs replaying logs to repair the disk
    repair_pv = f"{pv_name}-repair"
    repair_pvc = f"{pvc_name}-repair"
    repair_job_name = f"xfs-repair-{pvc_name}"
    create_persistent_volume(disk_name, repair_pv, repair_pvc, namespace, False)
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


def is_job_pod_cleanedup(namespace, job_name):
    config.load_kube_config()
    v1 = client.BatchV1Api()
    try:
        job = v1.read_namespaced_job(job_name, namespace)
        return False
    except Exception as e:
        return True


def wait_for_operation(project, zone, operation_name, zone_operations_client):
    while True:
        result = zone_operations_client.get(
            project=project, zone=zone, operation=operation_name
        )
        logger.info(f"Waiting for operation {operation_name} {result}")

        if result.status == compute_v1.Operation.Status.DONE:
            if "error" in result:
                raise Exception(result.error)
            return result

        time.sleep(20)


def create_persistent_volume(disk_name, pv_name, pvc_name, namespace, read_only):
    config.load_kube_config()
    v1 = client.CoreV1Api()

    # Delete existing PVC if it exists
    try:
        existing_pvc = v1.read_namespaced_persistent_volume_claim(
            name=pvc_name, namespace=namespace
        )
        if existing_pvc:
            logger.info(f"PVC {pvc_name} already exists. Deleting it.")
            v1.delete_namespaced_persistent_volume_claim(
                name=pvc_name, namespace=namespace
            )
            logger.info(f"PVC {pvc_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            raise

    # Delete existing PV if it exists
    try:
        existing_pv = v1.read_persistent_volume(name=pv_name)
        if existing_pv:
            logger.info(f"PV {pv_name} already exists. Deleting it.")
            v1.delete_persistent_volume(name=pv_name)
            logger.info(f"PV {pv_name} deleted.")
    except client.exceptions.ApiException as e:
        if e.status != 404:
            raise

    # Create PersistentVolume
    pv = client.V1PersistentVolume(
        api_version="v1",
        kind="PersistentVolume",
        metadata=client.V1ObjectMeta(name=pv_name),
        spec=client.V1PersistentVolumeSpec(
            capacity={"storage": "10000Gi"},
            access_modes=["ReadOnlyMany"],
            gce_persistent_disk=client.V1GCEPersistentDiskVolumeSource(
                pd_name=disk_name,
                fs_type="xfs",
                read_only=read_only,
            ),
            persistent_volume_reclaim_policy="Retain",
            storage_class_name="standard",
        ),
    )

    # Create PersistentVolumeClaim
    pvc = client.V1PersistentVolumeClaim(
        api_version="v1",
        kind="PersistentVolumeClaim",
        metadata=client.V1ObjectMeta(name=pvc_name, namespace=namespace),
        spec=client.V1PersistentVolumeClaimSpec(
            access_modes=["ReadOnlyMany"],
            resources=client.V1ResourceRequirements(requests={"storage": "10000Gi"}),
            storage_class_name="standard",
            volume_name=pv_name,
        ),
    )

    v1.create_persistent_volume(body=pv)
    v1.create_namespaced_persistent_volume_claim(namespace=namespace, body=pvc)


def create_one_pvc_from_snapshot(pvc_name, snapshot_name, namespace, label):
    config.load_kube_config()
    api_instance = client.CoreV1Api()
    storage_size = "10Ti" if TESTNET_SNAPSHOT_NAME in snapshot_name else "8Ti"
    # Define the PVC manifest
    pvc_manifest = {
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {
            "name": f"{pvc_name}",
            "annotations": {
                "volume.kubernetes.io/storage-provisioner": "pd.csi.storage.gke.io"
            },
            "labels": {"run": f"{label}"},
        },
        "spec": {
            "accessModes": ["ReadOnlyMany"],
            "resources": {"requests": {"storage": storage_size}},
            "storageClassName": "ssd-data-xfs",
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


def create_pvcs_from_snapshot(run_id, snapshot_name, namespace, pvc_num, label):
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

    # Execute tasks in parallel
    tasks = [
        (f"{run_id}-{snapshot_name}-{pvc_id}", snapshot_name, namespace, label)
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


def create_disk_pv_pvc(
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
        futures = [
            executor.submit(create_disk_pv_pvc_from_snapshot, *task) for task in tasks
        ]
        for future in concurrent.futures.as_completed(futures):
            try:
                result = future.result()
                logger.info(f"Task result: {result}")
            except Exception as e:
                logger.error(f"Task generated an exception: {e}")

    # start a self deleteing job to mount the xfs disks for repairing


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
    region = "us-central1"
    project_id = "aptos-devinfra-0"
    target_namespace = "default"
    zone = "us-central1-a"
    cluster_name = "devinfra-usce1-0"

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
    create_disk_pv_pvc(
        project_id,
        zone,
        cluster_name,
        og_snapshot_name,
        snapshot_name,
        new_pv_prefix,
        target_namespace,
    )
