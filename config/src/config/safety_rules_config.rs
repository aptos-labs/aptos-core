// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{IdentityBlob, LoggerConfig, SecureBackend, WaypointConfig},
    keys::ConfigKey,
};
use aptos_crypto::{bls12381, Uniform};
use aptos_types::{network_address::NetworkAddress, waypoint::Waypoint, PeerId};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SafetyRulesConfig {
    pub backend: SecureBackend,
    pub logger: LoggerConfig,
    pub service: SafetyRulesService,
    pub test: Option<SafetyRulesTestConfig>,
    // Read/Write/Connect networking operation timeout in milliseconds.
    pub network_timeout_ms: u64,
    pub enable_cached_safety_data: bool,
    pub initial_safety_rules_config: InitialSafetyRulesConfig,
}

impl Default for SafetyRulesConfig {
    fn default() -> Self {
        Self {
            backend: SecureBackend::InMemoryStorage,
            logger: LoggerConfig::default(),
            service: SafetyRulesService::Local,
            test: None,
            // Default value of 30 seconds for a timeout
            network_timeout_ms: 30_000,
            enable_cached_safety_data: true,
            initial_safety_rules_config: InitialSafetyRulesConfig::None,
        }
    }
}

impl SafetyRulesConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        if let SecureBackend::OnDiskStorage(backend) = &mut self.backend {
            backend.set_data_dir(data_dir);
        } else if let SecureBackend::RocksDbStorage(backend) = &mut self.backend {
            backend.set_data_dir(data_dir);
        }
    }
}

// TODO: Find a cleaner way so WaypointConfig isn't duplicated
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InitialSafetyRulesConfig {
    FromFile {
        identity_blob_path: PathBuf,
        waypoint: WaypointConfig,
    },
    None,
}

impl InitialSafetyRulesConfig {
    pub fn from_file(identity_blob_path: PathBuf, waypoint: WaypointConfig) -> Self {
        Self::FromFile {
            identity_blob_path,
            waypoint,
        }
    }

    pub fn waypoint(&self) -> Waypoint {
        match self {
            InitialSafetyRulesConfig::FromFile { waypoint, .. } => waypoint.waypoint(),
            InitialSafetyRulesConfig::None => panic!("Must have a waypoint"),
        }
    }

    pub fn identity_blob(&self) -> IdentityBlob {
        match self {
            InitialSafetyRulesConfig::FromFile {
                identity_blob_path, ..
            } => IdentityBlob::from_file(identity_blob_path).unwrap(),
            InitialSafetyRulesConfig::None => panic!("Must have an identity blob"),
        }
    }
}

/// Defines how safety rules should be executed
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SafetyRulesService {
    /// This runs safety rules in the same thread as event processor
    Local,
    /// This is the production, separate service approach
    Process(RemoteService),
    /// This runs safety rules in the same thread as event processor but data is passed through the
    /// light weight RPC (serializer)
    Serializer,
    /// This creates a separate thread to run safety rules, it is similar to a fork / exec style
    Thread,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteService {
    pub server_address: NetworkAddress,
}

impl RemoteService {
    pub fn server_address(&self) -> SocketAddr {
        self.server_address
            .to_socket_addrs()
            .expect("server_address invalid")
            .next()
            .expect("server_address invalid")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SafetyRulesTestConfig {
    pub author: PeerId,
    pub consensus_key: Option<ConfigKey<bls12381::PrivateKey>>,
    pub waypoint: Option<Waypoint>,
}

impl SafetyRulesTestConfig {
    pub fn new(author: PeerId) -> Self {
        Self {
            author,
            consensus_key: None,
            waypoint: None,
        }
    }

    pub fn consensus_key(&mut self, key: bls12381::PrivateKey) {
        self.consensus_key = Some(ConfigKey::new(key));
    }

    pub fn random_consensus_key(&mut self, rng: &mut StdRng) {
        let privkey = bls12381::PrivateKey::generate(rng);
        self.consensus_key = Some(ConfigKey::<bls12381::PrivateKey>::new(privkey));
    }
}
