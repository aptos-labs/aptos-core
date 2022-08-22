// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{Peer, PeerRole, PeerSet};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_types::chain_id::ChainId;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time;
use url::Url;

use crate::{clients::aptos_api::RestClient, TelemetryServiceConfig};

pub type EpochNum = u64;
pub type ValidatorSetCache = Arc<RwLock<HashMap<ChainId, (EpochNum, PeerSet)>>>;

#[derive(Clone)]
pub struct ValidatorSetCacheUpdater {
    cache: ValidatorSetCache,

    query_addresses: Arc<HashMap<ChainId, String>>,
    update_interval: time::Duration,
}

impl ValidatorSetCacheUpdater {
    pub fn new(cache: ValidatorSetCache, config: &TelemetryServiceConfig) -> Self {
        Self {
            cache,
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
            let validators = client.validator_set_validator_addresses().await;
            match validators {
                Ok((validators, state)) => {
                    let received_chain_id = ChainId::new(state.chain_id);
                    if received_chain_id != *chain_id {
                        error!("Chain Id mismatch: Received in headers: {}. Provided in configuration: {} for {}", received_chain_id, chain_id, url);
                        continue;
                    }

                    let mut store = self.cache.write();

                    let peer_set = validators
                        .iter()
                        .map(|(peer_id, addrs)| {
                            (
                                *peer_id,
                                Peer::from_addrs(PeerRole::Validator, addrs.to_vec()),
                            )
                        })
                        .collect();

                    debug!(
                        "Validator set for chain id {} at epoch {}: {:?}",
                        chain_id, state.epoch, peer_set
                    );

                    store.insert(*chain_id, (state.epoch, peer_set));
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
