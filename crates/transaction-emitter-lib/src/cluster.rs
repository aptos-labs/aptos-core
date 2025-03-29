// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{emitter::load_specific_account, instance::Instance, ClusterArgs};
use anyhow::{anyhow, bail, format_err, Result};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
};
use aptos_rest_client::{Client as RestClient, State};
use aptos_sdk::types::{chain_id::ChainId, AccountKey, LocalAccount};
use futures::{stream::FuturesUnordered, StreamExt};
use log::{info, warn};
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
        maybe_chain_id: Option<ChainId>,
        maybe_api_key: Option<String>,
    ) -> Result<Self> {
        let num_peers = peers.len();

        let mut instance_states = Vec::new();
        let mut errors = Vec::new();
        let fetch_timestamp = aptos_infallible::duration_since_epoch().as_secs();
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
                maybe_api_key.clone(),
            );
            futures.push(async move {
                let result = instance.rest_client().get_ledger_information().await;
                (instance, result)
            });
        }

        let results: Vec<_> = futures.collect().await;

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
        if max_timestamp + 10 < fetch_timestamp {
            return Err(anyhow!(
                "None of the rest endpoints provided have chain timestamp within 10s of local time: {} < {}",
                max_timestamp,
                fetch_timestamp,
            ));
        }

        let chain_id_from_instances = get_chain_id_from_instances(instance_states.clone())?;
        let chain_id: ChainId = match maybe_chain_id {
            Some(c) => c,
            None => {
                warn!(
                    "Chain ID not provided, using the chain ID derived from the rest endpoints: {}",
                    chain_id_from_instances
                );
                chain_id_from_instances
            },
        };

        for (instance, state) in instance_states.into_iter() {
            let state_timestamp = state.timestamp_usecs / 1000000;
            if state.chain_id != chain_id.id() {
                warn!(
                    "Excluding client {} running wrong chain {}, instead of {}",
                    instance.peer_name(),
                    state.chain_id,
                    chain_id.id(),
                );
            } else if state_timestamp + 10 < fetch_timestamp {
                warn!(
                    "Excluding Client {} too stale, {}, while current time when fetching is {} (delta of {}s)",
                    instance.peer_name(),
                    state_timestamp,
                    fetch_timestamp,
                    fetch_timestamp - state_timestamp,
                );
            } else {
                info!(
                    "Client {} is healthy ({}s delay), adding to the list of end points for load testing",
                    instance.peer_name(),
                    fetch_timestamp.saturating_sub(state_timestamp),
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
        load_specific_account(self.account_key(), self.coin_source_is_root, client).await
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

/// In the case that the chain_id is not provided, we can derive it from the instances
/// Error if there there is a mix of chain_ids from the instances
fn get_chain_id_from_instances(instance_states: Vec<(Instance, State)>) -> Result<ChainId> {
    let num_instances = instance_states.len();
    let mut chain_id_counts = std::collections::HashMap::new();
    for (_, state) in instance_states {
        *chain_id_counts.entry(state.chain_id).or_insert(0) += 1;
    }
    let (max_chain_id, num_instances_with_max_chain_id) = chain_id_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .expect("Failed to get the most frequent chain ID from the instances");
    if num_instances_with_max_chain_id < num_instances {
        bail!(
            "The most frequent chain ID {} is only present in {}/{} instances",
            max_chain_id,
            num_instances_with_max_chain_id,
            num_instances
        );
    }
    Ok(ChainId::new(max_chain_id))
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_sdk::types::chain_id::ChainId;

    fn create_dummy_rest_api_state(chain_id: u8) -> State {
        State {
            chain_id,
            epoch: 0,
            version: 0,
            timestamp_usecs: 0,
            oldest_ledger_version: 0,
            oldest_block_height: 0,
            block_height: 0,
            cursor: None,
        }
    }

    #[test]
    fn test_get_chain_id_from_instances_mix() {
        let chain_id_1 = ChainId::new(1);
        let chain_id_2 = ChainId::new(2);
        let chain_id_3 = ChainId::new(3);

        // some dummy instances with a mix of chain_ids
        // expect this to fail
        let instance_states = vec![
            (
                Instance::new(
                    "peer1".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_1.id()),
            ),
            (
                Instance::new(
                    "peer2".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_1.id()),
            ),
            (
                Instance::new(
                    "peer3".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_2.id()),
            ),
            (
                Instance::new(
                    "peer4".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_3.id()),
            ),
        ];

        assert!(get_chain_id_from_instances(instance_states).is_err());
    }

    #[test]
    fn test_get_chain_id_from_instances_ok() {
        let chain_id_3 = ChainId::new(3);

        // some dummy instances with a mix of chain_ids
        // expect this to fail
        let instance_states = vec![
            (
                Instance::new(
                    "peer1".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_3.id()),
            ),
            (
                Instance::new(
                    "peer2".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_3.id()),
            ),
            (
                Instance::new(
                    "peer3".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_3.id()),
            ),
            (
                Instance::new(
                    "peer4".to_string(),
                    Url::parse("http://localhost:8080").unwrap(),
                    None,
                    None,
                ),
                create_dummy_rest_api_state(chain_id_3.id()),
            ),
        ];

        assert!(get_chain_id_from_instances(instance_states).is_ok_and(|x| x == chain_id_3),);
    }
}
