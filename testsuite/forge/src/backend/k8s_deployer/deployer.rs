// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    FORGE_DEPLOYER_IMAGE_TAG, FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME,
    FORGE_DEPLOYER_VALUES_ENV_VAR_NAME, FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO,
    FORGE_TESTNET_DEPLOYER_DOCKER_IMAGE_REPO,
};
use crate::{maybe_create_k8s_resource, K8sApi, KubeClientApi, ReadWrite, Result};
use k8s_openapi::{
    api::{
        batch::v1::Job,
        core::v1::{ConfigMap, Namespace, ServiceAccount},
        rbac::v1::RoleBinding,
    },
    Resource,
};
use kube::{
    api::{DynamicObject, ObjectMeta, PostParams},
    ResourceExt,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, fmt, sync::Arc};

/// The ForgeDeployerManager is responsible for managing the lifecycle of forge deployers, wihch deploy the
/// forge components to the k8s cluster.
pub struct ForgeBackendManager {}

impl ForgeBackendManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Gets a k8s configmap for the forge deployer that contains the values needed to deploy the forge components
    /// Does not actually create the configmap in k8s
    fn build_configmap(&self, values: &serde_json::Value) -> Result<ConfigMap> {
        let configmap_name = values.get("name").unwrap().as_str().unwrap().to_string();
        let deploy_values_json = serde_json::to_string(&values)?;

        // create the configmap with values
        let config_map = ConfigMap {
            metadata: ObjectMeta {
                name: Some(configmap_name.clone()),
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

    /// Gets a k8s job for the forge deployer that implements the particular interface that it expects:
    /// - Runs the corresponding forge-<component>-deployer image
    /// - Sets the FORGE_DEPLOY_VALUES_JSON environment variable to the configmap that contains the values
    /// Does not actually create the job in k8s
    fn build_job(&self, values: &serde_json::Value) -> Result<Job> {
        let job_name = values.get("name").unwrap().as_str().unwrap().to_string();
        let image = values.get("image").unwrap().as_str().unwrap().to_string();

        // Check out how concise this is!
        let job: Job = serde_json::from_value(json!({
            "metadata": {
                "name": job_name,
            },
            "spec": {
                "template": {
                    "spec": {
                        "service_account_name": FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME,
                        "containers": [{
                            "name": "forge-deployer",
                            "image": image,
                            "env": [{
                                "name": FORGE_DEPLOYER_VALUES_ENV_VAR_NAME,
                                "value_from": {
                                    "config_map_key_ref": {
                                            "name": job_name,
                                            "key": "deploy-values.json",
                                        },
                                },
                            }],
                        }],
                        "restart_policy": "Never",
                    },
                },
                "backoff_limit": 0,
            },
        }))?;

        Ok(job)
    }

    fn dynamic<T: Resource + Serialize>(&self, resource: &T) -> Result<DynamicObject> {
        // This is so dumb
        let serialized_resource = serde_json::to_string(&resource)?;
        Ok(serde_json::from_str(&serialized_resource)?)
    }

    fn from_dynamic<T: Resource + DeserializeOwned>(&self, resource: &DynamicObject) -> Result<T> {
        let serialized_resource = serde_json::to_string(&resource)?;
        Ok(serde_json::from_str(&serialized_resource)?)
    }

    pub async fn start(
        &self,
        client: Arc<dyn KubeClientApi>,
        values: serde_json::Value,
    ) -> Result<()> {
        let namespace = self.dynamic(&self.build_namespace(client.get_namespace()))?;
        client.create(&namespace).await?;

        let configmap = self.dynamic(&self.build_configmap(&values)?)?;
        client.create(&configmap).await?;

        let job = self.dynamic(&self.build_job(&values)?)?;
        client.create(&job).await?;

        Ok(())
    }

    fn build_namespace(&self, namespace: String) -> Namespace {
        Namespace {
            metadata: ObjectMeta {
                name: Some(namespace),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn build_serviceaccount(&self, namespace: String) -> ServiceAccount {
        // create a serviceaccount FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME
        ServiceAccount {
            metadata: ObjectMeta {
                name: Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn build_rolebinding(&self, namespace: String) -> RoleBinding {
        // create a rolebinding for the service account to the clusterrole cluster-admin
        RoleBinding {
            metadata: ObjectMeta {
                name: Some("forge-admin".to_string()),
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
                namespace: Some(namespace),
                ..Default::default()
            }]),
        }
    }

    pub async fn completed(
        &self,
        client: Arc<dyn KubeClientApi>,
        job_name: String,
    ) -> Result<bool> {
        let job: Job = self.from_dynamic(&client.get(Job::KIND, &job_name).await?)?;
        Ok(job
            .status
            .expect("Failed to get job status")
            .succeeded
            .expect("Failed to get job succeeded number")
            > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeKubeClient, MockK8sResourceApi};
    use serde_json::json;

    /// Test creating a forge deployer manager and creating an indexer deployment with it. Nothing
    /// exists in the namespace yet
    #[tokio::test]
    async fn test_start_deployer_fresh_environment() {
        let values = json!({
            "profile": "large-banana",
            "era": "1",
        });
        let manager = ForgeBackendManager::new();
        let client = Arc::new(FakeKubeClient::new());
        manager.start(client, values).await.unwrap();
        // let indexer_deployer_name = manager.get_name(ForgeDeployerType::Indexer);
        // .expect(format!("Expected job {} to exist", indexer_deployer_name).as_str());
        // .expect(format!("Expected configmap {} to exist", indexer_deployer_name).as_str());
    }

    /// Test starting a deployer with an existing job in the namespace. This should fail as the job already exists
    /// and we cannot override/mutate it.
    #[tokio::test]
    async fn test_start_deployer_existing_job() {
        let values = json!({
            "profile": "large-banana",
            "era": "1",
        });
        let manager = ForgeBackendManager::new();
        let client = Arc::new(FakeKubeClient::new());
        let result = manager.start(client, values).await;
        assert!(result.is_err());
    }

    /// Test starting a deployer with an existing job in the namespace but a different era. This should be allowed
    /// as the new job/deployment will be in a different era and unrelated to the existing job
    #[tokio::test]
    async fn test_start_deployer_existing_job_different_era() {
        let values = json!({
            "profile": "large-banana",
            "era": "2",
        });
        let manager = ForgeBackendManager::new();
        let client = Arc::new(FakeKubeClient::new());
        manager.start(client, values).await.unwrap();
    }

    /// Test ensure_namespace_prepared creates the namespace, serviceaccount, and rolebinding
    /// Collisions should be OK to ensure idempotency
    #[tokio::test]
    async fn test_ensure_namespace_prepared_fresh_namespace() {
        let values = json!({
            "profile": "large-banana",
            "era": "1",
        });
        let manager = ForgeBackendManager::new();
        let client = Arc::new(FakeKubeClient::new());
        manager.start(client, values).await.unwrap();
        // .expect("Issue ensuring namespace prepared");
        // .expect(format!("Expected namespace {} to exist", "forge-large-banana").as_str());
        // assert_eq!(
        //     namespace.metadata.name,
        //     Some("forge-large-banana".to_string())
        // );
        // let serviceaccount = manager
        //     .serviceaccount_api
        //     .get(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME)
        //     .await
        //     .expect(
        //         format!(
        //             "Expected serviceaccount {} to exist",
        //             FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME
        //         )
        //         .as_str(),
        //     );
        // assert_eq!(
        //     serviceaccount.metadata.name,
        //     Some(FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME.to_string())
        // );
        // let rolebinding = manager.rolebinding_api.get("forge-admin").await.unwrap();
        // assert_eq!(rolebinding.metadata.name, Some("forge-admin".to_string()));
    }

    /// Test the same thing but with existing resources. This should not error out and should be idempotent
    #[tokio::test]
    async fn test_ensure_namespace_prepared_existing_resources() {
        let values = json!({
            "profile": "large-banana",
            "era": "1",
        });
        let manager = ForgeBackendManager::new();
        // manager
        //     .create_namespace()
        //     .await
        //     .expect("Issue ensuring namespace prepared");
    }
}
