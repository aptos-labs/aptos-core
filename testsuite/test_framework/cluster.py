# Cluster abstraction for the forge test framework

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
import os
from typing import Dict, List, Optional, TypedDict

from .shell import Shell

from tenacity import retry, wait_exponential, stop_after_attempt


class Cloud(Enum):
    AWS = "AWS"
    GCP = "GCP"


class AwsListClusterResult(TypedDict):
    clusters: List[str]


class AwsError(Exception):
    pass


class GcpError(Exception):
    pass


class GetPodsItemMetadata(TypedDict):
    name: str
    labels: dict


class GetPodsItemStatus(TypedDict):
    phase: str


class GetPodsItem(TypedDict):
    metadata: GetPodsItemMetadata
    status: GetPodsItemStatus


class GetPodsResult(TypedDict):
    items: List[GetPodsItem]


@dataclass
class ForgeCluster:
    name: str
    cloud: Cloud = Cloud.AWS
    region: Optional[str] = "us-west-2"
    kubeconf: Optional[str] = None
    is_multiregion: bool = False

    def __repr__(self) -> str:
        return f"{self.cloud}/{self.region}/{self.name}"

    def set_kubeconf(self, kubeconf: str) -> ForgeCluster:
        self.kubeconf = kubeconf
        return self

    @property
    def kubectl_create_context_arg(self) -> List[str]:
        return ["--context=karmada-apiserver"] if self.is_multiregion else []

    async def write(self, shell: Shell) -> None:
        assert self.kubeconf is not None, "kubeconf must be set"
        await self.write_cluster_config(shell, self.name, self.kubeconf)

    async def get_jobs(self, shell: Shell) -> List[ForgeJob]:
        assert self.kubeconf is not None, "kubeconf must be set"
        pod_result = (
            (
                await shell.gen_run(
                    [
                        "kubectl",
                        "get",
                        "pods",
                        "-n",
                        "default",
                        "--kubeconfig",
                        self.kubeconf,
                        "-o",
                        "json",
                    ]
                )
            )
            .unwrap()
            .decode()
        )
        pods_result: GetPodsResult = json.loads(pod_result)
        pods = pods_result["items"]
        forge_jobs = []

        # For each forge test runner pod, get the forge namespace and get the pods in that namespace
        # to infer the number of validators and fullnodes for each job
        for pod in pods:
            if (  # use the forge_namespace label to filter out forge pods
                not pod["metadata"]["name"].startswith("forge-")
                or "forge-namespace" not in pod["metadata"]["labels"]
            ):
                continue
            forge_namespace = pod["metadata"]["labels"]["forge-namespace"]
            forge_namespace_pods_result_str = (
                (
                    await shell.gen_run(
                        [
                            "kubectl",
                            "get",
                            "pods",
                            "-n",
                            forge_namespace,
                            "--kubeconfig",
                            self.kubeconf,
                            "-o",
                            "json",
                        ]
                    )
                )
                .unwrap()
                .decode()
            )
            forge_namespace_pods_result: GetPodsResult = json.loads(
                forge_namespace_pods_result_str
            )
            forge_namespace_pods = forge_namespace_pods_result["items"]
            validator_pods = [
                forge_pod
                for forge_pod in forge_namespace_pods
                if "validator" in forge_pod["metadata"]["name"]
            ]
            fullnode_pods = [
                forge_pod
                for forge_pod in forge_namespace_pods
                if "fullnode" in forge_pod["metadata"]["name"]
            ]
            job = ForgeJob.from_pod(self, pod)
            job.num_validators = len(validator_pods)
            job.num_fullnodes = len(fullnode_pods)
            forge_jobs.append(job)
        return forge_jobs

    def assert_auth(self, shell: Shell) -> None:
        if self.cloud == Cloud.AWS:
            list_eks_clusters(shell)
        elif self.cloud == Cloud.GCP:
            list_gke_clusters(shell)
        else:
            raise Exception("Unsupported cloud type")

    async def write_cluster_config(
        self, shell: Shell, cluster_name: str, temp: str
    ) -> None:
        if self.is_multiregion:
            cmd = [
                "gcloud",
                "secrets",
                "versions",
                "access",
                "latest",
                "--secret",
                "karmada-kubeconfig",
                "--project",
                "forge-gcp-multiregion-test",
                "--out-file",
                temp,
            ]
        elif self.cloud == Cloud.AWS:
            cmd = [
                "aws",
                "eks",
                "update-kubeconfig",
                "--name",
                cluster_name,
                "--kubeconfig",
                temp,
            ]
        elif self.cloud == Cloud.GCP:
            # set the KUBE_CONFIG to temp so the resulting kubeconfig is written to it
            os.environ["KUBECONFIG"] = temp
            # The project must already be set via: gcloud config set project <project>
            cmd = [
                "gcloud",
                "container",
                "clusters",
                "get-credentials",
                cluster_name,
                "--zone",
                self.region,
            ]
        else:
            raise Exception("Unsupported cloud type")
        (await shell.gen_run(cmd)).unwrap()


def list_eks_clusters(shell: Shell) -> Dict[str, ForgeCluster]:
    cluster_json = shell.run(["aws", "eks", "list-clusters"]).unwrap()
    # This type annotation is not enforced, just helpful
    try:
        cluster_result: AwsListClusterResult = json.loads(cluster_json.decode())
        clusters: Dict[str, ForgeCluster] = {}
        for cluster_name in cluster_result["clusters"]:
            if cluster_name.startswith("aptos-forge-"):
                clusters[cluster_name] = ForgeCluster(
                    cloud=Cloud.AWS,
                    name=cluster_name,
                )
        return clusters
    except Exception as e:
        raise AwsError("Failed to list EKS clusters") from e


@retry(wait=wait_exponential(multiplier=1, min=4, max=10), stop=stop_after_attempt(3))
def list_gke_clusters(shell: Shell) -> Dict[str, ForgeCluster]:
    cluster_json = shell.run(
        ["gcloud", "container", "clusters", "list", "--format=json"]
    ).unwrap()
    try:
        cluster_result = json.loads(cluster_json.decode())
        clusters: Dict[str, ForgeCluster] = {}
        for cluster_config in cluster_result:
            cluster_name = cluster_config["name"]
            if cluster_name.startswith("aptos-forge-"):
                clusters[cluster_name] = ForgeCluster(
                    cloud=Cloud.GCP,
                    name=cluster_name,
                    region=cluster_config["location"],
                )
        return clusters
    except Exception as e:
        raise GcpError("Failed to list GKE clusters") from e


def find_forge_cluster(
    shell: Shell, cloud: Cloud, name: str, kubeconf: str
) -> ForgeCluster:
    clusters: Dict[str, ForgeCluster] = {}
    if cloud == Cloud.AWS:
        clusters = list_eks_clusters(shell)
    else:
        clusters = list_gke_clusters(shell)
    if name not in clusters:
        raise Exception(f"Cluster {name} not found")
    return clusters[name].set_kubeconf(kubeconf)


@dataclass
class ForgeJob:
    name: str
    phase: str
    cluster: ForgeCluster
    num_validators: int = 0
    num_fullnodes: int = 0

    @classmethod
    def from_pod(cls, cluster: ForgeCluster, pod: GetPodsItem) -> ForgeJob:
        return cls(
            name=pod["metadata"]["name"],
            phase=pod["status"]["phase"],
            cluster=cluster,
        )

    def running(self):
        return self.phase == "Running"

    def succeeded(self):
        return self.phase == "Succeeded"

    def failed(self):
        return self.phase == "Failed"
