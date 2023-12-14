// Copyright © Aptos Foundation
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
use futures::{stream::FuturesUnordered, StreamExt};
use rand::seq::SliceRandom;
use std::{convert::TryFrom, time::Instant};
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
        api_key: Option<String>,
    ) -> Result<Self> {
        let num_peers = peers.len();

        let mut instance_states = Vec::new();
        let mut errors = Vec::new();
        let start = Instant::now();
        let futures = FuturesUnordered::new();
        for url in &peers {
            let instance = Instance::new(
                format!(
                    "{}:{}",
                    url.host().unwrap(),
                    url.port_or_known_default().unwrap()
                ), /* short_hash */
                url.clone(),
                None,
                api_key.clone(),
            );
            futures.push(async move {
                let result = instance.rest_client().get_ledger_information().await;
                (instance, result)
            });
        }

        let results: Vec<_> = futures.collect().await;
        let fetch_time_s = start.elapsed().as_secs();
        for (instance, result) in results {
            match result {
                Ok(v) => instance_states.push((instance, v.into_inner())),
                Err(err) => {
                    warn!(
                        "Excluding client {} because failing to fetch the ledger information",
                        instance.peer_name()
                    );
                    errors.push(err)
                },
            }
        }

        if !errors.is_empty() {
            warn!(
                "Failed to build some endpoints for the cluster: {:?}",
                errors
            );
        }

        let mut instances = Vec::new();
        let max_timestamp = instance_states
            .iter()
            .map(|(_, s)| s.timestamp_usecs / 1000000)
            .max()
            .unwrap();

        for (instance, state) in instance_states.into_iter() {
            let state_timestamp = state.timestamp_usecs / 1000000;
            if state.chain_id != chain_id.id() {
                warn!(
                    "Excluding client {} running wrong chain {}",
                    instance.peer_name(),
                    state.chain_id
                );
            } else if state_timestamp + 20 + fetch_time_s < max_timestamp {
                warn!(
                    "Excluding Client {} too stale, {}, while chain at {} (delta of {}s)",
                    instance.peer_name(),
                    state_timestamp,
                    max_timestamp,
                    max_timestamp - state_timestamp,
                );
            } else {
                info!(
                    "Client {} is healthy, adding to the list of end points for load testing",
                    instance.peer_name()
                );
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
        for url in &args.get_targets()? {
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

        // some sanity check around the URL and whether we expect an API key or not
        // just print it out for now rather than enforcing, since this is subject to change
        for url in &urls {
            if url.host_str().unwrap().starts_with("api.") {
                if args.node_api_key.is_none() {
                    println!("URL {} starts with api.* but no API key was provided. Hint: generate one at https://developers.aptoslabs.com", url);
                }
            } else if args.node_api_key.is_some() {
                println!(
                    "URL {} does not start with api.* but an API key was provided. You may not need it.",
                    url
                );
            }
        }

        let (coin_source_key, is_root) = args.coin_source_args.get_private_key()?;

        let cluster = Cluster::from_host_port(
            urls,
            coin_source_key,
            is_root,
            args.chain_id,
            args.node_api_key.clone(),
        )
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
            account_key.authentication_key().account_address()
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
