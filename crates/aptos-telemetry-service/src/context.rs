// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::{convert::Infallible, sync::Arc};

use crate::clients::big_query::TableWriteClient;
use crate::types::common::EpochedPeerStore;
use crate::{
    clients::humio::IngestClient as HumioClient,
    clients::victoria_metrics_api::Client as MetricsClient,
};
use aptos_crypto::{noise, x25519};
use aptos_infallible::RwLock;
use aptos_types::chain_id::ChainId;
use aptos_types::PeerId;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, TokenData, Validation};
use serde::de::DeserializeOwned;
use serde::Serialize;
use warp::Filter;

#[derive(Clone, Default)]
pub struct PeerStoreTuple {
    validators: Arc<RwLock<EpochedPeerStore>>,
    validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
    public_fullnodes: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
}

impl PeerStoreTuple {
    pub fn new(
        validators: Arc<RwLock<EpochedPeerStore>>,
        validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
        public_fullnodes: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
    ) -> Self {
        Self {
            validators,
            validator_fullnodes,
            public_fullnodes,
        }
    }

    pub fn validators(&self) -> &Arc<RwLock<EpochedPeerStore>> {
        &self.validators
    }

    pub fn validator_fullnodes(&self) -> &Arc<RwLock<EpochedPeerStore>> {
        &self.validator_fullnodes
    }

    pub fn public_fullnodes(&self) -> &HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>> {
        &self.public_fullnodes
    }
}

#[derive(Clone)]
pub struct ClientTuple {
    bigquery_client: TableWriteClient,
    victoria_metrics_client: MetricsClient,
    humio_client: HumioClient,
}

impl ClientTuple {
    pub(crate) fn new(
        bigquery_client: TableWriteClient,
        victoria_metrics_client: MetricsClient,
        humio_client: HumioClient,
    ) -> ClientTuple {
        Self {
            bigquery_client,
            victoria_metrics_client,
            humio_client,
        }
    }
}

#[derive(Clone)]
pub struct JsonWebTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JsonWebTokenService {
    pub fn from_base64_secret(secret: &str) -> Self {
        let encoding_key = jsonwebtoken::EncodingKey::from_base64_secret(secret)
            .expect("jsonwebtoken key should be in base64 format.");
        let decoding_key = jsonwebtoken::DecodingKey::from_base64_secret(secret)
            .expect("jsonwebtoken key should be in base64 format.");
        Self {
            encoding_key,
            decoding_key,
        }
    }

    pub fn encode<T: Serialize>(&self, claims: T) -> Result<String, jsonwebtoken::errors::Error> {
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512);
        jsonwebtoken::encode(&header, &claims, &self.encoding_key)
    }

    pub fn decode<T: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<TokenData<T>, jsonwebtoken::errors::Error> {
        jsonwebtoken::decode::<T>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS512),
        )
    }
}

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    peers: PeerStoreTuple,
    clients: Option<ClientTuple>,
    chain_set: HashSet<ChainId>,
    jwt_service: JsonWebTokenService,
    log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
}

impl Context {
    pub fn new(
        private_key: x25519::PrivateKey,
        peers: PeerStoreTuple,
        clients: Option<ClientTuple>,
        chain_set: HashSet<ChainId>,
        jwt_service: JsonWebTokenService,
        log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    ) -> Self {
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            peers,
            clients,
            chain_set,
            jwt_service,
            log_env_map,
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        self.noise_config.clone()
    }

    pub fn peers(&self) -> &PeerStoreTuple {
        &self.peers
    }

    pub fn jwt_service(&self) -> &JsonWebTokenService {
        &self.jwt_service
    }

    pub fn metrics_client(&self) -> &MetricsClient {
        &self.clients.as_ref().unwrap().victoria_metrics_client
    }

    pub fn humio_client(&self) -> &HumioClient {
        &self.clients.as_ref().unwrap().humio_client
    }

    pub(crate) fn bigquery_client(&self) -> Option<&TableWriteClient> {
        self.clients.as_ref().map(|c| &c.bigquery_client)
    }

    pub fn chain_set(&self) -> &HashSet<ChainId> {
        &self.chain_set
    }

    #[cfg(test)]
    pub fn chain_set_mut(&mut self) -> &mut HashSet<ChainId> {
        &mut self.chain_set
    }

    pub fn log_env_map(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.log_env_map
    }

    #[cfg(test)]
    pub fn log_env_map_mut(&mut self) -> &mut HashMap<ChainId, HashMap<PeerId, String>> {
        &mut self.log_env_map
    }
}
