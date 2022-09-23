// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{emitter::query_sequence_number, instance::Instance, ClusterArgs};
use anyhow::{anyhow, bail, format_err, Result};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
    Uniform,
};
use aptos_logger::{info, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{account_config::aptos_test_root_address, chain_id::ChainId, AccountKey, LocalAccount},
};
use rand::seq::SliceRandom;
use std::convert::TryFrom;
use url::Url;

#[derive(Debug)]
pub struct Cluster {
    instances: Vec<Instance>,
    mint_key_pair: KeyPair<Ed25519PrivateKey, Ed25519PublicKey>,
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
        mint_key: Ed25519PrivateKey,
        chain_id: ChainId,
        reuse_accounts: bool,
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

        let mint_key_pair = if reuse_accounts {
            dummy_key_pair()
        } else {
            KeyPair::from(mint_key)
        };

        Ok(Self {
            instances,
            mint_key_pair,
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

        let mint_key = args.mint_args.get_mint_key()?;

        let cluster = Cluster::from_host_port(urls, mint_key, args.chain_id, args.reuse_accounts)
            .await
            .map_err(|e| format_err!("failed to create a cluster from host and port: {:?}", e))?;

        Ok(cluster)
    }

    fn account_key(&self) -> AccountKey {
        AccountKey::from_private_key(clone(&self.mint_key_pair.private_key))
    }

    async fn load_account_with_mint_key(
        &self,
        client: &RestClient,
        address: AccountAddress,
    ) -> Result<LocalAccount> {
        let sequence_number = query_sequence_number(client, address).await.map_err(|e| {
            format_err!(
                "query_sequence_number on {:?} for account {} failed: {:?}",
                client,
                address,
                e
            )
        })?;
        Ok(LocalAccount::new(
            address,
            self.account_key(),
            sequence_number,
        ))
    }

    pub async fn load_aptos_root_account(&self, client: &RestClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, aptos_test_root_address())
            .await
    }

    pub async fn load_faucet_account(&self, client: &RestClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, aptos_test_root_address())
            .await
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

pub fn dummy_key_pair() -> KeyPair<Ed25519PrivateKey, Ed25519PublicKey> {
    Ed25519PrivateKey::generate_for_testing().into()
}
