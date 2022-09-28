// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{Get, K8sApi, Result, KUBECTL_BIN};
use std::{process::Command, sync::Arc, time::Duration};

use anyhow::bail;
use aptos_retrier::ExponentWithLimitDelay;
use k8s_openapi::api::{apps::v1::StatefulSet, core::v1::Pod};

use again::RetryPolicy;
use aptos_logger::info;
use json_patch::{Patch as JsonPatch, PatchOperation, ReplaceOperation};
use kube::{
    api::{Api, Meta, Patch, PatchParams},
    client::Client as K8sClient,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::create_k8s_client;

#[derive(Error, Debug)]
#[error("{0}")]
enum WorkloadScalingError {
    RetryableError(String),
    FinalError(String),
}

pub struct KubeImage {
    pub name: String,
    pub tag: String,
}

pub fn get_stateful_set_image(stateful_set: &StatefulSet) -> Result<KubeImage> {
    let s = stateful_set
        .spec
        .as_ref()
        .expect("Failed to get StatefulSet spec")
        .template
        .spec
        .as_ref()
        .expect("Failed to get StatefulSet pod spec")
        .containers[0]
        .image
        .as_ref()
        .expect("Failed to get StatefulSet image")
        .split(':')
        .collect::<Vec<&str>>();

    Ok(KubeImage {
        name: s[0].to_string(),
        tag: s[1].to_string(),
    })
}

/// Waits for a single K8s StatefulSet to be ready
pub async fn wait_stateful_set(
    kube_client: &K8sClient,
    kube_namespace: &str,
    sts_name: &str,
    desired_replicas: u64,
    retry_policy: RetryPolicy,
) -> Result<()> {
    let stateful_set_api = Arc::new(K8sApi::<StatefulSet>::from_client(
        kube_client.clone(),
        Some(kube_namespace.to_string()),
    ));
    let pod_api = Arc::new(K8sApi::<Pod>::from_client(
        kube_client.clone(),
        Some(kube_namespace.to_string()),
    ));
    retry_policy
        .retry_if(
            move || {
                check_stateful_set_status(
                    stateful_set_api.clone(),
                    pod_api.clone(),
                    sts_name,
                    desired_replicas,
                )
            },
            |e: &WorkloadScalingError| matches!(e, WorkloadScalingError::RetryableError(_)),
        )
        .await?;

    Ok(())
}

/// Checks the status of a single K8s StatefulSet. Also inspects the pods to make sure they are all ready.
async fn check_stateful_set_status(
    stateful_set_api: Arc<dyn Get<StatefulSet>>,
    pod_api: Arc<dyn Get<Pod>>,
    sts_name: &str,
    desired_replicas: u64,
) -> Result<(), WorkloadScalingError> {
    match stateful_set_api.get(sts_name).await {
        Ok(s) => {
            let sts_name = &s.name();
            // get the StatefulSet status
            if let Some(sts_status) = s.status {
                let ready_replicas = sts_status.ready_replicas.unwrap_or(0) as u64;
                let replicas = sts_status.replicas as u64;
                if ready_replicas == replicas && replicas == desired_replicas {
                    info!(
                        "StatefulSet {} has scaled to {}",
                        sts_name, desired_replicas
                    );
                    return Ok(());
                }
                info!(
                    "StatefulSet {} has {}/{} replicas",
                    sts_name, ready_replicas, desired_replicas
                );
            }
            let pod_name = format!("{}-0", sts_name);
            // Get the StatefulSet's Pod status
            if let Some(status) = pod_api
                .get(&pod_name)
                .await
                .map_err(|e| WorkloadScalingError::RetryableError(e.to_string()))?
                .status
            {
                if let Some(ref container_statuses) = status.container_statuses {
                    if let Some(container_status) = container_statuses.last() {
                        if let Some(state) = &container_status.state {
                            if let Some(waiting) = &state.waiting {
                                if let Some(waiting_reason) = &waiting.reason {
                                    match waiting_reason.as_str() {
                                        "ImagePullBackOff" => {
                                            info!("Pod {} has ImagePullBackOff", &pod_name);
                                            return Err(WorkloadScalingError::FinalError(
                                                "ImagePullBackOff".to_string(),
                                            ));
                                        }
                                        "CrashLoopBackOff" => {
                                            info!("Pod {} has CrashLoopBackOff", &pod_name);
                                            return Err(WorkloadScalingError::FinalError(
                                                "CrashLoopBackOff".to_string(),
                                            ));
                                        }
                                        "ErrImagePull" => {
                                            info!("Pod {} has ErrImagePull", &pod_name);
                                            return Err(WorkloadScalingError::FinalError(
                                                "ErrImagePull".to_string(),
                                            ));
                                        }
                                        _ => {
                                            info!("Waiting for pod {}", &pod_name);
                                            return Err(WorkloadScalingError::RetryableError(
                                                format!("Waiting for pod {}", &pod_name),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(phase) = status.phase.as_ref() {
                    info!("Pod {} at phase {}", &pod_name, phase)
                }
                Err(WorkloadScalingError::RetryableError(format!(
                    "Retry due to pod {} status {:?}",
                    &pod_name, status
                )))
            } else {
                Err(WorkloadScalingError::FinalError(format!(
                    "Pod {} status not found",
                    &pod_name
                )))
            }
        }
        Err(e) => {
            info!("Failed to get StatefulSet: {}", e);
            Err(WorkloadScalingError::RetryableError(format!(
                "Failed to get StatefulSet: {}",
                e
            )))
        }
    }
}

/// Given the name of a node's StatefulSet, sets the node's image tag. Assumes that the StatefulSet has only one container
/// Note that this function will not wait for the StatefulSet to be ready.
pub async fn set_stateful_set_image_tag(
    stateful_set_name: String,
    container_name: String,
    image_tag: String,
    kube_namespace: String,
) -> Result<()> {
    let kube_client: K8sClient = create_k8s_client().await;
    let sts_api: Api<StatefulSet> = Api::namespaced(kube_client.clone(), &kube_namespace);
    let sts = sts_api.get(&stateful_set_name).await?;
    let image_repo = get_stateful_set_image(&sts)?.name;

    // replace the image tag
    let new_image = format!("{}:{}", &image_repo, &image_tag);

    // set the image using kubectl
    // patching the node spec may not work
    Command::new(KUBECTL_BIN)
        .args([
            "-n",
            &kube_namespace,
            "set",
            "image",
            &format!("statefulset/{}", &stateful_set_name),
            &format!("{}={}", &container_name, &new_image),
        ])
        .status()
        .expect("Failed to set image for StatefulSet");

    Ok(())
}

/// Scales the given StatefulSet to the given number of replicas
pub async fn scale_stateful_set_replicas(
    sts_name: &str,
    kube_namespace: &str,
    replica_num: u64,
) -> Result<()> {
    let kube_client = create_k8s_client().await;
    let stateful_set_api: Api<StatefulSet> = Api::namespaced(kube_client.clone(), kube_namespace);
    let pp = PatchParams::apply("forge").force();
    let patch = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "StatefulSet",
        "metadata": {
            "name": sts_name,
        },
        "spec": {
            "replicas": replica_num,
        }
    });
    let patch = Patch::Apply(&patch);
    stateful_set_api.patch(sts_name, &pp, &patch).await?;
    // retry for ~5 min at a fixed interval
    let retry_policy = RetryPolicy::fixed(Duration::from_secs(10)).with_max_retries(6 * 5);
    wait_stateful_set(
        &kube_client,
        kube_namespace,
        sts_name,
        replica_num,
        retry_policy,
    )
    .await?;

    Ok(())
}

pub async fn set_identity(
    sts_name: &str,
    kube_namespace: &str,
    k8s_secret_name: &str,
) -> Result<()> {
    let kube_client = create_k8s_client().await;
    let stateful_set_api: Api<StatefulSet> = Api::namespaced(kube_client.clone(), kube_namespace);
    let patch_op = PatchOperation::Replace(ReplaceOperation {
        // The json path below should match `terraform/helm/aptos-node/templates/validator.yaml`.
        path: "/spec/template/spec/volumes/1/secret/secretName".to_string(),
        value: json!(k8s_secret_name),
    });
    let patch: Patch<Value> = Patch::Json(JsonPatch(vec![patch_op]));
    let pp = PatchParams::apply("forge");
    stateful_set_api.patch(sts_name, &pp, &patch).await?;
    Ok(())
}

pub async fn get_identity(sts_name: &str, kube_namespace: &str) -> Result<String> {
    let kube_client = create_k8s_client().await;
    let stateful_set_api: Api<StatefulSet> = Api::namespaced(kube_client.clone(), kube_namespace);
    let sts = stateful_set_api.get(sts_name).await?;
    // The json path below should match `terraform/helm/aptos-node/templates/validator.yaml`.
    let secret_name = sts.spec.unwrap().template.spec.unwrap().volumes.unwrap()[1]
        .secret
        .clone()
        .unwrap()
        .secret_name
        .unwrap();
    Ok(secret_name)
}

pub async fn check_for_container_restart(
    kube_client: &K8sClient,
    kube_namespace: &str,
    sts_name: &str,
) -> Result<()> {
    aptos_retrier::retry_async(
        ExponentWithLimitDelay::new(1000, 10 * 1000, 60 * 1000),
        || {
            let pod_api: Api<Pod> = Api::namespaced(kube_client.clone(), kube_namespace);
            Box::pin(async move {
                // Get the StatefulSet's Pod status
                let pod_name = format!("{}-0", sts_name);
                if let Some(status) = pod_api.get_status(&pod_name).await?.status {
                    if let Some(container_statuses) = status.container_statuses {
                        for container_status in container_statuses {
                            if container_status.restart_count > 0 {
                                bail!(
                                    "Container {} in pod {} restarted {} times ",
                                    container_status.name,
                                    &pod_name,
                                    container_status.restart_count
                                );
                            }
                        }
                        return Ok(());
                    }
                    // In case of no restarts, k8 apis returns no container statuses
                    Ok(())
                } else {
                    bail!("Can't query the pod status for {}", sts_name)
                }
            })
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use k8s_openapi::api::apps::v1::StatefulSet;
    use k8s_openapi::api::apps::v1::{StatefulSetSpec, StatefulSetStatus};
    use k8s_openapi::api::core::v1::{
        ContainerState, ContainerStateWaiting, ContainerStatus, PodStatus,
    };
    use kube::{api::ObjectMeta, Error as KubeError};

    struct MockStatefulSetApi {
        stateful_set: StatefulSet,
    }

    impl MockStatefulSetApi {
        fn from_stateful_set(stateful_set: StatefulSet) -> Self {
            MockStatefulSetApi { stateful_set }
        }
    }

    #[async_trait]
    impl Get<StatefulSet> for MockStatefulSetApi {
        async fn get(&self, _name: &str) -> Result<StatefulSet, KubeError> {
            Ok(self.stateful_set.clone())
        }
    }

    struct MockPodApi {
        pod: Pod,
    }

    impl MockPodApi {
        fn from_pod(pod: Pod) -> Self {
            MockPodApi { pod }
        }
    }

    #[async_trait]
    impl Get<Pod> for MockPodApi {
        async fn get(&self, _name: &str) -> Result<Pod, KubeError> {
            Ok(self.pod.clone())
        }
    }

    #[tokio::test]
    async fn test_check_stateful_set_status() {
        // mock a StatefulSet with 0/1 replicas
        // this should then mean we check the underlying pod to see what's up
        let stateful_set_api = Arc::new(MockStatefulSetApi::from_stateful_set(StatefulSet {
            metadata: ObjectMeta {
                name: Some("test-stateful-set".to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(StatefulSetSpec {
                replicas: Some(1),
                ..StatefulSetSpec::default()
            }),
            status: Some(StatefulSetStatus {
                replicas: 1,
                ready_replicas: Some(0),
                ..StatefulSetStatus::default()
            }),
        }));

        // we should retry if the pod status is not explicitly bad
        let pod_default_api = Arc::new(MockPodApi::from_pod(Pod {
            status: Some(PodStatus::default()),
            ..Pod::default()
        }));
        let ret = check_stateful_set_status(
            stateful_set_api.clone(),
            pod_default_api.clone(),
            "test-stateful-set",
            1,
        )
        .await;
        assert!(matches!(
            ret.err(),
            Some(WorkloadScalingError::RetryableError(_))
        ));

        // the pod explicitly has a bad status, so we should fail fast
        let pod_default_api = Arc::new(MockPodApi::from_pod(Pod {
            metadata: ObjectMeta {
                name: Some("test-stateful-set-0".to_string()),
                ..ObjectMeta::default()
            },
            status: Some(PodStatus {
                container_statuses: Some(vec![ContainerStatus {
                    name: "test-container".to_string(),
                    restart_count: 0,
                    state: Some(ContainerState {
                        waiting: Some(ContainerStateWaiting {
                            reason: Some("CrashLoopBackOff".to_string()),
                            ..ContainerStateWaiting::default()
                        }),
                        ..ContainerState::default()
                    }),
                    ..ContainerStatus::default()
                }]),
                ..PodStatus::default()
            }),
            ..Pod::default()
        }));
        let ret = check_stateful_set_status(
            stateful_set_api.clone(),
            pod_default_api.clone(),
            "test-stateful-set",
            1,
        )
        .await;
        assert!(matches!(
            ret.err(),
            Some(WorkloadScalingError::FinalError(_))
        ));
    }
}
