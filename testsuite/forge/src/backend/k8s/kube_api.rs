// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use kube::{
    api::{Api, PostParams},
    client::Client as K8sClient,
    Error as KubeError, Resource as ApiResource,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

// Create kube API wrapper traits such that they are testable

#[derive(Clone)]
pub struct K8sApi<K> {
    api: Api<K>,
}

impl<K> K8sApi<K>
where
    K: ApiResource,
{
    pub fn from_client(kube_client: K8sClient, kube_namespace: Option<String>) -> Self
    where
        <K as ApiResource>::DynamicType: Default,
    {
        if let Some(kube_namespace) = kube_namespace {
            K8sApi {
                api: Api::namespaced(kube_client, &kube_namespace),
            }
        } else {
            K8sApi {
                api: Api::all(kube_client),
            }
        }
    }
}

#[async_trait]
pub trait ReadWrite<K>: Send + Sync {
    async fn get(&self, name: &str) -> Result<K, KubeError>;
    async fn create(&self, pp: &PostParams, k: &K) -> Result<K, KubeError>;
}

// Implement the traits for K8sApi

#[async_trait]
impl<K> ReadWrite<K> for K8sApi<K>
where
    K: k8s_openapi::Resource + Send + Sync + Clone + DeserializeOwned + Serialize + Debug,
{
    async fn get(&self, name: &str) -> Result<K, KubeError> {
        self.api.get(name).await
    }

    async fn create(&self, pp: &PostParams, k: &K) -> Result<K, KubeError> {
        self.api.create(pp, k).await
    }
}

#[cfg(test)]
pub mod mocks {
    use super::*;
    use crate::Result;
    use async_trait::async_trait;
    use hyper::StatusCode;
    use k8s_openapi::api::{
        apps::v1::StatefulSet,
        core::v1::{ConfigMap, Namespace, PersistentVolumeClaim, Pod, Secret, Service},
    };
    use kube::{api::PostParams, error::ErrorResponse, Error as KubeError};

    // Mock StatefulSet API

    pub struct MockStatefulSetApi {
        stateful_set: StatefulSet,
    }

    impl MockStatefulSetApi {
        pub fn from_stateful_set(stateful_set: StatefulSet) -> Self {
            MockStatefulSetApi { stateful_set }
        }
    }

    #[async_trait]
    impl ReadWrite<StatefulSet> for MockStatefulSetApi {
        async fn get(&self, name: &str) -> Result<StatefulSet, KubeError> {
            if self.stateful_set.metadata.name == Some(name.to_string()) {
                return Ok(self.stateful_set.clone());
            }
            return Err(KubeError::Api(ErrorResponse {
                status: "failed".to_string(),
                message: format!(
                    "StatefulSet with name {} could not be found in {:?}",
                    name, self.stateful_set
                ),
                reason: "not_found".to_string(),
                code: 404,
            }));
        }

        async fn create(
            &self,
            _pp: &PostParams,
            stateful_set: &StatefulSet,
        ) -> Result<StatefulSet, KubeError> {
            if self.stateful_set.metadata.name == stateful_set.metadata.name {
                return Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: format!(
                        "StatefulSet with same name already exists in {:?}",
                        self.stateful_set
                    ),
                    reason: "already_exists".to_string(),
                    code: 409,
                }));
            }
            Ok(self.stateful_set.clone())
        }
    }

    // Mock Pod API

    pub struct MockPodApi {
        pod: Pod,
    }

    impl MockPodApi {
        pub fn from_pod(pod: Pod) -> Self {
            MockPodApi { pod }
        }
    }

    #[async_trait]
    impl ReadWrite<Pod> for MockPodApi {
        async fn get(&self, _name: &str) -> Result<Pod, KubeError> {
            Ok(self.pod.clone())
        }

        async fn create(&self, _pp: &PostParams, _pod: &Pod) -> Result<Pod, KubeError> {
            Ok(self.pod.clone())
        }
    }

    // Mock ConfigMap API

    pub struct MockConfigMapApi {
        config_map: ConfigMap,
    }

    impl MockConfigMapApi {
        pub fn from_config_map(config_map: ConfigMap) -> Self {
            MockConfigMapApi { config_map }
        }
    }

    #[async_trait]
    impl ReadWrite<ConfigMap> for MockConfigMapApi {
        async fn get(&self, name: &str) -> Result<ConfigMap, KubeError> {
            if self.config_map.metadata.name == Some(name.to_string()) {
                return Ok(self.config_map.clone());
            }
            return Err(KubeError::Api(ErrorResponse {
                status: "failed".to_string(),
                message: format!(
                    "ConfigMap with name {} could not be found in {:?}",
                    name, self.config_map
                ),
                reason: "not_found".to_string(),
                code: 404,
            }));
        }

        async fn create(
            &self,
            _pp: &PostParams,
            config_map: &ConfigMap,
        ) -> Result<ConfigMap, KubeError> {
            if self.config_map.metadata.name == config_map.metadata.name {
                return Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: format!(
                        "ConfigMap with same name already exists in {:?}",
                        self.config_map
                    ),
                    reason: "already_exists".to_string(),
                    code: 409,
                }));
            }
            Ok(self.config_map.clone())
        }
    }

    // Mock PersistentVolumeClaim API

    pub struct MockPersistentVolumeClaimApi {
        persistent_volume_claim: PersistentVolumeClaim,
    }

    impl MockPersistentVolumeClaimApi {
        pub fn from_persistent_volume_claim(
            persistent_volume_claim: PersistentVolumeClaim,
        ) -> Self {
            MockPersistentVolumeClaimApi {
                persistent_volume_claim,
            }
        }
    }

    #[async_trait]
    impl ReadWrite<PersistentVolumeClaim> for MockPersistentVolumeClaimApi {
        async fn get(&self, name: &str) -> Result<PersistentVolumeClaim, KubeError> {
            if self.persistent_volume_claim.metadata.name == Some(name.to_string()) {
                return Ok(self.persistent_volume_claim.clone());
            }
            return Err(KubeError::Api(ErrorResponse {
                status: "failed".to_string(),
                message: format!(
                    "PersistentVolumeClaim with name {} could not be found in {:?}",
                    name, self.persistent_volume_claim
                ),
                reason: "not_found".to_string(),
                code: 404,
            }));
        }

        async fn create(
            &self,
            _pp: &PostParams,
            persistent_volume_claim: &PersistentVolumeClaim,
        ) -> Result<PersistentVolumeClaim, KubeError> {
            if self.persistent_volume_claim.metadata.name == persistent_volume_claim.metadata.name {
                return Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: format!(
                        "PersistentVolumeClaim with same name already exists in {:?}",
                        self.persistent_volume_claim
                    ),
                    reason: "already_exists".to_string(),
                    code: 409,
                }));
            }
            Ok(self.persistent_volume_claim.clone())
        }
    }

    // Mock Service API

    pub struct MockServiceApi {
        service: Service,
    }

    impl MockServiceApi {
        pub fn from_service(service: Service) -> Self {
            MockServiceApi { service }
        }
    }

    #[async_trait]
    impl ReadWrite<Service> for MockServiceApi {
        async fn get(&self, name: &str) -> Result<Service, KubeError> {
            if self.service.metadata.name == Some(name.to_string()) {
                return Ok(self.service.clone());
            }
            return Err(KubeError::Api(ErrorResponse {
                status: "failed".to_string(),
                message: format!(
                    "Service with name {} could not be found in {:?}",
                    name, self.service
                ),
                reason: "not_found".to_string(),
                code: 404,
            }));
        }

        async fn create(&self, _pp: &PostParams, service: &Service) -> Result<Service, KubeError> {
            if self.service.metadata.name == service.metadata.name {
                return Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: format!(
                        "Service with same name already exists in {:?}",
                        self.service
                    ),
                    reason: "already_exists".to_string(),
                    code: 409,
                }));
            }
            Ok(self.service.clone())
        }
    }

    // Mock Service API
    pub struct MockSecretApi {
        secret: Option<Secret>,
    }

    impl MockSecretApi {
        pub fn from_secret(secret: Option<Secret>) -> Self {
            MockSecretApi { secret }
        }
    }

    #[async_trait]
    impl ReadWrite<Secret> for MockSecretApi {
        async fn get(&self, _name: &str) -> Result<Secret, KubeError> {
            match self.secret {
                Some(ref s) => Ok(s.clone()),
                None => Err(KubeError::Api(ErrorResponse {
                    status: "status".to_string(),
                    message: "message".to_string(),
                    reason: "reason".to_string(),
                    code: 404,
                })),
            }
        }

        async fn create(&self, _pp: &PostParams, secret: &Secret) -> Result<Secret, KubeError> {
            return Ok(secret.clone());
        }
    }

    // Mock API that always fails to create a new Namespace

    pub struct FailedNamespacesApi {
        status_code: u16,
    }

    impl FailedNamespacesApi {
        pub fn from_status_code(status_code: u16) -> Self {
            FailedNamespacesApi { status_code }
        }
    }

    #[async_trait]
    impl ReadWrite<Namespace> for FailedNamespacesApi {
        async fn get(&self, _name: &str) -> Result<Namespace, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to get namespace".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }

        async fn create(
            &self,
            _pp: &PostParams,
            _namespace: &Namespace,
        ) -> Result<Namespace, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to create namespace".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }
    }
}
