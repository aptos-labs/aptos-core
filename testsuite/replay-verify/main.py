import yaml
from kubernetes import client, config
from kubernetes.client.rest import ApiException
import time
import logging
import os
from enum import Enum


SHARDING_ENABLED = False
MAX_RETRIES = 3
RETRY_DELAY = 5  # seconds
QUERY_DELAY = 10  # seconds
CONCURRENT_REPLAY = 30
REPLAY_CONCURRENCY_LEVEL = 1


class Network(Enum):
    TESETNET = 1
    MAINNET = 2


logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

PUSH_METRICS_ENDPOINT = "PUSH_METRICS_ENDPOINT"


def set_env_var(container, name, value):
    if "env" not in container:
        container["env"] = []

    # Check if the environment variable already exists
    for env_var in container["env"]:
        if env_var["name"] == name:
            env_var["value"] = value
            return

    # If it doesn't exist, add it
    container["env"].append({"name": name, "value": value})


def get_env_var(name, default_value=""):
    return os.getenv(name, default_value)


class WorkerPod:
    def __init__(
        self,
        start_version,
        end_version,
        label,
        network=Network.TESETNET,
        namespace="default",
    ):
        self.client = client.CoreV1Api()
        self.name = f"{label}-replay-verify-{start_version}-{end_version}"
        self.start_version = start_version
        self.end_version = end_version
        self.status = None
        self.log = None
        self.namespace = namespace
        self.network = network
        self.label = label

    def update_status(self):
        self.status = self.get_pod_status()

    def is_completed(self):
        self.update_status()
        if self.status and self.status.status.phase in ["Succeeded", "Failed"]:
            return True
        return False

    def is_failed(self):
        self.update_status()
        if self.status and self.status.status.phase == "Failed":
            return True
        return False
    
    def should_reschedule(self):
        if self.get_failure_reason() == "Evicted":
            return True
        return False
    
    def get_failure_reason(self):
        self.update_status()
        if self.status and self.status.status.phase == "Failed":
            return self.status.status.reason
        return None

    def get_phase(self):
        self.update_status()
        if self.status:
            return self.status.status.phase
        return None

    def has_txn_mismatch(self):
        if self.status:
            container_statuses = self.status.status.container_statuses
            if (
                container_statuses
                and container_statuses[0].state
                and container_statuses[0].state.terminated
            ):
                return container_statuses[0].state.terminated.exit_code == 2
        return False

    def get_target_db_dir(self):
        if self.network == Network.TESETNET:
            return "/mnt/testnet_archive/db"
        else:
            return "/mnt/mainnet_archive/db"

    def start(self):        
        # Load the worker YAML from the file
        with open("replay-verify-worker-template.yaml", "r") as f:
            pod_manifest = yaml.safe_load(f)

        # Create the Kubernetes API client to start a pod
        pod_manifest["metadata"]["name"] = self.name  # Unique name for each pod
        pod_manifest["metadata"]["labels"]["run"] = self.label
        pod_manifest["spec"]["containers"][0]["name"] = self.name
        pod_manifest["spec"]["containers"][0]["command"] = [
            "aptos-debugger",
            "aptos-db",
            "replay-on-archive",
            "--start-version",
            str(self.start_version),
            "--end-version",
            str(self.end_version),
            "--target-db-dir",
            self.get_target_db_dir(),
            "--concurrent-replay",
            f"{CONCURRENT_REPLAY}",
            "--replay-concurrency-level",
            f"{REPLAY_CONCURRENCY_LEVEL}",
        ]

        if SHARDING_ENABLED:
            pod_manifest["spec"]["containers"][0]["command"].append(
                "--enable-storage-sharding"
            )
        set_env_var(
            pod_manifest["spec"]["containers"][0],
            PUSH_METRICS_ENDPOINT,
            get_env_var(PUSH_METRICS_ENDPOINT, "http://localhost:9091"),
        )
        retries = 0
        while retries <= MAX_RETRIES:
            try:
                retries += 1
                response = self.client.create_namespaced_pod(
                    namespace=self.namespace, body=pod_manifest
                )
                logger.info(f"Created pod {self.name}")
                return
            except ApiException as e:
                logger.warning(
                    f"Retry {retries}/{MAX_RETRIES} for pod {self.name} failed: {e}"
                )
                time.sleep(RETRY_DELAY)
                
    def delete_pod(self):
        response = self.client.delete_namespaced_pod(  
            name=self.name,  
            namespace=self.namespace,  
            body=client.V1DeleteOptions(  
                propagation_policy='Foreground',  
                grace_period_seconds=0  
            )  
        )  

    def get_pod_exit_code(self):
        # Check the status of the pod containers
        for container_status in self.status.status.container_statuses:
            if container_status.state.terminated:
                return container_status.state.terminated.exit_code
        return None

    def get_pod_status(self):
        pod_status = self.client.read_namespaced_pod_status(
            name=self.name, namespace=self.namespace
        )
        return pod_status

    def get_humio_log_link(self):
        # TODO: Implement this ref:get_humio_link_for_node_logs
        return f"https://humio.com/search?query=namespace%3D%22{self.namespace}%22%20pod%3D%22{self.name}%22"


class ReplayScheduler:
    def __init__(
        self,
        id,
        start_version,
        end_version,
        ranges_to_skip,
        worker_cnt,
        range_size,
        network=Network.TESETNET,
    ):
        config.load_kube_config()
        self.client = client.CoreV1Api()
        self.id = id
        self.namespace = "default"
        self.start_version = start_version
        self.end_version = end_version
        self.ranges_to_skip = ranges_to_skip
        self.range_size = range_size
        self.ranges_to_skip = ranges_to_skip
        self.current_workers = [None] * worker_cnt
        self.tasks = []
        self.network = network
        self.failed_workpod_logs = []
        self.txn_mismatch_logs = []

    def get_label(self):
        return f"{self.id}-{self.network}"

    def create_tasks(self):  
        current = self.start_version  
    
        sorted_skips = [r for r in sorted(self.ranges_to_skip, key=lambda x: x[0]) if r[0] > self.start_version]  
    
        while current < self.end_version:
            print(current)
            
            while sorted_skips and sorted_skips[0][0] <= current < sorted_skips[0][1]:  
                current = sorted_skips[0][1] + 1 
                sorted_skips.pop(0)  
    
            range_end = min((current + self.range_size, self.end_version, sorted_skips[0][0] if sorted_skips else self.end_version))  
    
            if current < range_end:  
                self.tasks.append((current, range_end))  
                current = range_end 
        print(self.tasks)

    def schedule(self, from_scratch=False):
        if from_scratch:
            self.kill_all_running_pods(self.get_label())
        self.create_tasks()

        while len(self.tasks) > 0:
            for i in range(len(self.current_workers)):
                if (
                    self.current_workers[i] is None
                    or self.current_workers[i].is_completed()
                ):
                    if (
                        self.current_workers[i] is not None
                        and self.current_workers[i].is_completed()
                    ):
                        self.process_completed_pod(self.current_workers[i])
                    if len(self.tasks) == 0:
                        break
                    task = self.tasks.pop(0)
                    worker_pod = WorkerPod(
                        task[0],
                        task[1],
                        self.get_label(),
                        self.network,
                        self.namespace,
                    )
                    self.current_workers[i] = worker_pod
                    worker_pod.start()
                if self.current_workers[i] is not None:
                    print(f"Checking worker {i}: {self.current_workers[i].get_phase()}")
            time.sleep(QUERY_DELAY)
        print("All tasks have been scheduled")

    def process_completed_pod(self, worker_pod):
        if worker_pod.has_txn_mismatch():
            logger.info(
                f"Worker {worker_pod.name} failed with txn mismatch"
            )
            self.txn_mismatch_logs.append(worker_pod.get_humio_log_link())

        if worker_pod.is_failed():
            if worker_pod.should_reschedule():
                logger.info(
                    f"Worker {worker_pod.name} failed with {worker_pod.get_failure_reason()}. Rescheduling"
                )
                # clean up the existing pod
                worker_pod.delete_pod()
                # re-enter the task to the queue
                self.tasks.append((worker_pod.start_version, worker_pod.end_version))
            else:
                self.failed_workpod_logs.append(worker_pod.get_humio_log_link())

    def kill_all_running_pods(self, label):
        # Delete all pods in the namespace
        response = self.client.delete_collection_namespaced_pod(
            namespace=self.namespace,
            label_selector=f"run={label}",
        )

    def collect_all_failed_logs(self):
        logger.info("Collecting logs from remaining pods")
        all_completed = False
        while not all_completed:
            all_completed = True
            for worker in self.current_workers:
                if worker is not None:
                    if not worker.is_completed():
                        all_completed = False
                    else:
                        self.collect_logs_from_completed_pod(worker)
            time.sleep(QUERY_DELAY)
        return (self.failed_workpod_logs, self.txn_mismatch_logs)


def main():
    scheduler = ReplayScheduler(
        "test", 1000000000, 11000000000, [(1000001000, 1000002000), (3000, 4000)], worker_cnt=10, range_size=10_000_000
    )
    scheduler.schedule(from_scratch=True)
    print(scheduler.collect_all_failed_logs())
    
if __name__ == "__main__":
    main()
