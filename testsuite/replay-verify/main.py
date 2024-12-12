import yaml
from kubernetes import client, config as KubernetesConfig
from kubernetes.client.rest import ApiException
from google.cloud import storage
import time
import logging
import os
from enum import Enum
import urllib.parse
import json
import argparse
import sys

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))

from testsuite import forge
from archive_disk_utils import (
    TESTNET_SNAPSHOT_NAME,
    MAINNET_SNAPSHOT_NAME,
    create_replay_verify_pvcs_from_snapshot,
    get_kubectl_credentials,
)

SHARDING_ENABLED = False
MAX_RETRIES = 5
RETRY_DELAY = 20  # seconds
QUERY_DELAY = 5  # seconds

REPLAY_CONCURRENCY_LEVEL = 1


class Network(Enum):
    TESTNET = 1
    MAINNET = 2

    def __str__(self):
        return self.name.lower()

    @classmethod
    def from_string(cls, name: str):
        try:
            return cls[name.upper()]
        except KeyError:
            raise ValueError(f"{name} is not a valid Network name")


logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def construct_humio_url(labels_run, pod_name, start_time, end_time):
    query = f'#k8s.cluster = "devinfra-usce1-0" | k8s.labels.run = "{labels_run}" | "k8s.pod_name" = "{pod_name}"'

    params = {
        "live": "false",
        "query": query,
        "start": f"{int(start_time*1000)}",
        "end": f"{int(end_time*1000)}",
    }

    encoded_params = urllib.parse.urlencode(params, quote_via=urllib.parse.quote)
    url = f"https://cloud.us.humio.com/k8s/search?{encoded_params}"

    return url


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
        id,
        start_version,
        end_version,
        label,
        image,
        pvcs,
        replay_config,
        network=Network.TESTNET,
        namespace="default",
    ):
        self.id = id
        self.client = client.CoreV1Api()
        self.name = f"{label}-replay-verify-{start_version}-{end_version}"
        self.start_version = start_version
        self.end_version = end_version
        self.status = None
        self.log = None
        self.namespace = namespace
        self.network = network
        self.label = label
        self.start_time = time.time()
        self.image = image
        self.pvcs = pvcs
        self.config = replay_config

    def update_status(self):
        if self.status is not None and self.status.status.phase in [
            "Succeeded",
            "Failed",
        ]:
            return
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
        return "/mnt/archive/db"

    def get_claim_name(self):
        idx = self.id % len(self.pvcs)
        return self.pvcs[idx]

    def start(self):
        # Load the worker YAML from the file
        with open("replay-verify-worker-template.yaml", "r") as f:
            pod_manifest = yaml.safe_load(f)

        # Create the Kubernetes API client to start a pod
        pod_manifest["metadata"]["name"] = self.name  # Unique name for each pod
        pod_manifest["metadata"]["labels"]["run"] = self.label
        pod_manifest["spec"]["containers"][0]["image"] = self.image
        pod_manifest["spec"]["volumes"][0]["persistentVolumeClaim"][
            "claimName"
        ] = self.get_claim_name()
        pod_manifest["spec"]["containers"][0]["name"] = self.get_claim_name()
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
            f"{self.config.concurrent_replayer}",
            "--replay-concurrency-level",
            f"{REPLAY_CONCURRENCY_LEVEL}",
            "--timeout-secs",
            f"{self.config.timeout_secs}",
            "--block-cache-size",
            "10737418240",
        ]

        if SHARDING_ENABLED:
            pod_manifest["spec"]["containers"][0]["command"].append(
                "--enable-storage-sharding"
            )
        retries = 1
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
                propagation_policy="Foreground", grace_period_seconds=0
            ),
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
        return construct_humio_url(self.label, self.name, self.start_time, time.time())


class ReplayConfig:
    def __init__(self, network):
        if network == Network.TESTNET:
            self.concurrent_replayer = 20
            self.pvc_number = 5
            self.min_range_size = 10_000
            self.range_size = 5_000_000
            self.timeout_secs = 900
        else:
            self.concurrent_replayer = 18
            self.pvc_number = 8
            self.min_range_size = 10_000
            self.range_size = 2_000_000
            self.timeout_secs = 400


class TaskStats:
    def __init__(self, name):
        self.name = name
        self.start_time = time.time()
        self.end_time = None
        self.retry_count = 0
        self.durations = []

    def set_end_time(self):
        self.end_time = time.time()
        self.durations.append(self.end_time - self.start_time)

    def increment_retry_count(self):
        self.retry_count += 1

    def __str__(self) -> str:
        return f"Start time: {self.start_time}, End time: {self.end_time}, Duration: {self.durations}, Retry count: {self.retry_count}"


class ReplayScheduler:
    def __init__(
        self,
        id,
        start_version,
        end_version,
        ranges_to_skip,
        worker_cnt,
        range_size,
        image,
        replay_config,
        network=Network.TESTNET,
        namespace="default",
    ):
        KubernetesConfig.load_kube_config()
        self.client = client.CoreV1Api()
        self.id = id
        self.namespace = namespace
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
        # record
        self.task_stats = {}
        self.image = image
        self.pvcs = []
        self.config = replay_config

    def __str__(self):
        return f"""ReplayScheduler:
            id: {self.id}
            start_version: {self.start_version}
            end_version: {self.end_version}
            range_size: {self.range_size}
            worker_cnt: {worker_cnt}
            image: {image}
            number_of_pvc: {self.config.pvc_number}
            timeout_secs: {self.config.timeout_secs}
            namespace: {self.namespace}"""

    def get_label(self):
        return f"{self.id}-{self.network}"

    def create_tasks(self):
        current = self.start_version

        sorted_skips = [
            r
            for r in sorted(self.ranges_to_skip, key=lambda x: x[0])
            if r[0] > self.start_version
        ]

        while current < self.end_version:
            while sorted_skips and sorted_skips[0][0] <= current < sorted_skips[0][1]:
                current = sorted_skips[0][1] + 1
                sorted_skips.pop(0)

            range_end = min(
                (
                    current + self.range_size,
                    self.end_version,
                    sorted_skips[0][0] if sorted_skips else self.end_version,
                )
            )

            if current < range_end:
                # avoid having too many small tasks, simply skip the task
                if range_end - current >= self.config.min_range_size:
                    self.tasks.append((current, range_end))
                current = range_end
        logger.info(self.tasks)

    def create_pvc_from_snapshot(self):
        snapshot_name = (
            TESTNET_SNAPSHOT_NAME
            if self.network == Network.TESTNET
            else MAINNET_SNAPSHOT_NAME
        )
        pvcs = create_replay_verify_pvcs_from_snapshot(
            self.id,
            snapshot_name,
            self.namespace,
            self.config.pvc_number,
            self.get_label(),
        )
        assert len(pvcs) == self.config.pvc_number, "failed to create all pvcs"
        self.pvcs = pvcs

    def schedule(self, from_scratch=False):
        if from_scratch:
            self.kill_all_pods()
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
                        self.process_completed_pod(self.current_workers[i], i)
                    if len(self.tasks) == 0:
                        break
                    task = self.tasks.pop(0)
                    worker_pod = WorkerPod(
                        i,
                        task[0],
                        task[1],
                        self.get_label(),
                        self.image,
                        self.pvcs,
                        self.config,
                        self.network,
                        self.namespace,
                    )
                    self.current_workers[i] = worker_pod
                    worker_pod.start()
                    # collecting stats
                    self.task_stats[worker_pod.name] = TaskStats(worker_pod.name)

                if self.current_workers[i] is not None:
                    logger.info(
                        f"Checking worker {i}: {self.current_workers[i].name}: {self.current_workers[i].get_phase()}"
                    )
            time.sleep(QUERY_DELAY)
        logger.info("All tasks have been scheduled")

    def process_completed_pod(self, worker_pod, worker_idx):
        if worker_pod.has_txn_mismatch():
            logger.info(f"Worker {worker_pod.name} failed with txn mismatch")
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
                self.task_stats[worker_pod.name].increment_retry_count()
            else:
                self.failed_workpod_logs.append(worker_pod.get_humio_log_link())

        self.task_stats[worker_pod.name].set_end_time()
        self.current_workers[worker_idx] = None

    def cleanup(self):
        self.kill_all_pods()
        self.delete_all_pvcs()

    def kill_all_pods(self):
        # Delete all pods in the namespace
        response = self.client.delete_collection_namespaced_pod(
            namespace=self.namespace,
            label_selector=f"run={self.get_label()}",
        )

    def delete_all_pvcs(self):
        response = self.client.delete_collection_namespaced_persistent_volume_claim(
            namespace=self.namespace,
            label_selector=f"run={self.get_label()}",
        )

    def collect_all_failed_logs(self):
        logger.info("Collecting logs from remaining pods")
        all_completed = False
        while not all_completed:
            all_completed = True
            for idx, worker in enumerate(self.current_workers):
                if worker is not None:
                    logger.info(f"Checking worker {idx} {worker.name}")
                    if not worker.is_completed():
                        all_completed = False
                    else:
                        self.process_completed_pod(worker, idx)
            time.sleep(QUERY_DELAY)

        return (self.failed_workpod_logs, self.txn_mismatch_logs)

    def print_stats(self):
        for key, value in self.task_stats.items():
            logger.info(f"{key}: {value}")

    # read skip ranges from gcp bucket


def read_skip_ranges(network):
    storage_client = storage.Client()
    bucket = storage_client.bucket("replay_verify_skip_ranges")
    source_blob_name = f"{network}_skip_ranges.json"
    # Get the blob (file) from the bucket
    blob = bucket.blob(source_blob_name)

    data = json.loads(blob.download_as_text())
    skip_ranges = [
        (int(range["start_version"]), int(range["end_version"]))
        for range in data["skip_ranges"]
    ]
    return (data["start"], data["end"], skip_ranges)


def parse_args():
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("--network", required=True, choices=["testnet", "mainnet"])
    parser.add_argument("--start", required=False, type=int)
    parser.add_argument("--end", required=False, type=int)
    parser.add_argument("--worker_cnt", required=False, type=int)
    parser.add_argument("--range_size", required=False, type=int)
    parser.add_argument(
        "--namespace", required=False, type=str, default="replay-verify"
    )
    parser.add_argument("--image_tag", required=False, type=str)
    parser.add_argument("--cleanup", required=False, action="store_true", default=False)
    args = parser.parse_args()
    return args


def get_image(image_tag=None):
    shell = forge.LocalShell()
    git = forge.Git(shell)
    image_name = "tools"
    default_latest_image = (
        forge.find_recent_images(
            shell,
            git,
            1,
            image_name=image_name,
        )[0]
        if image_tag is None
        else image_tag
    )
    full_image = f"{forge.GAR_REPO_NAME}/{image_name}:{default_latest_image}"
    return full_image


def print_logs(failed_workpod_logs, txn_mismatch_logs):
    if len(failed_workpod_logs) > 0:
        logger.info("Failed workpods found")
        for log in failed_workpod_logs:
            logger.info(log)
    if len(txn_mismatch_logs) == 0:
        logger.info("No txn mismatch found")
    else:
        logger.info("Txn mismatch found")
        for log in txn_mismatch_logs:
            logger.info(log)


if __name__ == "__main__":
    args = parse_args()
    get_kubectl_credentials("aptos-devinfra-0", "us-central1", "devinfra-usce1-0")
    (start, end, skip_ranges) = read_skip_ranges(args.network)
    image = get_image(args.image_tag) if args.image_tag is not None else get_image()
    run_id = image[-5:]
    network = Network.from_string(args.network)
    config = ReplayConfig(network)
    worker_cnt = args.worker_cnt if args.worker_cnt else config.pvc_number * 7
    range_size = args.range_size if args.range_size else config.range_size

    if args.start is not None:
        assert (
            args.start >= start
        ), f"start version {args.start} is out of range {start} - {end}"
    if args.end is not None:
        assert (
            args.end <= end
        ), f"end version {args.end} is out of range {start} - {end}"

    scheduler = ReplayScheduler(
        run_id,
        start if args.start is None else args.start,
        end if args.end is None else args.end,
        skip_ranges,
        worker_cnt=worker_cnt,
        range_size=range_size,
        image=image,
        replay_config=config,
        network=network,
        namespace=args.namespace,
    )
    logger.info(f"scheduler: {scheduler}")
    cleanup = args.cleanup
    if cleanup:
        scheduler.cleanup()
        exit(0)
    else:
        scheduler.create_pvc_from_snapshot()
        try:
            scheduler.schedule(from_scratch=True)
            (failed_logs, txn_mismatch_logs) = scheduler.collect_all_failed_logs()
            scheduler.print_stats()
            print_logs(failed_logs, txn_mismatch_logs)
            if txn_mismatch_logs:
                logger.error("Transaction mismatch logs found.")
                exit(1)

        finally:
            scheduler.cleanup()
