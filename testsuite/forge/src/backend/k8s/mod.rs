// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{Factory, GenesisConfig, Result, Swarm, Version};
use anyhow::bail;
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
    helm_repo: String,
    image_tag: String,
    base_image_tag: String,
}

// These are test keys for forge ephemeral networks. Do not use these elsewhere!
pub const DEFAULT_ROOT_KEY: &str =
    "48136DF3174A3DE92AFDB375FFE116908B69FF6FAB9B1410E548A33FEA1D159D";
const DEFAULT_ROOT_PRIV_KEY: &str =
    "E25708D90C72A53B400B27FC7602C4D546C7B7469FA6E12544F0EBFB2F16AE19";

impl K8sFactory {
    pub fn new(helm_repo: String, image_tag: String, base_image_tag: String) -> Result<K8sFactory> {
        let root_key: [u8; ED25519_PRIVATE_KEY_LENGTH] =
            hex::decode(DEFAULT_ROOT_PRIV_KEY)?.try_into().unwrap();

        Ok(Self {
            root_key,
            helm_repo,
            image_tag,
            base_image_tag,
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

        uninstall_testnet_resources().await?;
        let era = reinstall_testnet_resources(
            self.helm_repo.clone(),
            node_num.get(),
            format!("{}", init_version),
            format!("{}", genesis_version),
            false,
            genesis_modules_path,
        )
        .await?;

        let swarm = K8sSwarm::new(
            &self.root_key,
            &self.helm_repo,
            &self.image_tag,
            &self.base_image_tag,
            format!("{}", init_version).as_str(),
            &era,
        )
        .await
        .unwrap();
        Ok(Box::new(swarm))
    }
}
