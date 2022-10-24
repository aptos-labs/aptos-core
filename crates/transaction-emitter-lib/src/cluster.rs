// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{emitter::query_sequence_number, instance::Instance, ClusterArgs};
use anyhow::{anyhow, bail, format_err, Result};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
};
use aptos_logger::{info, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::{
    account_config::aptos_test_root_address, chain_id::ChainId, AccountKey, LocalAccount,
};
use rand::seq::SliceRandom;
use std::convert::TryFrom;
use url::Url;

#[derive(Debug)]
pub struct Cluster {
    instances: Vec<Instance>,
    coin_source_key_pair: KeyPair<Ed25519PrivateKey, Ed25519PublicKey>,
    pub coin_source_is_root: bool,
    pub chain_id: ChainId,
}

fn clone(key: &Ed25519PrivateKey) -> Ed25519PrivateKey {
    let serialized: &[u8] = &(key.to_bytes());
    Ed25519PrivateKey::try_from(serialized).unwrap()
}

impl Cluster {
    /// We assume the URLs have been validated at this point, specifically to
    /// confirm that they have a host and port set.
    pub async fn from_host_port(
        peers: Vec<Url>,
        coin_source_key: Ed25519PrivateKey,
        coin_source_is_root: bool,
        chain_id: ChainId,
    ) -> Result<Self> {
        let num_peers = peers.len();

        let mut instance_states = Vec::new();
        let mut errors = Vec::new();
        for url in &peers {
            let instance = Instance::new(
                format!(
                    "{}:{}",
                    url.host().unwrap(),
                    url.port_or_known_default().unwrap()
                ), /* short_hash */
                url.clone(),
                None,
            );
            match instance.rest_client().get_ledger_information().await {
                Ok(v) => instance_states.push((instance, v.into_inner())),
                Err(err) => errors.push(err),
            }
        }

        if !errors.is_empty() {
            warn!(
                "Failed to build some endpoints for the cluster: {:?}",
                errors
            );
        }

        let mut instances = Vec::new();
        let max_version = instance_states
            .iter()
            .map(|(_, s)| s.version)
            .max()
            .unwrap();

        for (instance, state) in instance_states.into_iter() {
            if state.chain_id != chain_id.id() {
                warn!(
                    "Client {} running wrong chain {}",
                    instance.peer_name(),
                    state.chain_id
                );
            } else if state.version + 100000 < max_version {
                warn!(
                    "Client {} too stale, {}, while chain at {}",
                    instance.peer_name(),
                    state.version,
                    max_version
                );
            } else {
                instances.push(instance);
            }
        }

        if instances.is_empty() {
            return Err(anyhow!(
                "None of the rest endpoints provided are reachable: {:?}",
                errors
            ));
        }

        info!(
            "Creating the cluster with {}/{} end points",
            instances.len(),
            num_peers
        );

        Ok(Self {
            instances,
            coin_source_key_pair: KeyPair::from(coin_source_key),
            coin_source_is_root,
            chain_id,
        })
    }

    pub async fn try_from_cluster_args(args: &ClusterArgs) -> Result<Self> {
        let mut urls = Vec::new();
        for url in &args.targets {
            if !url.has_host() {
                bail!("No host found in URL: {}", url);
            }
            let mut url = url.clone();
            if url.port_or_known_default().is_none() {
                url.set_port(Some(8080))
                    .map_err(|_| format_err!("Failed to set port unexpectedly"))?;
            }
            urls.push(url);
        }

        let (coin_source_key, is_root) = args.coin_source_args.get_private_key()?;

        let cluster = Cluster::from_host_port(urls, coin_source_key, is_root, args.chain_id)
            .await
            .map_err(|e| format_err!("failed to create a cluster from host and port: {:?}", e))?;

        Ok(cluster)
    }

    fn account_key(&self) -> AccountKey {
        AccountKey::from_private_key(clone(&self.coin_source_key_pair.private_key))
    }

    pub async fn load_coin_source_account(&self, client: &RestClient) -> Result<LocalAccount> {
        let account_key = self.account_key();
        let address = if self.coin_source_is_root {
            aptos_test_root_address()
        } else {
            account_key.authentication_key().derived_address()
        };

        let sequence_number = query_sequence_number(client, address).await.map_err(|e| {
            format_err!(
                "query_sequence_number on {:?} for account {} failed: {:?}",
                client,
                address,
                e
            )
        })?;
        Ok(LocalAccount::new(address, account_key, sequence_number))
    }

    pub fn random_instance(&self) -> Instance {
        let mut rnd = rand::thread_rng();
        self.instances
            .choose(&mut rnd)
            .expect("random_validator_instance requires non-empty validator_instances")
            .clone()
    }

    pub fn all_instances(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter()
    }
}
