// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{Factory, GenesisConfig, GenesisConfigFn, NodeConfigFn, Result, Swarm, Version};
use anyhow::bail;
use aptos_logger::info;
use rand::rngs::StdRng;
use std::{convert::TryInto, num::NonZeroUsize, time::Duration};

use aptos_sdk::crypto::ed25519::ED25519_PRIVATE_KEY_LENGTH;
pub use testnet::*;
pub use constants::*;
pub use kube_api::*;
pub use node::K8sNode;
pub use stateful_set::*;
pub use swarm::*;

pub struct K8sNativeFactory {
    root_key: [u8; ED25519_PRIVATE_KEY_LENGTH],
    image_tag: String,
    upgrade_image_tag: String,
    kube_namespace: String,
}

impl K8sNativeFactory {
    pub fn new(
        kube_namespace: String,
        image_tag: String,
        upgrade_image_tag: String,
    ) -> Result<K8sNativeFactory> {
        let root_key: [u8; ED25519_PRIVATE_KEY_LENGTH] =
            hex::decode(DEFAULT_ROOT_PRIV_KEY)?.try_into().unwrap();

        match kube_namespace.as_str() {
            "default" => {
                info!("Using the default kubernetes namespace");
            },
            s if s.starts_with("forge") => {
                info!("Using forge namespace: {}", s);
            },
            _ => {
                bail!(
                    "Invalid kubernetes namespace provided: {}. Use forge-*",
                    kube_namespace
                );
            },
        }

        Ok(Self {
            root_key,
            image_tag,
            upgrade_image_tag,
            kube_namespace,
        })
    }
}

#[async_trait::async_trait]
impl Factory for K8sNativeFactory {
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        let version = vec![
            Version::new(0, self.image_tag.clone()),
            Version::new(1, self.upgrade_image_tag.clone()),
        ];
        Box::new(version.into_iter())
    }

    async fn launch_swarm(
        &self,
        _rng: &mut StdRng,
        num_validators: NonZeroUsize,
        num_fullnodes: usize,
        init_version: &Version,
        genesis_version: &Version,
        genesis_config: Option<&GenesisConfig>,
        cleanup_duration: Duration,
        genesis_config_fn: Option<GenesisConfigFn>,
        node_config_fn: Option<NodeConfigFn>,
        existing_db_tag: Option<String>,
    ) -> Result<Box<dyn Swarm>> {
        let genesis_modules_path = match genesis_config {
            Some(config) => match config {
                GenesisConfig::Bundle(_) => {
                    bail!("k8s_native forge backend does not support raw bytes as genesis modules. please specify a path instead")
                },
                GenesisConfig::Path(path) => Some(path.clone()),
            },
            None => None,
        };

        let kube_client = create_k8s_client().await;
        let (validators, fullnodes) = {
            // clear the cluster of resources
            delete_k8s_resources(kube_client.clone(), &self.kube_namespace).await?;
            // create the forge-management configmap before installing anything
            create_management_configmap(self.kube_namespace.clone(), self.keep, cleanup_duration)
                .await?;
            // create a secret to access pyroscope
            create_pyroscope_secret(self.kube_namespace.clone()).await?;
            if let Some(existing_db_tag) = existing_db_tag {
                bail!("k8s_native forge backend does not support existing_db_tag");
            }
            // try installing testnet resources, but clean up if it fails
            match install_testnet_resources(
                self.kube_namespace.clone(),
                num_validators.get(),
                num_fullnodes,
                format!("{}", init_version),
                format!("{}", genesis_version),
                genesis_modules_path,
                self.use_port_forward,
                self.enable_haproxy,
                genesis_config_fn,
                node_config_fn,
            )
            .await
            {
                Ok(res) => res,
                Err(e) => {
                    uninstall_testnet_resources(self.kube_namespace.clone()).await?;
                    bail!(e);
                },
            }
        };

        let swarm = K8sSwarm::new(
            &self.root_key,
            &self.image_tag,
            &self.upgrade_image_tag,
            &self.kube_namespace,
            validators,
            fullnodes,
            self.keep,
        )
        .await
        .unwrap();
        Ok(Box::new(swarm))
    }
}
