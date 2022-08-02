use std::{sync::Arc, collections::HashMap};
use anyhow::{anyhow,Result};
use aptos_config::config::{Peer, PeerRole};
use aptos_rest_client::state::State;
use aptos_types::{network_address::NetworkAddress, PeerId, chain_id::ChainId};
use aptos_infallible::RwLock;

use crate::{rest_client::RestClient, types::validator_set::{ValidatorConfig, ValidatorInfo}};

#[derive(Clone)]
pub struct ValidatorCache {
    client: RestClient,
    pub(crate) validator_store: Arc<RwLock<HashMap<(ChainId,PeerId), Peer>>>,
}

impl ValidatorCache {
    pub fn new(rest_client: RestClient) -> ValidatorCache {
        ValidatorCache {
            client: rest_client,
            validator_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update(&self) {    
        let validators = self.validator_set_validator_addresses().await;
        match validators {
            Ok((validators, state)) => {
                let chain_id = state.chain_id;
                let chain_id = ChainId::new(chain_id);
                let mut store = self.validator_store.write();
                store.clear();
                for (peer_id, network_addresses) in validators {
                    let peer = Peer::from_addrs(PeerRole::Validator, network_addresses);
                    store.insert((chain_id, peer_id), peer);
                }
                println!("Updated store {:#?}", store);
            },
            Err(err) => {
                println!("Unable to update validators: {}", err)
            }
        }
    }
    
    async fn validator_set_validator_addresses(
        &self,
    ) -> Result<(Vec<(PeerId, Vec<NetworkAddress>)>, State)> {
        self.validator_set_addresses(|info| {
            Self::validator_addresses(info.config())
        })
        .await
    }
    
    fn validator_addresses(
        config: &ValidatorConfig,
    ) -> Result<Vec<NetworkAddress>> {
        config
            .validator_network_addresses()
            .map_err(|e| anyhow!("unable to parse network address {}", e.to_string()))
    }
    
    async fn validator_set_addresses<F: Fn(ValidatorInfo) -> Result<Vec<NetworkAddress>>> (
        &self,
        address_accessor: F,
    ) -> Result<(Vec<(PeerId, Vec<NetworkAddress>)>, State)> {
        let (set, state) = self.client.validator_set().await?;
        println!("validatorset: {:?}", set);
        let mut decoded_set = Vec::new();
        for info in set {
            let peer_id = *info.account_address();
            let addrs = address_accessor(info)?;
            decoded_set.push((peer_id, addrs));
        }
    
        Ok((decoded_set, state))
    }
}

