// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{Peer, PeerRole, PeerSet};
use aptos_infallible::RwLock;
use aptos_types::chain_id::ChainId;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time;
use tracing::{debug, error};
use url::Url;

use crate::{clients::aptos_api::RestClient, TelemetryServiceConfig};

pub type EpochNum = u64;
pub type PeerSetCache = Arc<RwLock<HashMap<ChainId, (EpochNum, PeerSet)>>>;

#[derive(Clone)]
pub struct PeerSetCacheUpdater {
    validators: PeerSetCache,
    validator_fullnodes: PeerSetCache,

    query_addresses: Arc<HashMap<ChainId, String>>,
    update_interval: time::Duration,
}

impl PeerSetCacheUpdater {
    pub fn new(
        validators: PeerSetCache,
        validator_fullnodes: PeerSetCache,
        config: &TelemetryServiceConfig,
    ) -> Self {
        Self {
            validators,
            validator_fullnodes,
            query_addresses: Arc::new(config.trusted_full_node_addresses.clone()),
            update_interval: Duration::from_secs(config.update_interval),
        }
    }

    pub fn run(&self) {
        let mut interval = time::interval(self.update_interval);
        let cloned_self = self.clone();
        tokio::spawn(async move {
            loop {
                cloned_self.clone().update().await;
                interval.tick().await;
            }
        });
    }

    pub async fn update(&self) {
        for (chain_id, url) in self.query_addresses.iter() {
            let client = RestClient::new(Url::parse(url).unwrap());
            let result = client.validator_set_all_addresses().await;
            match result {
                Ok((peer_addrs, state)) => {
                    let received_chain_id = ChainId::new(state.chain_id);
                    if received_chain_id != *chain_id {
                        error!("Chain Id mismatch: Received in headers: {}. Provided in configuration: {} for {}", received_chain_id, chain_id, url);
                        continue;
                    }

                    let mut validator_cache = self.validators.write();
                    let mut vfn_cache = self.validator_fullnodes.write();

                    let validator_peers = peer_addrs
                        .iter()
                        .map(|(peer_id, validator_addrs, _)| {
                            (
                                *peer_id,
                                Peer::from_addrs(PeerRole::Validator, validator_addrs.to_vec()),
                            )
                        })
                        .collect();

                    debug!(
                        "Validator peers for chain id {} at epoch {}: {:?}",
                        chain_id, state.epoch, validator_peers
                    );

                    validator_cache.insert(*chain_id, (state.epoch, validator_peers));

                    let vfn_peers = peer_addrs
                        .iter()
                        .map(|(peer_id, _, network_address)| {
                            (
                                *peer_id,
                                Peer::from_addrs(
                                    PeerRole::ValidatorFullNode,
                                    network_address.to_vec(),
                                ),
                            )
                        })
                        .collect();

                    debug!(
                        "Validator fullnode peers for chain id {} at epoch {}: {:?}",
                        chain_id, state.epoch, vfn_peers
                    );

                    vfn_cache.insert(*chain_id, (state.epoch, vfn_peers));
                }
                Err(err) => {
                    error!(
                        "Fetching validator set failed for Chain Id {}. Err: {}",
                        chain_id, err
                    )
                }
            }
        }
    }
}
