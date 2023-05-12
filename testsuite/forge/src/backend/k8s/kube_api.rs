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
pub trait Get<K>: Send + Sync {
    async fn get(&self, name: &str) -> Result<K, KubeError>;
}

#[async_trait]
pub trait Create<K>: Send + Sync {
    async fn create(&self, pp: &PostParams, k: &K) -> Result<K, KubeError>;
}

#[async_trait]
impl<K> Get<K> for K8sApi<K>
where
    K: k8s_openapi::Resource + Send + Sync + Clone + DeserializeOwned + Serialize + Debug,
{
    async fn get(&self, name: &str) -> Result<K, KubeError> {
        self.api.get(name).await
    }
}

#[async_trait]
impl<K> Create<K> for K8sApi<K>
where
    K: k8s_openapi::Resource + Send + Sync + Clone + DeserializeOwned + Serialize + Debug,
{
    async fn create(&self, pp: &PostParams, k: &K) -> Result<K, KubeError> {
        self.api.create(pp, k).await
    }
}
