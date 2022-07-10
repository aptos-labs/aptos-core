// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{Factory, GenesisConfig, Result, Swarm, Version};
use anyhow::bail;
use aptos_logger::info;
use rand::rngs::StdRng;
use std::{convert::TryInto, num::NonZeroUsize};

mod cluster_helper;
mod node;
mod swarm;

pub use cluster_helper::*;
pub use node::K8sNode;
pub use swarm::*;

use aptos_sdk::crypto::ed25519::ED25519_PRIVATE_KEY_LENGTH;

pub struct K8sFactory {
    root_key: [u8; ED25519_PRIVATE_KEY_LENGTH],
    image_tag: String,
    base_image_tag: String,
    kube_namespace: String,
    use_port_forward: bool,
    keep: bool,
}

// These are test keys for forge ephemeral networks. Do not use these elsewhere!
pub const DEFAULT_ROOT_KEY: &str =
    "48136DF3174A3DE92AFDB375FFE116908B69FF6FAB9B1410E548A33FEA1D159D";
const DEFAULT_ROOT_PRIV_KEY: &str =
    "E25708D90C72A53B400B27FC7602C4D546C7B7469FA6E12544F0EBFB2F16AE19";

impl K8sFactory {
    pub fn new(
        kube_namespace: String,
        image_tag: String,
        base_image_tag: String,
        use_port_forward: bool,
        keep: bool,
    ) -> Result<K8sFactory> {
        let root_key: [u8; ED25519_PRIVATE_KEY_LENGTH] =
            hex::decode(DEFAULT_ROOT_PRIV_KEY)?.try_into().unwrap();

        match kube_namespace.as_str() {
            "default" => {
                info!("Using the default kubernetes namespace");
            }
            s if s.starts_with("forge") => {
                info!("Using forge namespace: {}", s);
            }
            _ => {
                bail!(
                    "Invalid kubernetes namespace provided: {}. Use forge-*",
                    kube_namespace
                );
            }
        }

        Ok(Self {
            root_key,
            image_tag,
            base_image_tag,
            kube_namespace,
            use_port_forward,
            keep,
        })
    }
}

#[async_trait::async_trait]
impl Factory for K8sFactory {
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        let version = vec![
            Version::new(0, self.base_image_tag.clone()),
            Version::new(1, self.image_tag.clone()),
        ];
        Box::new(version.into_iter())
    }

    async fn launch_swarm(
        &self,
        _rng: &mut StdRng,
        node_num: NonZeroUsize,
        init_version: &Version,
        genesis_version: &Version,
        genesis_config: Option<&GenesisConfig>,
    ) -> Result<Box<dyn Swarm>> {
        let genesis_modules_path = match genesis_config {
            Some(config) => match config {
                GenesisConfig::Bytes(_) => {
                    bail!("k8s forge backend does not support raw bytes as genesis modules. please specify a path instead")
                }
                GenesisConfig::Path(path) => Some(path.clone()),
            },
            None => None,
        };

        // create the forge-management configmap before installing anything
        create_management_configmap(self.kube_namespace.clone(), self.keep).await?;

        // try installing testnet resources, but clean up if it fails
        let (_era, validators, fullnodes) = match install_testnet_resources(
            self.kube_namespace.clone(),
            node_num.get(),
            format!("{}", init_version),
            format!("{}", genesis_version),
            genesis_modules_path,
            self.use_port_forward,
        )
        .await
        {
            Ok(res) => res,
            Err(e) => {
                uninstall_testnet_resources(self.kube_namespace.clone()).await?;
                bail!(e);
            }
        };

        let swarm = K8sSwarm::new(
            &self.root_key,
            &self.image_tag,
            &self.base_image_tag,
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
