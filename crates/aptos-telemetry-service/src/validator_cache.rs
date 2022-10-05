// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{Peer, PeerRole, PeerSet};
use aptos_infallible::RwLock;
use aptos_rest_client::{error::RestError, Response};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS, chain_id::ChainId, on_chain_config::ValidatorSet, PeerId,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time;
use tracing::{debug, error};
use url::Url;

use crate::types::common::EpochedPeerStore;

#[derive(Clone)]
pub struct PeerSetCacheUpdater {
    validators: Arc<RwLock<EpochedPeerStore>>,
    validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,

    query_addresses: Arc<HashMap<ChainId, String>>,
    update_interval: time::Duration,
}

impl PeerSetCacheUpdater {
    pub fn new(
        validators: Arc<RwLock<EpochedPeerStore>>,
        validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
        trusted_full_node_addresses: HashMap<ChainId, String>,
        update_interval: Duration,
    ) -> Self {
        Self {
            validators,
            validator_fullnodes,
            query_addresses: Arc::new(trusted_full_node_addresses),
            update_interval,
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
            let client = aptos_rest_client::Client::new(Url::parse(url).unwrap());
            let result: Result<Response<ValidatorSet>, RestError> = client
                .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
                .await;
            match result {
                Ok(response) => {
                    let (peer_addrs, state) = response.into_parts();

                    let received_chain_id = ChainId::new(state.chain_id);
                    if received_chain_id != *chain_id {
                        error!("Chain Id mismatch: Received in headers: {}. Provided in configuration: {} for {}", received_chain_id, chain_id, url);
                        continue;
                    }

                    let mut validator_cache = self.validators.write();
                    let mut vfn_cache = self.validator_fullnodes.write();

                    let validator_peers: PeerSet = peer_addrs
                        .clone()
                        .into_iter()
                        .filter_map(|validator_info| -> Option<(PeerId, Peer)> {
                            validator_info
                                .config()
                                .validator_network_addresses()
                                .map(|addresses| {
                                    (
                                        *validator_info.account_address(),
                                        Peer::from_addrs(PeerRole::Validator, addresses),
                                    )
                                })
                                .map_err(|err| {
                                    error!(
                                        "unable to parse validator network address for validator info {}: {}",
                                        validator_info, err
                                    )
                                })
                                .ok()
                        })
                        .collect();

                    debug!(
                        "Validator peers for chain id {} at epoch {}: {:?}",
                        chain_id, state.epoch, validator_peers
                    );

                    if !validator_peers.is_empty() {
                        validator_cache.insert(*chain_id, (state.epoch, validator_peers));
                    }

                    let vfn_peers: PeerSet = peer_addrs
                        .into_iter()
                        .filter_map(|validator_info| -> Option<(PeerId, Peer)> {
                            validator_info
                                .config()
                                .fullnode_network_addresses()
                                .map(|addresses| {
                                    (
                                        *validator_info.account_address(),
                                        Peer::from_addrs(PeerRole::ValidatorFullNode, addresses),
                                    )
                                })
                                .map_err(|err| {
                                    error!(
                                        "unable to parse fullnode network address for validator info {}: {}",
                                        validator_info, err
                                    )
                                })
                                .ok()
                        })
                        .collect();

                    debug!(
                        "Validator fullnode peers for chain id {} at epoch {}: {:?}",
                        chain_id, state.epoch, vfn_peers
                    );

                    if !vfn_peers.is_empty() {
                        vfn_cache.insert(*chain_id, (state.epoch, vfn_peers));
                    }
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

#[cfg(test)]
mod tests {
    use super::PeerSetCacheUpdater;
    use aptos_crypto::{
        bls12381::{PrivateKey, PublicKey},
        test_utils::KeyPair,
        Uniform,
    };
    use aptos_infallible::RwLock;
    use aptos_rest_client::aptos_api_types::*;
    use aptos_types::{
        chain_id::ChainId, network_address::NetworkAddress, on_chain_config::ValidatorSet,
        validator_config::ValidatorConfig, validator_info::ValidatorInfo, PeerId,
    };
    use httpmock::MockServer;
    use rand_core::OsRng;
    use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

    #[tokio::test]
    async fn test_validator_cache_updater_with_invalid_address() {
        let mut rng = OsRng;
        let keypair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
        let validator_info = ValidatorInfo::new(
            PeerId::random(),
            10,
            ValidatorConfig::new(keypair.public_key, vec![0, 0], vec![0, 0], 2),
        );
        let validator_set = ValidatorSet::new(vec![validator_info]);

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("GET")
                .path("/v1/accounts/0000000000000000000000000000000000000000000000000000000000000001/resource/0x1::stake::ValidatorSet");
            then.status(200)
                .body(bcs::to_bytes(&validator_set).unwrap())
                .header(X_APTOS_CHAIN_ID, "25")
                .header(X_APTOS_EPOCH, "10")
                .header(X_APTOS_LEDGER_VERSION, "10")
                .header(X_APTOS_LEDGER_OLDEST_VERSION, "2")
                .header(X_APTOS_BLOCK_HEIGHT, "25")
                .header(X_APTOS_OLDEST_BLOCK_HEIGHT, "10")
                .header(X_APTOS_LEDGER_TIMESTAMP, "10");
        });

        let mut fullnodes = HashMap::new();
        fullnodes.insert(ChainId::new(25), server.base_url());

        let updater = PeerSetCacheUpdater::new(
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new())),
            fullnodes,
            Duration::from_secs(10),
        );

        updater.update().await;

        mock.assert();
        assert!(updater.validators.read().is_empty());
        assert!(updater.validator_fullnodes.read().is_empty());
    }

    #[tokio::test]
    async fn test_validator_cache_updater_with_valid_address() {
        let mut rng = OsRng;
        let keypair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
        let validator_info = ValidatorInfo::new(
            PeerId::random(),
            10,
            ValidatorConfig::new(
                keypair.public_key,
                bcs::to_bytes(&vec![NetworkAddress::from_str("/dns/a5f3d921730874389bb2f66275f163a5-8f14ad5b5e992c1c.elb.ap-southeast-1.amazonaws.com/tcp/6180/noise-ik/0xc5edf62233096df793b554e1013b07c83d01b3cf50c14ac83a0a7e0cfe340426/handshake/0").unwrap()]).unwrap(),
                bcs::to_bytes(&vec![NetworkAddress::from_str("/dns/fullnode0.testnet.aptoslabs.com/tcp/6182/noise-ik/0xea19ab47ed9191865f15d85d751ed0663205c0b2f0f465714b1947c023715973/handshake/0").unwrap()]).unwrap(),
                2,
            ),
        );
        let validator_set = ValidatorSet::new(vec![validator_info.clone()]);

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("GET")
                .path("/v1/accounts/0000000000000000000000000000000000000000000000000000000000000001/resource/0x1::stake::ValidatorSet");
            then.status(200)
            .body(bcs::to_bytes(&validator_set).unwrap())
            .header(X_APTOS_CHAIN_ID, "25")
            .header(X_APTOS_EPOCH, "10")
            .header(X_APTOS_LEDGER_VERSION, "10")
            .header(X_APTOS_LEDGER_OLDEST_VERSION, "2")
            .header(X_APTOS_BLOCK_HEIGHT, "25")
            .header(X_APTOS_OLDEST_BLOCK_HEIGHT, "10")
            .header(X_APTOS_LEDGER_TIMESTAMP, "10");
        });

        let mut fullnodes = HashMap::new();
        fullnodes.insert(ChainId::new(25), server.base_url());

        let updater = PeerSetCacheUpdater::new(
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new())),
            fullnodes,
            Duration::from_secs(10),
        );

        updater.update().await;

        mock.assert();
        assert_eq!(updater.validators.read().len(), 1);
        assert_eq!(updater.validator_fullnodes.read().len(), 1);
        assert_eq!(
            updater
                .validators
                .read()
                .get(&ChainId::new(25))
                .unwrap()
                .1
                .get(validator_info.account_address())
                .unwrap()
                .addresses,
            validator_info
                .config()
                .validator_network_addresses()
                .unwrap()
        );
        assert_eq!(
            updater
                .validator_fullnodes
                .read()
                .get(&ChainId::new(25))
                .unwrap()
                .1
                .get(validator_info.account_address())
                .unwrap()
                .addresses,
            validator_info
                .config()
                .fullnode_network_addresses()
                .unwrap()
        );
    }
}
