// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    DEFAULT_FORGE_DEPLOYER_IMAGE_TAG, FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME,
    FORGE_DEPLOYER_VALUES_ENV_VAR_NAME,
};
use crate::{k8s_wait_indexer_strategy, maybe_create_k8s_resource, K8sApi, ReadWrite, Result};
use anyhow::bail;
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{ConfigMap, Namespace, ServiceAccount},
    rbac::v1::RoleBinding,
};
use kube::{
    api::{ObjectMeta, PostParams},
    ResourceExt,
};
use std::{collections::BTreeMap, sync::Arc};

/// The ForgeDeployerManager is responsible for managing the lifecycle of forge deployers, which deploy the
/// forge components to the k8s cluster.
pub struct ForgeDeployerManager {
    // all the k8s APIs we need. Specifying each API separately allows for easier testing
    pub jobs_api: Arc<dyn ReadWrite<Job>>,
    pub config_maps_api: Arc<dyn ReadWrite<ConfigMap>>,
    pub namespace_api: Arc<dyn ReadWrite<Namespace>>,
    pub serviceaccount_api: Arc<dyn ReadWrite<ServiceAccount>>,
    pub rolebinding_api: Arc<dyn ReadWrite<RoleBinding>>,

    pub namespace: String,
    pub image_repo: String,
    pub image_tag: Option<String>,

    // the values to use for the deployer, including namespace, era, etc
    pub config: serde_json::Value,
}

impl ForgeDeployerManager {
    pub fn new(
        kube_client: kube::Client,
        namespace: String,
        image_repo: String,
        image_tag: Option<String>,
        config: serde_json::Value,
    ) -> Self {
        let jobs_api = Arc::new(K8sApi::from_client(
            kube_client.clone(),
            Some(namespace.clone()),
        ));
        let config_maps_api = Arc::new(K8sApi::from_client(
            kube_client.clone(),
            Some(namespace.clone()),
        ));
        let namespace_api = Arc::new(K8sApi::from_client(kube_client.clone(), None));
        let serviceaccount_api = Arc::new(K8sApi::from_client(
            kube_client.clone(),
            Some(namespace.clone()),
        ));
        let rolebinding_api = Arc::new(K8sApi::from_client(
            kube_client.clone(),
            Some(namespace.clone()),
        ));

        // ensure it lives long enough between async
        Self {
            jobs_api,
            config_maps_api,
            namespace_api,
            serviceaccount_api,
            rolebinding_api,
            namespace,
            image_repo,
            image_tag,
            config,
        }
    }

    /// Return the canonical name for the deployer and each of its components
    pub fn get_name(&self) -> String {
        // derive the deployer_type from the image_repo. The type is the last part of the image repo
        let deployer_type = self
            .image_repo
            .split('/')
            .last()
            .expect("Failed to get deployer type from image repo");
        deployer_type.to_string()
    }

    /// Builds a k8s configmap for the forge deployer that contains the values needed to deploy the forge components
    /// Does not actually create the configmap in k8s
    fn build_forge_deployer_k8s_config_map(&self) -> Result<ConfigMap> {
        let configmap_name = self.get_name();
        let deploy_values_json = serde_json::to_string(&self.config)?;

        // create the configmap with values
        let config_map = ConfigMap {
            metadata: ObjectMeta {
                name: Some(configmap_name.clone()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            data: Some(BTreeMap::from([(
                "deploy-values.json".to_string(),
                deploy_values_json,
            )])),
            ..Default::default()
        };

        Ok(config_map)
    }

    /// Builds a k8s job for the forge deployer that implements the particular interface that it expects:
    /// - Runs the corresponding forge-<component>-deployer image
    /// - Sets the FORGE_DEPLOY_VALUES_JSON environment variable to the configmap that contains the values
    /// Does not actually create the job in k8s
    fn build_forge_deployer_k8s_job(&self, configmap_name: String) -> Result<Job> {
        let job_name = self.get_name();
        let image_repo: &str = &self.image_repo;
        let image_tag: &str = match self.image_tag {
            Some(ref tag) => tag,
            None => DEFAULT_FORGE_DEPLOYER_IMAGE_TAG,
        };

        let job = Job {
            metadata: ObjectMeta {
                name: Some(job_name.clone()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::batch::v1::JobSpec {
                template: k8s_openapi::api::core::v1::PodTemplateSpec {
                    spec: Some(k8s_openapi::api::core::v1::PodSpec {
                        service_account_name: Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string()),
                        containers: vec![k8s_openapi::api::core::v1::Container {
                            name: "forge-deployer".to_string(),
                            image: Some(format!("{}:{}", image_repo, image_tag)),
                            env: Some(vec![k8s_openapi::api::core::v1::EnvVar {
                                name: FORGE_DEPLOYER_VALUES_ENV_VAR_NAME.to_string(),
                                value_from: Some(k8s_openapi::api::core::v1::EnvVarSource {
                                    config_map_key_ref: Some(
                                        k8s_openapi::api::core::v1::ConfigMapKeySelector {
                                            name: Some(configmap_name),
                                            key: "deploy-values.json".to_string(),
                                            ..Default::default()
                                        },
                                    ),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        }],
                        restart_policy: Some("Never".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                backoff_limit: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(job)
    }

    pub async fn start(&self) -> Result<()> {
        self.ensure_namespace_prepared().await?;
        let config_map = self.build_forge_deployer_k8s_config_map()?;
        let job = self.build_forge_deployer_k8s_job(config_map.name())?;
        self.config_maps_api
            .create(&PostParams::default(), &config_map)
            .await?;
        self.jobs_api.create(&PostParams::default(), &job).await?;
        Ok(())
    }

    fn build_namespace(&self) -> Namespace {
        Namespace {
            metadata: ObjectMeta {
                name: Some(self.namespace.clone()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn build_service_account(&self) -> ServiceAccount {
        ServiceAccount {
            metadata: ObjectMeta {
                name: Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn build_role_binding(&self) -> RoleBinding {
        RoleBinding {
            metadata: ObjectMeta {
                name: Some("forge-admin".to_string()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            role_ref: k8s_openapi::api::rbac::v1::RoleRef {
                api_group: "rbac.authorization.k8s.io".to_string(),
                kind: "ClusterRole".to_string(),
                name: "cluster-admin".to_string(),
            },
            subjects: Some(vec![k8s_openapi::api::rbac::v1::Subject {
                kind: "ServiceAccount".to_string(),
                name: FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string(),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            }]),
        }
    }

    async fn ensure_namespace_prepared(&self) -> Result<()> {
        let namespace = self.build_namespace();
        maybe_create_k8s_resource(self.namespace_api.clone(), namespace.clone()).await?;
        let service_account = self.build_service_account();
        maybe_create_k8s_resource(self.serviceaccount_api.clone(), service_account).await?;
        let role_binding = self.build_role_binding();
        maybe_create_k8s_resource(self.rolebinding_api.clone(), role_binding).await?;
        Ok(())
    }

    /**
     * Wait for the deployer job to complete.
     */
    pub async fn wait_completed(&self) -> Result<()> {
        aptos_retrier::retry_async(k8s_wait_indexer_strategy(), || {
            Box::pin(async move {
                let job_name: String = self.get_name();
                let job = self.jobs_api.get(&job_name).await?;
                let completed = job
                    .status
                    .as_ref()
                    .expect("Failed to get job status")
                    .succeeded
                    .expect("Failed to get job succeeded number")
                    > 0;
                if completed {
                    Ok(())
                } else {
                    bail!("Job not completed yet: {:?}", job);
                }
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockK8sResourceApi, FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO};
    use serde_json::json;

    fn get_mock_forge_deployer_manager() -> ForgeDeployerManager {
        let namespace = "forge-large-banana".to_string();
        let config = serde_json::from_value(json!(
            {
                "profile": "large-banana",
                "era": "1",
                "namespace": namespace,
            }
        ))
        .expect("Issue creating Forge deployer config");
        ForgeDeployerManager {
            jobs_api: Arc::new(MockK8sResourceApi::new()),
            config_maps_api: Arc::new(MockK8sResourceApi::new()),
            namespace_api: Arc::new(MockK8sResourceApi::new()),
            serviceaccount_api: Arc::new(MockK8sResourceApi::new()),
            rolebinding_api: Arc::new(MockK8sResourceApi::new()),
            namespace,
            image_repo: FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO.to_string(),
            image_tag: None,
            config,
        }
    }

    /// Test creating a forge deployer manager and creating an indexer deployment with it. Nothing
    /// exists in the namespace yet
    #[tokio::test]
    async fn test_start_deployer_fresh_environment() {
        let manager = get_mock_forge_deployer_manager();
        manager.start().await.unwrap();
        let indexer_deployer_name = manager.get_name();
        manager
            .jobs_api
            .get(&indexer_deployer_name)
            .await
            .unwrap_or_else(|_| panic!("Expected job {} to exist", indexer_deployer_name));
        manager
            .config_maps_api
            .get(&indexer_deployer_name)
            .await
            .unwrap_or_else(|_| panic!("Expected configmap {} to exist", indexer_deployer_name));
    }

    /// Test starting a deployer with an existing job in the namespace. This should fail as the job already exists
    /// and we cannot override/mutate it.
    #[tokio::test]
    async fn test_start_deployer_existing_job() {
        let mut manager = get_mock_forge_deployer_manager();
        manager.jobs_api = Arc::new(MockK8sResourceApi::from_resource(Job {
            metadata: ObjectMeta {
                name: Some(manager.get_name()),
                namespace: Some(manager.namespace.clone()),
                ..Default::default()
            },
            ..Default::default()
        }));
        let result = manager.start().await;
        assert!(result.is_err());
    }

    /// Test ensure_namespace_prepared creates the namespace, serviceaccount, and rolebinding
    /// Collisions should be OK to ensure idempotency
    #[tokio::test]
    async fn test_ensure_namespace_prepared_fresh_namespace() {
        let manager = get_mock_forge_deployer_manager();
        manager
            .ensure_namespace_prepared()
            .await
            .expect("Issue ensuring namespace prepared");
        let namespace = manager
            .namespace_api
            .get("forge-large-banana")
            .await
            .unwrap_or_else(|_| panic!("Expected namespace {} to exist", "forge-large-banana"));
        assert_eq!(
            namespace.metadata.name,
            Some("forge-large-banana".to_string())
        );
        let serviceaccount = manager
            .serviceaccount_api
            .get(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Expected serviceaccount {} to exist",
                    FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME
                )
            });
        assert_eq!(
            serviceaccount.metadata.name,
            Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string())
        );
        let rolebinding = manager.rolebinding_api.get("forge-admin").await.unwrap();
        assert_eq!(rolebinding.metadata.name, Some("forge-admin".to_string()));
    }

    /// Test the same thing but with existing resources. This should not error out and should be idempotent
    #[tokio::test]
    async fn test_ensure_namespace_prepared_existing_resources() {
        let mut manager = get_mock_forge_deployer_manager();
        manager.namespace_api = Arc::new(MockK8sResourceApi::from_resource(Namespace {
            metadata: ObjectMeta {
                name: Some("forge-large-banana".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }));
        manager.serviceaccount_api = Arc::new(MockK8sResourceApi::from_resource(ServiceAccount {
            metadata: ObjectMeta {
                name: Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string()),
                namespace: Some("forge-large-banana".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }));
        manager.rolebinding_api = Arc::new(MockK8sResourceApi::from_resource(RoleBinding {
            metadata: ObjectMeta {
                name: Some("forge-admin".to_string()),
                namespace: Some("forge-large-banana".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }));
        manager
            .ensure_namespace_prepared()
            .await
            .expect("Issue ensuring namespace prepared");
    }
}
