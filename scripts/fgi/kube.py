# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import json
import os
import random
import subprocess
import tempfile
import time

FORGE_K8S_CLUSTERS = [
    "forge-0",
    "forge-1",
]

WORKSPACE_CHART_BUCKETS = {
    "forge-0": "aptos-testnet-forge-0-helm-312428ba",
    "forge-1": "aptos-testnet-forge-1-helm-a2b65112",
    "forge-dev": "aptos-testnet-forge-dev-helm-8d0a5291",
}

AWS_ACCOUNT = (
    subprocess.check_output(
        ["aws", "sts", "get-caller-identity",
            "--query", "Account", "--output", "text"],
        stderr=subprocess.DEVNULL,
        encoding="UTF-8",
    ).strip()
    if not os.getenv("AWS_ACCOUNT")
    else os.getenv("AWS_ACCOUNT")
)


# ================ Kube job ================
def create_forge_job(context, user, tag, base_tag, timeout_secs, forge_envs, forge_args):
    """Create the Forge K8s Job template"""
    job_name = f"forge-{user}-{int(time.time())}"
    job_name = job_name.replace("_", "-")  # underscore not allowed in pod name
    cluster_name = get_cluster_name_from_context(context)

    # job template to spin up. Edit this in place
    template = json.loads(
        subprocess.check_output(
            [
                "kubectl",
                "-o=json",
                f"--context={context}",
                "get",
                "job",
                "--selector=app.kubernetes.io/name=forge-debug",
            ],
            stderr=subprocess.DEVNULL,
            encoding="UTF-8",
        )
    )
    if len(template["items"]) != 1:
        print("ERROR: there must be exactly one forge-debug job")
        return None

    template = template["items"][0]

    # delete old spec details
    del template["spec"]["selector"]["matchLabels"]["controller-uid"]
    del template["spec"]["template"]["metadata"]["labels"]["controller-uid"]
    del template["spec"]["template"]["metadata"]["labels"]["job-name"]
    # change job name, labels, and backoff limit
    template["metadata"]["name"] = job_name
    template["metadata"]["labels"]["app.kubernetes.io/name"] = "forge"
    template["spec"]["template"]["metadata"]["labels"][
        "app.kubernetes.io/name"
    ] = "forge"
    template["spec"]["backoffLimit"] = 0
    # change startup command with timeout and extra args
    cmd = template["spec"]["template"]["spec"]["containers"][0]["command"][2]
    template["spec"]["template"]["spec"]["containers"][0]["command"][2] = cmd.replace(
        "tail -f /dev/null",
        f"timeout {timeout_secs} forge {' '.join(forge_args)} test k8s-swarm --cluster-name {cluster_name} --image-tag {tag} --base-image-tag {base_tag}".strip(),
    )
    # additional environment variables
    for env_var in forge_envs:
        name, value = env_var.split("=")
        template["spec"]["template"]["spec"]["containers"][0]["env"].append(
            {"name": name, "value": value}
        )
    # new image tag
    image_repo, _ = template["spec"]["template"]["spec"]["containers"][0][
        "image"
    ].split(":")
    template["spec"]["template"]["spec"]["containers"][0][
        "image"
    ] = f"{image_repo}:{tag}"
    return job_name, template


# ================ Kube queries ================


def get_cluster_context(cluster_name):
    """Get the Forge cluster context for use with kubectl"""
    return f"arn:aws:eks:us-west-2:{AWS_ACCOUNT}:cluster/aptos-{cluster_name}"


def get_cluster_name_from_context(context):
    """Get the Forge cluster name from the context"""
    return context.split("/")[1]


def kube_ensure_cluster(clusters):
    """Returns the workspace name of a cluster that is free, otherwise None"""
    attempts = 360
    for attempt in range(attempts):
        for cluster in clusters:
            context = get_cluster_context(cluster)
            running_pods = get_forge_pods_by_phase(context, "Running")
            pending_pods = get_forge_pods_by_phase(context, "Pending")
            monitoring_pods = get_monitoring_pod(context)

            # check pod status
            num_running_pods = len(running_pods["items"])
            num_pending_pods = len(pending_pods["items"])
            for pod in monitoring_pods["items"]:
                pod_name = pod["metadata"]["name"]
                healthy = pod["status"]["phase"] == "Running"
                if not healthy:
                    print(
                        f"{cluster} has an unhealthy monitoring pod {pod_name}. Skipping."
                    )
                    continue

            if num_running_pods > 0:
                print(
                    f"{cluster} has {num_running_pods} running forge pods. Skipping.")
            elif num_pending_pods > 0:
                print(
                    f"{cluster} has {num_pending_pods} pending forge pods. Skipping.")
            else:
                return cluster

        print(
            f"All clusters have jobs running on them. Retrying in 10 secs. Attempt: {attempt}/{attempts}"
        )
        time.sleep(10)
    print("Failed to schedule forge pod. All clusters are busy")
    return None


def kube_select_cluster():
    """
    randomly select a cluster that is free based on its pod status:
    - no other forge pods currently Running or Pending
    - all monitoring pods are ready
    """
    shuffled_clusters = random.sample(
        FORGE_K8S_CLUSTERS, len(FORGE_K8S_CLUSTERS))
    return kube_ensure_cluster(shuffled_clusters)


def kube_wait_job(job_name, context):
    """Wait for a K8s Job to be in a healthy state"""
    attempts = 360
    for _ in range(attempts):
        try:
            phase = get_forge_job_phase(job_name, context)
        except subprocess.CalledProcessError:
            print(f"kubectl get pod {job_name} failed. Retrying.")
            continue

        # pod is either Running, Succeeded, or assume it's working
        # https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
        if phase in ["Running", "Succeeded", "Unknown"]:
            print(f"{job_name} reached phase: {phase}")
            return 0

        if phase in ["Failed"]:
            print(f"{job_name} reached phase: {phase}")
            return 1

        # error pulling the image
        ret = subprocess.call(
            f"kubectl --context='{context}' get pod --selector=job-name={job_name} | grep -i -e ImagePullBackOff -e "
            f"InvalidImageName -e ErrImagePull",
            shell=True,
        )
        if ret == 0:
            image_name = get_forge_image_name(job_name, context)
            print(
                f"Job {job_name} failed to be scheduled because there was an error pulling the image: {image_name}"
            )
            subprocess.call(
                ["kubectl", f"--context={context}", "delete", "job", job_name]
            )
            return 1

        print(
            f"Waiting for {job_name} to be scheduled. Current phase: {phase}")
        time.sleep(1)

    print(f"Failed to schedule job: {job_name}")
    return 1


def kube_init_context(workspace=None):
    """Init the kube context for each available cluster, to ensure we can reach it"""
    try:
        subprocess.run(
            [
                "aws",
                "eks",
                "--region",
                "us-west-2",
                "describe-cluster",
                "--name",
                f"aptos-{FORGE_K8S_CLUSTERS[0]}",
            ],
            stdout=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        print("Failed to access EKS, try awsmfa?")
        raise
    # preserve the kube context by updating kubeconfig for the specified workspace
    clusters = FORGE_K8S_CLUSTERS + \
        [workspace] if workspace else FORGE_K8S_CLUSTERS
    for cluster in clusters:
        subprocess.run(
            [
                "aws",
                "eks",
                "--region",
                "us-west-2",
                "update-kubeconfig",
                "--name",
                f"aptos-{cluster}",
            ]
        )


# ================ Internal helpers ================


def get_forge_pods_by_phase(context, phase):
    """Get all Forge pods by phase"""
    try:
        return json.loads(
            subprocess.check_output(
                [
                    "kubectl",
                    "-o=json",
                    f"--context={context}",
                    "get",
                    "pods",
                    "--selector=app.kubernetes.io/name=forge",
                    f"--field-selector=status.phase=={phase}",
                ],
                stderr=subprocess.STDOUT,
                encoding="UTF-8",
            )
        )
    except subprocess.CalledProcessError as e:
        print(e.output)


def get_monitoring_pod(context):
    """Get all monitoring pods"""
    return json.loads(
        subprocess.check_output(
            [
                "kubectl",
                "-o=json",
                f"--context={context}",
                "get",
                "pods",
                "--selector=app.kubernetes.io/name=monitoring",
            ],
            stderr=subprocess.DEVNULL,
            encoding="UTF-8",
        )
    )


def get_forge_image_name(job_name, context):
    """Get the image name of the specified Forge job"""
    return get_forge_job_jsonpath(
        job_name, context, "{.items[0].spec.containers[0].image}"
    )


def get_forge_job_phase(job_name, context):
    """Get the current phase of the specified Forge job"""
    return get_forge_job_jsonpath(job_name, context, "{.items[0].status.phase}")


def get_forge_job_jsonpath(job_name, context, jsonpath):
    """Get the Forge job spec at the specified jsonpath"""
    return subprocess.check_output(
        [
            "kubectl",
            f"--context={context}",
            "get",
            "pod",
            f"--selector=job-name={job_name}",
            "-o",
            f"jsonpath={jsonpath}",
        ],
        encoding="UTF-8",
    )


def helm_s3_init(workspace):
    """Initializes the S3 bucket used as an internal Helm repo for Forge"""
    bucket_url = WORKSPACE_CHART_BUCKETS[workspace]
    subprocess.run(
        f"helm plugin install https://github.com/hypnoglow/helm-s3.git || true",
        shell=True,
        check=True
    )
    subprocess.run(
        ["helm", "s3", "init", f"s3://{bucket_url}/charts"],
        check=True
    )
    subprocess.run(
        ["helm", "repo", "add",
            f"testnet-{workspace}", f"s3://{bucket_url}/charts"],
        check=True
    )


def helm_package_push(chart_path, chart_name, workspace, dir):
    """Packages the helm charts at the given path and pushes it to the internal helm repo on S3"""
    subprocess.run(
        [
            "helm",
            "package",
            chart_path,
            "-d",
            dir,
            "--app-version",
            "1.0.0",
            "--version",
            "1.0.0"
        ],
        check=True
    )
    subprocess.run(
        f"helm s3 push --force {dir}/{chart_name}-*.tgz testnet-{workspace}",
        shell=True,
        check=True,
    )


def push_helm_charts(workspace):
    """
    Push all helm charts for usage by Forge
    Run from aptos-core root directory
    """
    helm_s3_init(workspace)
    tempdir = tempfile.mkdtemp()
    helm_package_push("terraform/testnet/testnet",
                      "testnet", workspace, tempdir)
    helm_package_push("terraform/helm/validator",
                      "aptos-validator", workspace, tempdir)
    helm_package_push("terraform/helm/fullnode",
                      "aptos-fullnode", workspace, tempdir)
