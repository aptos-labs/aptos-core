import argparse
import datetime
import dateparser
from enum import Enum
from google.cloud import storage
import json
from kubernetes import client, config as KubernetesConfig
from kubernetes.client.rest import ApiException
import logging
import os
import sys
from tenacity import retry, stop_after_attempt, wait_fixed, retry_if_exception_type
import time
import urllib.error
import urllib.parse
import yaml


sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))

from testsuite import forge
from archive_disk_utils import (
    TESTNET_SNAPSHOT_NAME,
    MAINNET_SNAPSHOT_NAME,
    create_replay_verify_pvcs_from_existing,
    create_replay_verify_pvcs_from_snapshot,
    get_kubectl_credentials,
)

SHARDING_ENABLED = True
MAX_RETRIES = 5
RETRY_DELAY = 5  # seconds
QUERY_DELAY = 5  # seconds
POD_STATUS_CACHE_TTL = 3  # seconds — reuse cached pod status for this long

# Timeout chain — each layer is a backstop for the one before:
#   scheduler (20m) → binary (30m) → K8s TTL controller (40m)
POD_TIMEOUT = 20 * 60  # scheduler kills stuck pods after 20 min
BINARY_TIMEOUT = POD_TIMEOUT + 10 * 60  # aptos-debugger self-terminates as backstop
POD_TTL = BINARY_TIMEOUT + 10 * 60  # K8s TTL controller as last resort

REPLAY_CONCURRENCY_LEVEL = 1

INT64_MAX = 9_223_372_036_854_775_807


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


class LocalPhase(Enum):
    """Local state for a WorkerPod.

    State transitions happen only inside WorkerPod.update_status. A pod
    starts in UNKNOWN and moves through non-terminal states (PENDING,
    RUNNING) mirroring the K8s phase as seen from successful status
    fetches. The three terminal states — SUCCEEDED, FAILED, LOST — never
    transition out: once terminal, the pod stays there.

    Non-terminal states can only move forward (UNKNOWN → PENDING →
    RUNNING) or directly to any terminal state. The specific terminal
    state depends on how the pod ended: SUCCEEDED (K8s phase Succeeded),
    FAILED (K8s phase Failed, including evictions), or LOST (status fetch
    raised an exception after retries — the pod is presumed gone or the
    API is unreachable).

    State semantics:
      UNKNOWN   — no status has been fetched yet
      PENDING   — K8s phase Pending (scheduling, pulling image, attaching volumes)
      RUNNING   — K8s phase Running (container is executing)
      SUCCEEDED — K8s phase Succeeded (container exited 0) — terminal
      FAILED    — K8s phase Failed (container exited non-zero, or evicted) — terminal
      LOST      — status fetch raised after retries; pod presumed gone — terminal

    Scheduler treatment of terminal states:
      SUCCEEDED — task marked succeeded, slot freed
      FAILED    — if evicted → reschedule task (up to MAX_RETRIES),
                  else → treat as permanent failure
      LOST      — reschedule task (up to MAX_RETRIES), else → permanent failure
    """
    UNKNOWN = "Unknown"
    PENDING = "Pending"
    RUNNING = "Running"
    SUCCEEDED = "Succeeded"
    FAILED = "Failed"
    LOST = "Lost"

    def __str__(self):
        return self.value


logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def construct_humio_url(
    pod_name: str, start_time: float, end_time: float
) -> str:
    query = f'#k8s.cluster = "devinfra-usce1-0" | "k8s.pod_name" = "{pod_name}"'

    params = {
        "live": "false",
        "query": query,
        "start": f"{int(start_time*1000)}",
        "end": f"{int(end_time*1000)}",
    }

    encoded_params = urllib.parse.urlencode(params, quote_via=urllib.parse.quote)
    url = f"https://cloud.us.humio.com/k8s/search?{encoded_params}"

    return url


def set_env_var(container: dict, name: str, value: str) -> None:
    if "env" not in container:
        container["env"] = []

    # Check if the environment variable already exists
    for env_var in container["env"]:
        if env_var["name"] == name:
            env_var["value"] = value
            return

    # If it doesn't exist, add it
    container["env"].append({"name": name, "value": value})


def get_env_var(name: str, default_value: str = "") -> str:
    return os.getenv(name, default_value)


class ReplayConfig:
    def __init__(self, network: Network) -> None:
        if network == Network.TESTNET:
            self.concurrent_replayer = 35
            self.pvc_number = 35
            self.min_range_size = 10_000
            self.range_size = 5_000_000
        else:
            self.concurrent_replayer = 35
            self.pvc_number = 10
            self.min_range_size = 10_000
            self.range_size = 2_000_000


class WorkerPod:
    def __init__(
        self,
        worker_id: int,
        start_version: int,
        end_version: int,
        label: str,
        image: str,
        pvcs: list[str],
        replay_config: ReplayConfig,
        network: Network = Network.TESTNET,
        namespace: str = "default",
    ) -> None:
        self.worker_id = worker_id
        self.client = client.CoreV1Api()
        self.name = f"{label}-replay-verify-{start_version}-{end_version}"
        self.start_version = start_version
        self.end_version = end_version
        self.status = None
        self.status_fetched_at = 0.0
        self.local_phase = LocalPhase.UNKNOWN
        self.log = None
        self.namespace = namespace
        self.network = network
        self.label = label
        self.start_time = time.time()
        self.image = image
        self.pvcs = pvcs
        self.config = replay_config

    def update_status(self) -> None:
        """Refresh self.local_phase from the K8s API (with caching).

        This is the ONLY method that mutates self.local_phase and self.status.
        All other getters (is_completed, get_phase, etc.) call this then read
        the state — they never fetch or transition on their own.

        Caching rules:
          - Terminal phases (SUCCEEDED/FAILED/LOST) are cached forever.
          - Non-terminal phases are cached for POD_STATUS_CACHE_TTL seconds.
          - Any exception during fetch transitions the pod to LOST.
        """
        # Terminal phases never change — no need to refetch
        if self.local_phase in (LocalPhase.SUCCEEDED, LocalPhase.FAILED, LocalPhase.LOST):
            return
        # Non-terminal phases — reuse the cached status if it's still fresh
        if (
            self.status is not None
            and time.time() - self.status_fetched_at < POD_STATUS_CACHE_TTL
        ):
            return
        try:
            self.status = self._get_pod_status_api_call()
            self.status_fetched_at = time.time()
            phase_str = self.status.status.phase
            try:
                self.local_phase = LocalPhase(phase_str)
            except ValueError:
                self.local_phase = LocalPhase.UNKNOWN
        except Exception as e:
            # _get_pod_status_api_call already retries 5x internally; if we still get
            # an exception, consider the pod permanently LOST. Clear the
            # cached status — the last-known state is no longer trustworthy.
            logger.error(f"Pod {self.name} marked LOST after status fetch failed: {e}")
            self.local_phase = LocalPhase.LOST
            self.status = None

    def is_completed(self) -> bool:
        self.update_status()
        return self.local_phase in (
            LocalPhase.SUCCEEDED,
            LocalPhase.FAILED,
            LocalPhase.LOST,
        )

    def is_failed(self) -> bool:
        self.update_status()
        return self.local_phase in (LocalPhase.FAILED, LocalPhase.LOST)

    def should_reschedule(self) -> bool:
        self.update_status()
        if self.local_phase == LocalPhase.LOST:
            return True
        return self.get_failure_reason() == "Evicted"

    def get_failure_reason(self) -> str | None:
        self.update_status()
        if self.local_phase == LocalPhase.LOST:
            return "Lost"
        if self.local_phase == LocalPhase.FAILED and self.status:
            return self.status.status.reason
        return None

    def get_phase(self) -> LocalPhase:
        self.update_status()
        return self.local_phase

    def get_container_status(self) -> list[client.V1ContainerStatus] | None:
        self.update_status()
        if self.status:
            return self.status.status.container_statuses
        return None

    def get_container_status_summary(self) -> str:
        """Return a one-line summary of the first container's state."""
        self.update_status()
        if self.local_phase == LocalPhase.LOST:
            return "pod-lost"
        container_statuses = self.get_container_status()
        if not container_statuses:
            return "no-container-status"
        cs = container_statuses[0]
        if cs.state:
            if cs.state.waiting:
                return f"Waiting({cs.state.waiting.reason}: {cs.state.waiting.message})"
            if cs.state.running:
                return f"Running(since {cs.state.running.started_at})"
            if cs.state.terminated:
                return f"Terminated(reason={cs.state.terminated.reason}, exit={cs.state.terminated.exit_code})"
        return "unknown-state"

    def get_age_secs(self) -> float:
        """Return seconds since this WorkerPod was created."""
        return time.time() - self.start_time

    def has_txn_mismatch(self) -> bool:
        if self.local_phase == LocalPhase.LOST or not self.status:
            return False
        container_statuses = self.status.status.container_statuses
        if (
            container_statuses
            and container_statuses[0].state
            and container_statuses[0].state.terminated
        ):
            return container_statuses[0].state.terminated.exit_code == 2
        return False

    def get_target_db_dir(self) -> str:
        return "/mnt/archive/db"

    def get_claim_name(self) -> str:
        idx = self.worker_id % len(self.pvcs)
        return self.pvcs[idx]

    def start(self) -> None:
        # Load the worker YAML from the file
        with open("replay-verify-worker-template.yaml", "r") as f:
            pod_manifest = yaml.safe_load(f)

        # Create the Kubernetes API client to start a pod
        pod_manifest["metadata"]["name"] = self.name  # Unique name for each pod
        pod_manifest["metadata"]["labels"]["run"] = self.label
        pod_manifest["spec"]["containers"][0]["image"] = self.image
        pod_ttl = POD_TTL
        pod_manifest["metadata"]["annotations"][
            "k8s-ttl-controller.twin.sh/ttl"
        ] = f"{pod_ttl}s"
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
            f"{BINARY_TIMEOUT}",
            "--block-cache-size",
            f"{36 * 1024 * 1024 * 1024}",
        ]
        # TODO(ibalajiarun): bump memory limit to 180GiB for heavy ranges
        if (
            self.network == Network.TESTNET
            and self.start_version >= 6700000000
            and self.end_version < 6800000000
        ):
            pod_manifest["spec"]["containers"][0]["resources"]["requests"][
                "memory"
            ] = "180Gi"

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
                logger.info(f"Created pod {self.name} (worker={self.worker_id}, pvc={self.get_claim_name()})")
                return
            except ApiException as e:
                logger.warning(
                    f"Retry {retries}/{MAX_RETRIES} for pod {self.name} failed: {e}"
                )
                time.sleep(RETRY_DELAY)

    def delete_pod(self) -> None:
        """Delete the pod. Best-effort — never throws.

        Transitions to LOST only if not already terminal (preserves
        SUCCEEDED/FAILED in case this is called after the pod finished).
        """
        if self.local_phase not in (
            LocalPhase.SUCCEEDED,
            LocalPhase.FAILED,
            LocalPhase.LOST,
        ):
            self.local_phase = LocalPhase.LOST
            self.status = None
        try:
            self._delete_pod_api_call()
        except Exception as e:
            logger.warning(f"Best-effort delete of pod {self.name} failed: {e}")

    @retry(
        stop=stop_after_attempt(MAX_RETRIES),
        wait=wait_fixed(RETRY_DELAY),
        retry=retry_if_exception_type(ApiException),
        before_sleep=lambda retry_state: logger.warning(
            f"Retry {retry_state.attempt_number}/{MAX_RETRIES} failed: {retry_state.outcome.exception()}"
        ),
    )
    def _delete_pod_api_call(self):
        try:
            return self.client.delete_namespaced_pod(
                name=self.name,
                namespace=self.namespace,
                body=client.V1DeleteOptions(
                    propagation_policy="Foreground", grace_period_seconds=0
                ),
            )
        except ApiException as e:
            if e.status == 404:  # Pod not found
                logger.info(f"Pod {self.name} already deleted or doesn't exist")
                return None  # Consider this a success
            raise  # Re-raise other API exceptions for retry

    @retry(
        stop=stop_after_attempt(MAX_RETRIES),
        wait=wait_fixed(RETRY_DELAY),
        retry=retry_if_exception_type(ApiException),
        before_sleep=lambda retry_state: logger.warning(
            f"Retry {retry_state.attempt_number}/{MAX_RETRIES} failed: {retry_state.outcome.exception()}"
        ),
    )
    def _get_pod_status_api_call(self):
        pod_status = self.client.read_namespaced_pod_status(
            name=self.name, namespace=self.namespace
        )
        return pod_status

    def get_humio_log_link(self):
        return construct_humio_url(self.name, self.start_time, time.time())


class TaskStats:
    def __init__(self, name: str) -> None:
        self.name: str = name
        self.start_time: float = time.time()
        self.end_time: float | None = None
        self.retry_count: int = 0
        self.succeeded: bool = False

    def set_end_time(self) -> None:
        # Unconditional update — on retries, the final call wins so the
        # recorded end_time reflects when the task actually finished.
        self.end_time = time.time()

    def increment_retry_count(self) -> None:
        self.retry_count += 1

    def reset_timing(self) -> None:
        """Reset start_time to now; clear end_time.

        Called when a retried task is re-dispatched, so the reported
        duration reflects only the final attempt (not queue wait time
        or earlier failed attempts).
        """
        self.start_time = time.time()
        self.end_time = None

    def set_succeeded(self):
        self.succeeded = True

    @property
    def duration_secs(self) -> float | None:
        if self.end_time is None:
            return None
        return self.end_time - self.start_time

    def _fmt_time(self, t: float | None) -> str:
        if t is None:
            return "?"
        return datetime.datetime.fromtimestamp(t, datetime.timezone.utc).strftime(
            "%Y-%m-%d %H:%M:%S UTC"
        )

    def __str__(self) -> str:
        duration = self.duration_secs
        duration_str = f"{duration:.1f}s" if duration is not None else "?"
        return (
            f"Succeeded: {self.succeeded}, "
            f"Start: {self._fmt_time(self.start_time)}, "
            f"End: {self._fmt_time(self.end_time)}, "
            f"Duration: {duration_str}, "
            f"Retry count: {self.retry_count}"
        )


class ReplayScheduler:
    def __init__(
        self,
        id: str,
        start_version: int,
        end_version: int,
        ranges_to_skip: list[tuple[int, int]],
        worker_cnt: int,
        range_size: int,
        image: str,
        replay_config: ReplayConfig,
        network: Network = Network.TESTNET,
        namespace: str = "default",
    ) -> None:
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
            pod_timeout: {POD_TIMEOUT}
            binary_timeout: {BINARY_TIMEOUT}
            pod_ttl: {POD_TTL}
            namespace: {self.namespace}"""

    def get_label(self):
        return f"{self.id}-{self.network}"

    def humio_hash_mismatch_url(self, start_time: float, end_time: float) -> str:
        query = f'"k8s.pod_name" = "{self.get_label()}-replay-verify-*" | "TransactionOutput does not match"'

        params = {
            "live": "false",
            "query": query,
            "start": f"{int(start_time*1000)}",
            "end": f"{int(end_time*1000)}",
        }

        encoded_params = urllib.parse.urlencode(params, quote_via=urllib.parse.quote)
        url = f"https://cloud.us.humio.com/k8s/search?{encoded_params}"

        return url

    def sorted_ranges_to_skip(self):
        if len(self.ranges_to_skip) == 0:
            return []

        sorted_skips = [
            list(r) for r in sorted(self.ranges_to_skip) if r[1] >= self.start_version
        ]

        if len(sorted_skips) == 0:
            return []

        # merge skip ranges
        ret = []
        current_skip = sorted_skips.pop(0)
        for next_skip in sorted_skips:
            if next_skip[0] > current_skip[1] + 1:
                ret.append(current_skip)
                current_skip = next_skip
            else:
                current_skip[1] = max(current_skip[1], next_skip[1])
        ret.append(current_skip)

        return ret

    def create_tasks(self) -> None:
        current = self.start_version

        skips = self.sorted_ranges_to_skip()

        range_size = self.range_size
        heavy_range_size = int(range_size / 5)

        while current <= self.end_version:
            (skip_start, skip_end) = (
                (INT64_MAX, INT64_MAX) if len(skips) == 0 else skips[0]
            )

            # TODO(ibalajiarun): temporary hack to handle heavy ranges
            if (
                self.network == Network.TESTNET
                and current >= 6700000000
                and current < 6800000000
            ):
                next_current = min(
                    current + heavy_range_size, self.end_version + 1, skip_start
                )
            else:
                next_current = min(
                    current + range_size, self.end_version + 1, skip_start
                )

            # Only skip if current is within the skip range
            if skip_start <= current <= skip_end:
                skips.pop(0)
                current = skip_end + 1
                continue
            elif skip_start <= next_current - 1 <= skip_end:
                # If the next current is within the skip range, we need to adjust it
                next_current = skip_start
            elif next_current > skip_start:
                # If the next current is beyond the skip range, we need to adjust it
                next_current = skip_start

            # avoid having too many small tasks, simply skip the task
            range = (current, next_current - 1)
            if next_current - current >= self.config.min_range_size:
                self.tasks.append(range)
            else:
                logger.info(f"Skipping small range {range}")

            current = next_current

        logger.info(f"Task ranges: {self.tasks}")

    def create_pvc_from_snapshot(self):
        snapshot_name = (
            TESTNET_SNAPSHOT_NAME
            if self.network == Network.TESTNET
            else MAINNET_SNAPSHOT_NAME
        )
        # Because PVCs can be shared among multiple replay-verify runs, a more correct TTL
        # would be computed from the number of shards and the expected run time of the replay-verify
        # run. However, for simplicity, we set the TTL to 9 hours.
        pvc_ttl = 9 * 60 * 60  # 9 hours
        pvcs = create_replay_verify_pvcs_from_snapshot(
            self.id,
            snapshot_name,
            self.namespace,
            self.config.pvc_number,
            self.get_label(),
            pvc_ttl,
        )
        assert len(pvcs) == self.config.pvc_number, "failed to create all pvcs"
        self.pvcs = pvcs

    def create_all_required_pvcs(self) -> None:
        snapshot_name = (
            TESTNET_SNAPSHOT_NAME
            if self.network == Network.TESTNET
            else MAINNET_SNAPSHOT_NAME
        )
        pvc = self.create_one_pvc_from_snapshot(snapshot_name)
        self.pvcs = [pvc]
        # Wait for the PVC to be bound before creating other PVCs
        logger.info(
            f"Waiting for the PVC {pvc} to be bound. "
            "This involves cloning a large disk volume from a snapshot and may take 10+ minutes..."
        )
        bound_status = self.get_pvc_bound_status()
        while not self.get_pvc_bound_status()[0]:
            time.sleep(QUERY_DELAY)
        logger.info(f"PVC {pvc} is now bound. Proceeding to clone additional PVCs...")
        self.create_pvc_from_existing(snapshot_name, pvc)

    def create_one_pvc_from_snapshot(self, snapshot_name: str) -> str:
        # Because PVCs can be shared among multiple replay-verify runs, a more correct TTL
        # would be computed from the number of shards and the expected run time of the replay-verify
        # run. However, for simplicity, we set the TTL to 3 hours.
        pvc_ttl = 8 * 60 * 60  # 8 hours
        pvcs = create_replay_verify_pvcs_from_snapshot(
            self.id,
            snapshot_name,
            self.namespace,
            1,  # only create one PVC
            self.get_label(),
            pvc_ttl,
        )
        assert len(pvcs) == 1, "failed to create the PVC"
        return pvcs[0]

    # Creates a pvc by cloning an existing pvc
    def create_pvc_from_existing(self, original_snapshot_name: str, existing_pvc: str):
        pvc_ttl = 8 * 60 * 60
        pvcs = create_replay_verify_pvcs_from_existing(
            self.id,
            original_snapshot_name,
            existing_pvc,
            self.config.pvc_number,
            self.namespace,
            self.get_label(),
            pvc_ttl,
        )
        assert len(pvcs) == self.config.pvc_number, "failed to create all pvcs"
        self.pvcs = pvcs

    @retry(
        stop=stop_after_attempt(MAX_RETRIES),
        wait=wait_fixed(RETRY_DELAY),
        retry=retry_if_exception_type(ApiException),
        before_sleep=lambda retry_state: logger.warning(
            f"Retry {retry_state.attempt_number}/{MAX_RETRIES} failed: {retry_state.outcome.exception()}"
        ),
    )
    def get_pvc_bound_status(self) -> list[bool]:
        statuses = []
        for pvc in self.pvcs:
            pvc_status = self.client.read_namespaced_persistent_volume_claim_status(
                name=pvc, namespace=self.namespace
            )
            if pvc_status.status.phase == "Bound":
                statuses.append(True)
            else:
                statuses.append(False)
        return statuses

    def _has_active_workers(self) -> bool:
        return any(w is not None for w in self.current_workers)

    def schedule(self, from_scratch: bool = False) -> tuple[list[str], list[str]]:
        """Dispatch all tasks to worker pods and wait for them to complete.

        The loop scans worker slots each iteration:
        - Completed or timed-out pods are processed and their slots freed.
        - Free slots get the next task from the queue (if any).
        - The loop exits when no tasks remain and all slots are empty.

        Returns (failed_workpod_logs, txn_mismatch_logs).
        """
        if from_scratch:
            self.kill_all_pods()
        self.create_tasks()

        schedule_start = time.time()
        last_summary_time = schedule_start
        self.total_tasks = len(self.tasks)
        pod_timeout = POD_TIMEOUT

        # Keep running while there are tasks to dispatch OR workers still active.
        while len(self.tasks) > 0 or self._has_active_workers():

            # --- Scan worker slots ---
            pvc_bound_status = self.get_pvc_bound_status()
            for i in range(len(self.current_workers)):
                worker = self.current_workers[i]

                if worker is not None:
                    if worker.is_completed():
                        # Pod finished (Succeeded or Failed) — process and free slot
                        self.process_completed_pod(worker, i)
                        worker = self.current_workers[i]
                    elif worker.get_age_secs() > pod_timeout:
                        # Pod has been running too long — reschedule if under
                        # retry limit, otherwise treat as permanent failure.
                        retries = self.task_stats[worker.name].retry_count + 1
                        logger.error(
                            f"Worker {i} timed out: {worker.name}, "
                            f"phase={worker.get_phase()}, "
                            f"container={worker.get_container_status_summary()}, "
                            f"age={int(worker.get_age_secs())}s, "
                            f"attempt {retries}/{MAX_RETRIES}"
                        )
                        if retries < MAX_RETRIES:
                            self.kill_pod_and_reschedule_task(worker, i)
                        else:
                            logger.error(
                                f"Worker {i} exceeded max retries, giving up: {worker.name}"
                            )
                            worker.delete_pod()
                            self.failed_workpod_logs.append(worker.get_humio_log_link())
                            self.current_workers[i] = None
                        # Sync local var with the slot (now None in both branches:
                        # kill_pod_and_reschedule_task clears it, and the else branch above clears
                        # it explicitly). This allows the dispatch check below to
                        # immediately fill this slot with a new task.
                        worker = None

                # If slot is free and there are tasks, dispatch one.
                # PVC must be bound first, unless this is the initial pod
                # that triggers binding (i < number of PVCs).
                pvc_ready = pvc_bound_status[i % len(self.pvcs)] or i < len(self.pvcs)
                if worker is None and pvc_ready and len(self.tasks) > 0:
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
                    if worker_pod.name not in self.task_stats:
                        self.task_stats[worker_pod.name] = TaskStats(worker_pod.name)
                    else:
                        # Retry dispatch: reset timing so duration reflects
                        # only this attempt, not earlier ones or queue wait.
                        self.task_stats[worker_pod.name].reset_timing()

            # --- Periodic status summary ---
            # Every 5 min while dispatching, every 60s while waiting.
            now = time.time()
            summary_interval = 60 if len(self.tasks) == 0 else 300
            if now - last_summary_time >= summary_interval:
                self._log_worker_summary(schedule_start, tasks_remaining=len(self.tasks))
                last_summary_time = now

            time.sleep(QUERY_DELAY)

        logger.info("All tasks completed")
        return (self.failed_workpod_logs, self.txn_mismatch_logs)

    def kill_pod_and_reschedule_task(self, worker_pod: WorkerPod, worker_idx: int):
        # clean up the existing pod
        worker_pod.delete_pod()
        # re-enter the task to the queue
        self.tasks.append((worker_pod.start_version, worker_pod.end_version))
        self.task_stats[worker_pod.name].increment_retry_count()
        self.current_workers[worker_idx] = None

    def process_completed_pod(self, worker_pod, worker_idx):
        duration = int(worker_pod.get_age_secs())

        if worker_pod.has_txn_mismatch():
            logger.info(f"Worker {worker_pod.name} failed with txn mismatch")
            self.txn_mismatch_logs.append(worker_pod.get_humio_log_link())

        if worker_pod.is_failed():
            reason = worker_pod.get_failure_reason()
            retries = self.task_stats[worker_pod.name].retry_count + 1
            if worker_pod.should_reschedule() and retries < MAX_RETRIES:
                logger.info(
                    f"Worker {worker_idx} completed: {worker_pod.name}, "
                    f"status=Failed({reason}), duration={duration}s, "
                    f"rescheduling (attempt {retries}/{MAX_RETRIES})"
                )
                # Don't set end_time — task will be re-dispatched
                self.kill_pod_and_reschedule_task(worker_pod, worker_idx)
            else:
                logger.info(
                    f"Worker {worker_idx} completed: {worker_pod.name}, "
                    f"status=Failed({reason}), duration={duration}s"
                )
                self.failed_workpod_logs.append(worker_pod.get_humio_log_link())
                self.current_workers[worker_idx] = None
                self.task_stats[worker_pod.name].set_end_time()
        else:
            logger.info(
                f"Worker {worker_idx} completed: {worker_pod.name}, "
                f"status=Succeeded, duration={duration}s"
            )
            self.task_stats[worker_pod.name].set_succeeded()
            self.current_workers[worker_idx] = None
            self.task_stats[worker_pod.name].set_end_time()

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

    def print_stats(self):
        for key, value in self.task_stats.items():
            logger.info(f"{key}: {value}")

    def _log_worker_summary(
        self,
        phase_start_time: float,
        tasks_remaining: int | None = None,
        log_level: int = logging.INFO,
    ):
        """Dump status of every worker slot."""
        from collections import Counter
        log = lambda msg: logger.log(log_level, msg)
        phase_counts = Counter()
        header = f"=== Worker status (elapsed={int(time.time() - phase_start_time)}s"
        if tasks_remaining is not None:
            dispatched = self.total_tasks - tasks_remaining
            pct = int(dispatched / self.total_tasks * 100) if self.total_tasks > 0 else 100
            header += f", dispatched {dispatched}/{self.total_tasks} — {pct}%"
        header += ") ==="
        log("")
        log(header)
        empty_count = 0
        for idx, worker in enumerate(self.current_workers):
            if worker is None:
                empty_count += 1
                continue
            phase = worker.get_phase()
            phase_counts[phase] += 1
            detail = ""
            if phase not in (LocalPhase.SUCCEEDED, LocalPhase.FAILED, LocalPhase.RUNNING):
                detail = f", container={worker.get_container_status_summary()}"
            log(
                f"  Worker {idx}: {worker.name}, phase={phase}, age={int(worker.get_age_secs())}s{detail}"
            )
        summary = ", ".join(
            f"{phase}={count}" for phase, count in sorted(phase_counts.items(), key=lambda x: x[0].value)
        )
        log(f"  Summary: {summary}, empty={empty_count}")
        log("=== End worker status ==="  )
        log("")

    # read skip ranges from gcp bucket


def read_skip_ranges(network: str) -> tuple[int, int, list[tuple[int, int]]]:
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

    end = int(
        json.loads(
            urllib.request.urlopen(fullnode_api_url(network)).read().decode()
        )["ledger_version"]
    )

    return (data["start"], end, skip_ranges)


def fullnode_api_url(network: str) -> str:
    return f"https://fullnode.{network}.aptoslabs.com/v1"


@retry(
    stop=stop_after_attempt(5),
    wait=wait_fixed(2),
    retry=retry_if_exception_type((urllib.error.HTTPError, urllib.error.URLError)),
)
def get_txn_timestamp_usecs(network: str, version: int) -> int:
    """Get the timestamp (in microseconds) of a transaction by version."""
    url = f"{fullnode_api_url(network)}/transactions/by_version/{version}"
    data = json.loads(urllib.request.urlopen(url).read().decode())
    return int(data["timestamp"])


def timestamp_to_version(network: str, target_usecs: int, lo: int, hi: int) -> int:
    """Binary search for the version closest to the target timestamp."""
    while lo < hi:
        mid = (lo + hi) // 2
        mid_ts = get_txn_timestamp_usecs(network, mid)
        if mid_ts < target_usecs:
            lo = mid + 1
        else:
            hi = mid
    return lo


def parse_timestamp(s: str) -> int:
    """Parse a timestamp string into microseconds since epoch.

    Uses the dateparser library which supports a wide range of formats including:
      - Relative:  "2 hours ago", "30 minutes ago", "1 day ago"
      - Date only: "2026-03-19"
      - Date+time: "2026-03-19 10:00", "2026-03-19 10:00:00"
      - ISO 8601:  "2026-03-19T10:00:00Z", "2026-03-19T10:00:00+00:00"
      - And many more natural language formats.
    All inputs without explicit timezone are interpreted as UTC.
    """
    dt = dateparser.parse(s, settings={"TIMEZONE": "UTC", "RETURN_AS_TIMEZONE_AWARE": True})
    if dt is None:
        raise ValueError(f"Unable to parse timestamp: {s!r}")
    return int(dt.timestamp() * 1_000_000)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("--network", required=True, choices=["testnet", "mainnet"])
    parser.add_argument("--start", required=False, type=int)
    parser.add_argument("--end", required=False, type=int)
    parser.add_argument(
        "--start-time",
        required=False,
        type=str,
        help='Start time. Accepts any format supported by dateparser: relative '
             '("2 hours ago", "1 day ago"), date ("2026-03-19"), datetime '
             '("2026-03-19 10:00"), ISO 8601, etc. UTC assumed. '
             "Mutually exclusive with --start.",
    )
    parser.add_argument(
        "--end-time",
        required=False,
        type=str,
        help='End time. Same formats as --start-time. Mutually exclusive with --end.',
    )
    parser.add_argument("--worker_cnt", required=False, type=int)
    parser.add_argument("--range_size", required=False, type=int)
    parser.add_argument(
        "--namespace", required=False, type=str, default="replay-verify"
    )
    parser.add_argument("--image_tag", required=False, type=str)
    parser.add_argument("--image_profile", required=False, type=str, default="performance")
    parser.add_argument("--cleanup", required=False, action="store_true", default=False)
    args = parser.parse_args()

    if args.start is not None and args.start_time is not None:
        parser.error("--start and --start-time are mutually exclusive")
    if args.end is not None and args.end_time is not None:
        parser.error("--end and --end-time are mutually exclusive")

    return args


def get_image(profile: str, image_tag: str | None = None) -> str:
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
    tag_prefix = "" if profile == "release" else f"{profile}_"
    full_image = f"{forge.GAR_REPO_NAME}/{image_name}:{tag_prefix}{default_latest_image}"
    return full_image


def print_logs(failed_workpod_logs: list[str], txn_mismatch_logs: list[str]) -> None:
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
    image = get_image(profile=args.image_profile, image_tag=args.image_tag)
    run_id = f"{datetime.datetime.now().strftime('%Y%m%d-%H%M%S')}-{image[-5:]}"
    network = Network.from_string(args.network)
    config = ReplayConfig(network)
    worker_cnt = args.worker_cnt if args.worker_cnt else config.pvc_number * 10
    range_size = args.range_size if args.range_size else config.range_size

    # Resolve time-based args to versions
    if args.start_time is not None:
        target_usecs = parse_timestamp(args.start_time)
        args.start = timestamp_to_version(args.network, target_usecs, start, end)
        logger.info(f"Resolved --start-time {args.start_time} to version {args.start}")
    if args.end_time is not None:
        target_usecs = parse_timestamp(args.end_time)
        args.end = timestamp_to_version(args.network, target_usecs, start, end)
        logger.info(f"Resolved --end-time {args.end_time} to version {args.end}")

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
        scheduler.create_all_required_pvcs()
        try:
            start_time = time.time()
            (failed_logs, txn_mismatch_logs) = scheduler.schedule(from_scratch=True)
            scheduler.print_stats()
            print_logs(failed_logs, txn_mismatch_logs)
            if txn_mismatch_logs:
                url = scheduler.humio_hash_mismatch_url(start_time, time.time())
                logger.error(
                    f"Transaction mismatch logs found. All mismatch logs: {url}"
                )
                exit(2)
            if len(failed_logs) > 0:
                logger.error("Failed tasks found.")
                exit(1)
        finally:
            scheduler.cleanup()
