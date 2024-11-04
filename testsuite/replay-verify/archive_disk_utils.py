import google.auth  
from google.cloud import compute_v1  
from kubernetes import client, config  
import time  
import logging  
import concurrent.futures  
  
# Constants  
DISK_COPIES = 2  
  
# Logging configuration  
logging.basicConfig(level=logging.INFO)  
logger = logging.getLogger(__name__)  
  
# Creating snapshot from archive node disk  
def create_snapshot_from_disk(project, zone, disk_name, snapshot_name):  
    # TODO: Implement this function, this requires permission to write to the archive node  
    # Example command: gcloud compute snapshots create testnet-archive --source-disk https://www.googleapis.com/compute/v1/projects/aptos-bowu-playground/zones/us-central1-a/disks/testnet-archive --project replay-verify  
    pass  
  
# Creating disk from import snapshots  
# require getting a hold of the kubectrl of the cluster
# eg: gcloud container clusters get-credentials replay-on-archive --region us-central1 --project replay-verify
def create_disk_pv_pvc_from_snapshot(project, zone, snapshot_name, disk_name, pv_name, pvc_name, namespace):  
    disk_client = compute_v1.DisksClient()  
    snapshot_client = compute_v1.SnapshotsClient()  
  
    # Check if the disk already exists  
    try:  
        disk = disk_client.get(project=project, zone=zone, disk=disk_name)  
        logger.info(f"Disk {disk_name} already exists. Deleting it.")  
          
        # Delete the existing disk  
        operation = disk_client.delete(project=project, zone=zone, disk=disk_name)  
        wait_for_operation(project, zone, operation.name, compute_v1.ZoneOperationsClient())  
        logger.info(f"Disk {disk_name} deleted.")  
    except Exception as e:  
        logger.info(f"Disk {disk_name} does not exist. Creating a new one.")  
  
    # Create a new disk from the snapshot  
    snapshot = snapshot_client.get(project=project, snapshot=snapshot_name)  
    disk_body = compute_v1.Disk(  
        name=disk_name,  
        source_snapshot=snapshot.self_link,  
        type=f"projects/{project}/zones/{zone}/diskTypes/pd-standard"  
    )  
  
    operation = disk_client.insert(project=project, zone=zone, disk_resource=disk_body)  
    wait_for_operation(project, zone, operation.name, compute_v1.ZoneOperationsClient())  
    logger.info(f"Disk {disk_name} created from snapshot {snapshot_name}.")  
  
    create_persistent_volume(disk_name, pv_name, pvc_name, namespace)  
  
def wait_for_operation(project, zone, operation_name, zone_operations_client):  
    while True:  
        result = zone_operations_client.get(project=project, zone=zone, operation=operation_name)  
        logger.info(f"Waiting for operation {operation_name} {result}")  
          
        if result.status == compute_v1.Operation.Status.DONE:  
            if 'error' in result:  
                raise Exception(result.error)  
            return result  
          
        time.sleep(20)  
  
def create_persistent_volume(disk_name, pv_name, pvc_name, namespace):  
    v1 = client.CoreV1Api()  
  
    # Delete existing PVC if it exists  
    try:  
        existing_pvc = v1.read_namespaced_persistent_volume_claim(name=pvc_name, namespace=namespace)  
        if existing_pvc:  
            logger.info(f"PVC {pvc_name} already exists. Deleting it.")  
            v1.delete_namespaced_persistent_volume_claim(name=pvc_name, namespace=namespace)  
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
                read_only=True  
            ),  
            persistent_volume_reclaim_policy="Retain",  
            storage_class_name="standard"  
        )  
    )  
  
    # Create PersistentVolumeClaim  
    pvc = client.V1PersistentVolumeClaim(  
        api_version="v1",  
        kind="PersistentVolumeClaim",  
        metadata=client.V1ObjectMeta(name=pvc_name, namespace=namespace),  
        spec=client.V1PersistentVolumeClaimSpec(  
            access_modes=["ReadOnlyMany"],  
            resources=client.V1ResourceRequirements(  
                requests={"storage": "10000Gi"}  
            ),  
            storage_class_name="standard",  
            volume_name=pv_name  
        )  
    )  
  
    v1.create_persistent_volume(body=pv)  
    v1.create_namespaced_persistent_volume_claim(namespace=namespace, body=pvc)  
  
def main():  
    project = "replay-verify"  
    zone = "us-central1-a"  
    snapshot_name = "testnet-archive"  
    prefix = "testnet-archive"  
    namespace = "default"  
  
    tasks = []  
  
    for copy in range(DISK_COPIES):  
        disk_name = f"{prefix}-{copy}"  
        pv_name = f"{prefix}-{copy}"  
        pvc_name = f"{prefix}-claim-{copy}"  
        tasks.append((project, zone, snapshot_name, disk_name, pv_name, pvc_name, namespace))  
  
    # Execute tasks in parallel  
    with concurrent.futures.ThreadPoolExecutor() as executor:  
        futures = [executor.submit(create_disk_pv_pvc_from_snapshot, *task) for task in tasks]  
        for future in concurrent.futures.as_completed(futures):  
            try:  
                result = future.result()  
                logger.info(f"Task result: {result}")  
            except Exception as e:  
                logger.error(f"Task generated an exception: {e}")  
  
if __name__ == "__main__":  
    main()  