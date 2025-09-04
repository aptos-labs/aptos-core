// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use kube::{
    api::{Api, ListParams, PostParams},
    client::Client as K8sClient,
    Error as KubeError, Resource as ApiResource,
};
use serde::{de::DeserializeOwned, Serialize};

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
    async fn get_status(&self, name: &str) -> Result<K, KubeError>;
    async fn list(&self, lp: &ListParams) -> Result<Vec<K>, KubeError>;
}

// Implement the traits for K8sApi

#[async_trait]
impl<K> ReadWrite<K> for K8sApi<K>
where
    K: k8s_openapi::Resource + Send + Sync + Clone + DeserializeOwned + Serialize + std::fmt::Debug,
{
    async fn get(&self, name: &str) -> Result<K, KubeError> {
        self.api.get(name).await
    }

    async fn create(&self, pp: &PostParams, k: &K) -> Result<K, KubeError> {
        self.api.create(pp, k).await
    }

    async fn get_status(&self, name: &str) -> Result<K, KubeError> {
        self.api.get_status(name).await
    }

    async fn list(&self, lp: &ListParams) -> Result<Vec<K>, KubeError> {
        let list = self.api.list(lp).await?;
        Ok(list.items)
    }
}

#[cfg(test)]
pub mod mocks {
    use super::ReadWrite;
    use crate::Result;
    use velor_infallible::Mutex;
    use async_trait::async_trait;
    use hyper::StatusCode;
    use k8s_openapi::Metadata;
    use kube::{
        api::{ListParams, ObjectMeta, PostParams},
        error::ErrorResponse,
        Error as KubeError,
    };
    use std::{collections::BTreeMap, sync::Arc};

    /// Generic k8s resource mock API where resource names are unique. Use it to mock namespaced resources or cluster-wide resources, but
    /// not resources across multiple namespaces.
    #[derive(Clone)]
    pub struct MockK8sResourceApi<T> {
        resources: Arc<Mutex<BTreeMap<String, T>>>,
    }

    impl<T> MockK8sResourceApi<T>
    where
        T: Clone + Metadata<Ty = ObjectMeta> + Send + Sync, // Ensure T has the necessary traits
    {
        pub fn new() -> Self {
            MockK8sResourceApi {
                resources: Arc::new(Mutex::new(BTreeMap::new())),
            }
        }

        pub fn from_resource(resource: T) -> Self {
            let resources = Arc::new(Mutex::new(BTreeMap::new()));
            resources.lock().insert(
                resource
                    .metadata()
                    .name
                    .as_ref()
                    .expect("Expected metadata to have name")
                    .clone(),
                resource.clone(),
            );
            MockK8sResourceApi { resources }
        }

        pub fn from_resources(resources_vec: Vec<T>) -> Self {
            let resources = Arc::new(Mutex::new(BTreeMap::new()));
            for resource in resources_vec {
                resources.lock().insert(
                    resource
                        .metadata()
                        .name
                        .as_ref()
                        .expect("Expected metadata to have name")
                        .clone(),
                    resource.clone(),
                );
            }
            MockK8sResourceApi { resources }
        }
    }

    impl<T> Default for MockK8sResourceApi<T>
    where
        T: Clone + Metadata<Ty = ObjectMeta> + Send + Sync,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl<T> ReadWrite<T> for MockK8sResourceApi<T>
    where
        T: Clone + Metadata<Ty = ObjectMeta> + Send + Sync, // Ensure T has the necessary traits
    {
        async fn get(&self, name: &str) -> Result<T, KubeError> {
            let resources = self.resources.lock();
            if let Some(resource) = resources.get(name) {
                Ok(resource.clone())
            } else {
                Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: format!("Resource with name {} could not be found", name),
                    reason: "not_found".to_string(),
                    code: 404,
                }))
            }
        }

        async fn create(&self, _pp: &PostParams, resource: &T) -> Result<T, KubeError> {
            let mut resources = self.resources.lock();
            if resources.contains_key(
                resource
                    .metadata()
                    .name
                    .as_ref()
                    .expect("Expected metadata to have name"),
            ) {
                return Err(KubeError::Api(ErrorResponse {
                    status: "failed".to_string(),
                    message: "Resource with same name already exists".to_string(),
                    reason: "already_exists".to_string(),
                    code: 409,
                }));
            }
            resources.insert(
                resource
                    .metadata()
                    .name
                    .as_ref()
                    .expect("Expected metadata to have name")
                    .clone(),
                resource.clone(),
            );
            Ok(resource.clone())
        }

        async fn get_status(&self, _name: &str) -> Result<T, KubeError> {
            todo!()
        }

        async fn list(&self, _lp: &ListParams) -> Result<Vec<T>, KubeError> {
            todo!()
        }
    }

    // Mock API that always fails to create a new Namespace

    pub struct FailedK8sResourceApi {
        status_code: u16,
    }

    impl FailedK8sResourceApi {
        pub fn from_status_code(status_code: u16) -> Self {
            FailedK8sResourceApi { status_code }
        }
    }

    #[async_trait]
    impl<T> ReadWrite<T> for FailedK8sResourceApi
    where
        T: Clone + Metadata<Ty = ObjectMeta> + Send + Sync, // Ensure T has the necessary traits
    {
        async fn get(&self, _name: &str) -> Result<T, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to get resource".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }

        async fn create(&self, _pp: &PostParams, _resource: &T) -> Result<T, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to create resource".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }

        async fn get_status(&self, _name: &str) -> Result<T, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to get resource status".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }

        async fn list(&self, _lp: &ListParams) -> Result<Vec<T>, KubeError> {
            let status = StatusCode::from_u16(self.status_code).unwrap();
            Err(KubeError::Api(ErrorResponse {
                status: status.to_string(),
                code: status.as_u16(),
                message: "Failed to list resources".to_string(),
                reason: "Failed to parse error data".into(),
            }))
        }
    }
}
