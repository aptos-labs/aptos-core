// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    debug, error,
    errors::ValidatorCacheUpdateError,
    metrics::{VALIDATOR_SET_UPDATE_FAILED_COUNT, VALIDATOR_SET_UPDATE_SUCCESS_COUNT},
    types::common::{ChainCommonName, EpochedPeerStore},
};
use velor_config::config::{Peer, PeerRole, PeerSet};
use velor_infallible::RwLock;
use velor_rest_client::Response;
use velor_types::{
    account_config::CORE_CODE_ADDRESS, chain_id::ChainId, on_chain_config::ValidatorSet, PeerId,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time;
use url::Url;

#[derive(Clone)]
pub struct PeerSetCacheUpdater {
    validators: Arc<RwLock<EpochedPeerStore>>,
    validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,

    query_addresses: Arc<HashMap<ChainCommonName, String>>,
    update_interval: time::Duration,
}

impl PeerSetCacheUpdater {
    pub fn new(
        validators: Arc<RwLock<EpochedPeerStore>>,
        validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
        trusted_full_node_addresses: HashMap<ChainCommonName, String>,
        update_interval: Duration,
    ) -> Self {
        Self {
            validators,
            validator_fullnodes,
            query_addresses: Arc::new(trusted_full_node_addresses),
            update_interval,
        }
    }

    pub fn run(self) {
        let mut interval = time::interval(self.update_interval);
        tokio::spawn(async move {
            loop {
                self.update().await;
                interval.tick().await;
            }
        });
    }

    async fn update(&self) {
        for (chain_name, url) in self.query_addresses.iter() {
            match self.update_for_chain(chain_name, url).await {
                Ok(_) => {
                    VALIDATOR_SET_UPDATE_SUCCESS_COUNT
                        .with_label_values(&[&chain_name.to_string()])
                        .inc();
                    debug!(
                        "validator set update successful for chain name {}",
                        chain_name
                    );
                },
                Err(err) => {
                    VALIDATOR_SET_UPDATE_FAILED_COUNT
                        .with_label_values(&[&chain_name.to_string(), &err.to_string()])
                        .inc();
                    error!(
                        "validator set update error for chain name {}: {:?}",
                        chain_name, err
                    );
                },
            }
        }
    }

    async fn update_for_chain(
        &self,
        chain_name: &ChainCommonName,
        url: &str,
    ) -> Result<(), ValidatorCacheUpdateError> {
        let client = velor_rest_client::Client::new(Url::parse(url).map_err(|e| {
            error!("invalid url for chain_id {}: {}", chain_name, e);
            ValidatorCacheUpdateError::InvalidUrl
        })?);
        let response: Response<ValidatorSet> = client
            .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
            .await
            .map_err(ValidatorCacheUpdateError::RestError)?;

        let (peer_addrs, state) = response.into_parts();

        let chain_id = ChainId::new(state.chain_id);

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
                            "unable to parse validator network address for validator info {} for chain name {}: {}",
                            validator_info, chain_name, err
                        )
                    })
                    .ok()
            })
            .collect();

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
                            "unable to parse fullnode network address for validator info {} in chain name {}: {}",
                            validator_info, chain_name, err
                        );
                    })
                    .ok()
            })
            .collect();

        debug!(
            "Validator peers for chain name {} (chain id {}) at epoch {}: {:?}",
            chain_name, chain_id, state.epoch, validator_peers
        );

        let result = if validator_peers.is_empty() && vfn_peers.is_empty() {
            Err(ValidatorCacheUpdateError::BothPeerSetEmpty)
        } else if validator_peers.is_empty() {
            Err(ValidatorCacheUpdateError::ValidatorSetEmpty)
        } else if vfn_peers.is_empty() {
            Err(ValidatorCacheUpdateError::VfnSetEmpty)
        } else {
            Ok(())
        };

        if !validator_peers.is_empty() {
            validator_cache.insert(chain_id, (state.epoch, validator_peers));
        }

        debug!(
            "Validator fullnode peers for chain name {} (chain id {}) at epoch {}: {:?}",
            chain_name, chain_id, state.epoch, vfn_peers
        );

        if !vfn_peers.is_empty() {
            vfn_cache.insert(chain_id, (state.epoch, vfn_peers));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::PeerSetCacheUpdater;
    use velor_crypto::{
        bls12381::{PrivateKey, PublicKey},
        test_utils::KeyPair,
        Uniform,
    };
    use velor_infallible::RwLock;
    use velor_rest_client::velor_api_types::*;
    use velor_types::{
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
                .header(X_VELOR_CHAIN_ID, "25")
                .header(X_VELOR_EPOCH, "10")
                .header(X_VELOR_LEDGER_VERSION, "10")
                .header(X_VELOR_LEDGER_OLDEST_VERSION, "2")
                .header(X_VELOR_BLOCK_HEIGHT, "25")
                .header(X_VELOR_OLDEST_BLOCK_HEIGHT, "10")
                .header(X_VELOR_LEDGER_TIMESTAMP, "10");
        });

        let mut fullnodes = HashMap::new();
        fullnodes.insert("testing".into(), server.base_url());

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
                bcs::to_bytes(&vec![NetworkAddress::from_str("/dns/fullnode0.testnet.velorlabs.com/tcp/6182/noise-ik/0xea19ab47ed9191865f15d85d751ed0663205c0b2f0f465714b1947c023715973/handshake/0").unwrap()]).unwrap(),
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
            .header(X_VELOR_CHAIN_ID, "25")
            .header(X_VELOR_EPOCH, "10")
            .header(X_VELOR_LEDGER_VERSION, "10")
            .header(X_VELOR_LEDGER_OLDEST_VERSION, "2")
            .header(X_VELOR_BLOCK_HEIGHT, "25")
            .header(X_VELOR_OLDEST_BLOCK_HEIGHT, "10")
            .header(X_VELOR_LEDGER_TIMESTAMP, "10");
        });

        let mut fullnodes = HashMap::new();
        fullnodes.insert("testing".into(), server.base_url());

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
