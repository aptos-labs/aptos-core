// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    clients::{
        big_query::TableWriteClient, humio::IngestClient as HumioClient,
        victoria_metrics_api::Client as MetricsClient,
    },
    types::common::EpochedPeerStore,
};
use aptos_crypto::{noise, x25519};
use aptos_infallible::RwLock;
use aptos_types::{chain_id::ChainId, PeerId};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, TokenData, Validation};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};
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
    bigquery_client: Option<TableWriteClient>,
    victoria_metrics_clients: Option<BTreeMap<String, MetricsClient>>,
    humio_client: Option<HumioClient>,
}

impl ClientTuple {
    pub(crate) fn new(
        bigquery_client: Option<TableWriteClient>,
        victoria_metrics_clients: Option<BTreeMap<String, MetricsClient>>,
        humio_client: Option<HumioClient>,
    ) -> ClientTuple {
        Self {
            bigquery_client,
            victoria_metrics_clients,
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
    clients: ClientTuple,
    jwt_service: JsonWebTokenService,
    log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,
}

impl Context {
    pub fn new(
        private_key: x25519::PrivateKey,
        peers: PeerStoreTuple,
        clients: ClientTuple,
        jwt_service: JsonWebTokenService,
        log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
        peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,
    ) -> Self {
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            peers,
            clients,
            jwt_service,
            log_env_map,
            peer_identities,
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

    pub fn metrics_client(&self) -> &BTreeMap<String, MetricsClient> {
        self.clients.victoria_metrics_clients.as_ref().unwrap()
    }

    #[cfg(test)]
    pub fn metrics_client_mut(&mut self) -> &mut BTreeMap<String, MetricsClient> {
        self.clients.victoria_metrics_clients.as_mut().unwrap()
    }

    pub fn humio_client(&self) -> &HumioClient {
        self.clients.humio_client.as_ref().unwrap()
    }

    pub(crate) fn bigquery_client(&self) -> Option<&TableWriteClient> {
        self.clients.bigquery_client.as_ref()
    }

    pub(crate) fn peer_identities(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.peer_identities
    }

    pub fn chain_set(&self) -> HashSet<ChainId> {
        self.peers.validators.read().keys().cloned().collect()
    }

    pub fn log_env_map(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.log_env_map
    }

    #[cfg(test)]
    pub fn log_env_map_mut(&mut self) -> &mut HashMap<ChainId, HashMap<PeerId, String>> {
        &mut self.log_env_map
    }
}
