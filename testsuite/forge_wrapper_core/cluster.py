from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
import os
from typing import List, Optional, TypedDict

from .shell import Shell


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


def list_eks_clusters(shell: Shell) -> List[str]:
    cluster_json = shell.run(["aws", "eks", "list-clusters"]).unwrap()
    # This type annotation is not enforced, just helpful
    try:
        cluster_result: AwsListClusterResult = json.loads(cluster_json.decode())
        clusters = []
        for cluster_name in cluster_result["clusters"]:
            if cluster_name.startswith("aptos-forge-"):
                clusters.append(cluster_name)
        return clusters
    except Exception as e:
        raise AwsError("Failed to list eks clusters") from e


def list_gke_clusters(shell: Shell) -> List[str]:
    cluster_json = shell.run(
        ["gcloud", "container", "clusters", "list", "--format=json"]
    ).unwrap()
    try:
        cluster_result = json.loads(cluster_json.decode())
        clusters = []
        for cluster_config in cluster_result:
            cluster_name = cluster_config["name"]
            if cluster_name.startswith("aptos-forge-"):
                clusters.append(cluster_name)
        return clusters
    except Exception as e:
        raise GcpError("Failed to list eks clusters") from e


@dataclass
class ForgeCluster:
    name: str
    kubeconf: str
    cloud: Cloud = Cloud.AWS
    region: Optional[str] = "us-west-2"
    zone: Optional[str] = None

    async def write(self, shell: Shell) -> None:
        await self.write_cluster_config(shell, self.name, self.kubeconf)

    async def get_jobs(self, shell: Shell) -> List[ForgeJob]:
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
        if self.cloud == Cloud.AWS:
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
            cmd = [
                "gcloud",
                "container",
                "clusters",
                "get-credentials",
                cluster_name,
                "--zone",
                # The default zone for now.
                # The project must already be set via: gcloud config set project <project>
                "us-central1-c",
            ]
        else:
            raise Exception("Unsupported cloud type")
        (await shell.gen_run(cmd)).unwrap()


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
