// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        default_if_zero, default_if_zero_u8, env_or_default, invariant, must_be_set,
        persistable_config::PersistableConfig, utils::RootPath, ApiConfig, BaseConfig,
        ConsensusConfig, Error, ExecutionConfig, IndexerConfig, IndexerGrpcConfig,
        InspectionServiceConfig, LoggerConfig, MempoolConfig, NetworkConfig,
        PeerMonitoringServiceConfig, SafetyRulesTestConfig, StateSyncConfig, StorageConfig,
        DEFAULT_BATCH_SIZE, DEFAULT_FETCH_TASKS, DEFAULT_PROCESSOR_TASKS,
    },
    network_id::NetworkId,
};
use aptos_crypto::x25519;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress as PeerId;
use rand::{prelude::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// The node configuration defines the configuration for a single Aptos
/// node (i.e., validator or fullnode). It is composed of module
/// configurations for each of the modules that the node uses (e.g.,
/// the API, indexer, mempool, state sync, etc.).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeConfig {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub consensus: ConsensusConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub failpoints: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub full_node_networks: Vec<NetworkConfig>,
    #[serde(default)]
    pub indexer: IndexerConfig,
    #[serde(default)]
    pub indexer_grpc: IndexerGrpcConfig,
    #[serde(default)]
    pub inspection_service: InspectionServiceConfig,
    #[serde(default)]
    pub logger: LoggerConfig,
    #[serde(default)]
    pub mempool: MempoolConfig,
    #[serde(default)]
    pub peer_monitoring_service: PeerMonitoringServiceConfig,
    #[serde(default)]
    pub state_sync: StateSyncConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub validator_network: Option<NetworkConfig>,
}

impl NodeConfig {
    /// Returns the data directory for this config
    pub fn get_data_dir(&self) -> &Path {
        &self.base.data_dir
    }

    /// Returns the working directory for this config (if set),
    /// otherwise, returns the data directory.
    pub fn get_working_dir(&self) -> &Path {
        match &self.base.working_dir {
            Some(working_dir) => working_dir,
            None => &self.base.data_dir,
        }
    }

    /// Sets the data directory for this config
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        // Set the base directory
        self.base.data_dir = data_dir.clone();

        // Set the data directory for each sub-module
        self.consensus.set_data_dir(data_dir.clone());
        self.storage.set_data_dir(data_dir);
    }

    /// Load the node config from the given path and perform several processing
    /// steps. Note: paths used in the node config are either absolute or
    /// relative to the config location.
    pub fn load_from_path<P: AsRef<Path>>(input_path: P) -> Result<Self, Error> {
        // Load the node config from disk
        let mut node_config = Self::load_config(&input_path)?;

        // Load the execution config
        let input_dir = RootPath::new(input_path);
        node_config.execution.load(&input_dir)?;

        // Validate the node config
        node_config.validate_config()?;

        // Update the data directory
        node_config.set_data_dir(node_config.get_data_dir().to_path_buf());
        Ok(node_config)
    }

    /// Validate the node config for consistency and correctness
    pub fn validate_config(&mut self) -> Result<(), Error> {
        self.validate_indexer_configs()?;
        self.validate_indexer_grpc_configs()?;
        self.validate_network_configs()
    }

    /// Returns the peer ID of the node based on the role
    pub fn get_peer_id(&self) -> Option<PeerId> {
        self.get_primary_network_config()
            .map(NetworkConfig::peer_id)
    }

    /// Returns the identity key of the node based on the role
    pub fn get_identity_key(&self) -> Option<x25519::PrivateKey> {
        self.get_primary_network_config()
            .map(NetworkConfig::identity_key)
    }

    /// Returns the primary network config of the node. If the node
    /// is a validator, the validator network config is returned.
    /// Otherwise, the public fullnode network config is returned.
    fn get_primary_network_config(&self) -> Option<&NetworkConfig> {
        if self.base.role.is_validator() {
            self.validator_network.as_ref()
        } else {
            self.full_node_networks
                .iter()
                .find(|config| config.network_id == NetworkId::Public)
        }
    }

    /// Validate `IndexerConfig`, ensuring that it's set up correctly
    /// Additionally, handles any strange missing default cases
    fn validate_indexer_configs(&mut self) -> Result<(), Error> {
        if !self.indexer.enabled {
            return Ok(());
        }

        self.indexer.postgres_uri = env_or_default(
            "INDEXER_DATABASE_URL",
            self.indexer.postgres_uri.clone(),
            must_be_set("postgres_uri", "INDEXER_DATABASE_URL"),
        );

        self.indexer.processor = env_or_default(
            "PROCESSOR_NAME",
            self.indexer
                .processor
                .clone()
                .or_else(|| Some("default_processor".to_string())),
            None,
        );

        self.indexer.starting_version = match std::env::var("STARTING_VERSION").ok() {
            None => self.indexer.starting_version,
            Some(s) => match s.parse::<u64>() {
                Ok(version) => Some(version),
                Err(_) => {
                    // Doing this instead of failing. This will allow a processor to have STARTING_VERSION: undefined when deploying
                    aptos_logger::warn!(
                        "Invalid STARTING_VERSION: {}, using {:?} instead",
                        s,
                        self.indexer.starting_version
                    );
                    self.indexer.starting_version
                },
            },
        };

        self.indexer.skip_migrations = self.indexer.skip_migrations.or(Some(false));
        self.indexer.check_chain_id = self.indexer.check_chain_id.or(Some(true));
        self.indexer.batch_size = default_if_zero(
            self.indexer.batch_size.map(|v| v as u64),
            DEFAULT_BATCH_SIZE as u64,
        )
        .map(|v| v as u16);
        self.indexer.fetch_tasks = default_if_zero(
            self.indexer.fetch_tasks.map(|v| v as u64),
            DEFAULT_FETCH_TASKS as u64,
        )
        .map(|v| v as u8);
        self.indexer.processor_tasks =
            default_if_zero_u8(self.indexer.processor_tasks, DEFAULT_PROCESSOR_TASKS);
        self.indexer.emit_every = self.indexer.emit_every.or(Some(0));
        self.indexer.gap_lookback_versions = env_or_default(
            "GAP_LOOKBACK_VERSIONS",
            self.indexer.gap_lookback_versions.or(Some(1_500_000)),
            None,
        );

        Ok(())
    }

    /// Validate `IndexerGrpcConfig`, ensuring that it's set up correctly
    /// Additionally, handles any strange missing default cases
    fn validate_indexer_grpc_configs(&mut self) -> Result<(), Error> {
        if !self.indexer_grpc.enabled {
            return Ok(());
        }

        self.indexer_grpc.address = self
            .indexer_grpc
            .address
            .clone()
            .or_else(|| Some("0.0.0.0:50051".to_string()));

        self.indexer_grpc.processor_task_count =
            self.indexer_grpc.processor_task_count.or(Some(20));

        self.indexer_grpc.processor_batch_size =
            self.indexer_grpc.processor_batch_size.or(Some(1000));

        self.indexer_grpc.output_batch_size = self.indexer_grpc.output_batch_size.or(Some(100));

        Ok(())
    }

    /// Checks `NetworkConfig` setups so that they exist on proper networks
    /// Additionally, handles any strange missing default cases
    fn validate_network_configs(&mut self) -> Result<(), Error> {
        if self.base.role.is_validator() {
            invariant(
                self.validator_network.is_some(),
                "Missing a validator network config for a validator node".into(),
            )?;
        } else {
            invariant(
                self.validator_network.is_none(),
                "Provided a validator network config for a full_node node".into(),
            )?;
        }

        if let Some(network) = &mut self.validator_network {
            network.load_validator_network()?;
            network.mutual_authentication = true; // This should always be the default for validators
        }
        for network in &mut self.full_node_networks {
            network.load_fullnode_network()?;
        }

        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&mut self, output_path: P) -> Result<(), Error> {
        let output_dir = RootPath::new(&output_path);
        self.execution.save(&output_dir)?;
        // This must be last as calling save on subconfigs may change their fields
        self.save_config(&output_path)?;
        Ok(())
    }

    /// Randomizes the various ports of the node config
    pub fn randomize_ports(&mut self) {
        // Randomize the ports for the services
        self.api.randomize_ports();
        self.inspection_service.randomize_ports();
        self.storage.randomize_ports();
        self.logger.disable_console();

        // Randomize the ports for the networks
        if let Some(network) = self.validator_network.as_mut() {
            network.listen_address = crate::utils::get_available_port_in_multiaddr(true);
        }
        for network in self.full_node_networks.iter_mut() {
            network.listen_address = crate::utils::get_available_port_in_multiaddr(true);
        }
    }

    /// Generates a random config for testing purposes
    pub fn generate_random_config() -> Self {
        let mut rng = StdRng::from_seed([0u8; 32]);
        Self::generate_random_config_with_template(&NodeConfig::default(), &mut rng)
    }

    /// Generates a random config using the given template and rng
    pub fn generate_random_config_with_template(template: &Self, rng: &mut StdRng) -> Self {
        // Create the node and test configs
        let mut node_config = template.clone();

        // Modify the configs based on the role type
        if node_config.base.role.is_validator() {
            let peer_id = PeerId::random();

            if node_config.validator_network.is_none() {
                let network_config = NetworkConfig::network_with_id(NetworkId::Validator);
                node_config.validator_network = Some(network_config);
            }

            let validator_network = node_config.validator_network.as_mut().unwrap();
            validator_network.random_with_peer_id(rng, Some(peer_id));

            let mut safety_rules_test_config = SafetyRulesTestConfig::new(peer_id);
            safety_rules_test_config.random_consensus_key(rng);
            node_config.consensus.safety_rules.test = Some(safety_rules_test_config);
        } else {
            node_config.validator_network = None;
            if node_config.full_node_networks.is_empty() {
                let network_config = NetworkConfig::network_with_id(NetworkId::Public);
                node_config.full_node_networks.push(network_config);
            }
            for network in &mut node_config.full_node_networks {
                network.random(rng);
            }
        }

        // Create and use a temp directory for the data directory
        let temp_dir = TempPath::new();
        temp_dir.create_as_dir().unwrap_or_else(|error| {
            panic!(
                "Failed to create a temporary directory at {}! Error: {:?}",
                temp_dir.path().display(),
                error
            )
        });
        node_config.set_data_dir(temp_dir.path().to_path_buf());

        node_config
    }

    /// Returns the default config for a public full node
    pub fn get_default_pfn_config() -> Self {
        let contents = include_str!("test_data/public_full_node.yaml");
        parse_serialized_node_config(contents, "default_for_public_full_node")
    }

    /// Returns the default config for a validator
    pub fn get_default_validator_config() -> Self {
        let contents = include_str!("test_data/validator.yaml");
        parse_serialized_node_config(contents, "default_for_validator")
    }

    /// Returns the default config for a validator full node
    pub fn get_default_vfn_config() -> Self {
        let contents = include_str!("test_data/validator_full_node.yaml");
        parse_serialized_node_config(contents, "default_for_validator_full_node")
    }
}

/// Parses the given serialized config into a node config
fn parse_serialized_node_config(serialized_config: &str, caller: &'static str) -> NodeConfig {
    // Parse the node config
    let mut node_config =
        NodeConfig::parse_serialized_config(serialized_config).unwrap_or_else(|error| {
            panic!(
                "Failed to parse node config! Caller: {}, Error: {}",
                caller, error
            )
        });

    // Validate the config
    node_config.validate_config().unwrap_or_else(|error| {
        panic!(
            "Config validation failed! Caller: {}, Error: {}",
            caller, error
        )
    });

    node_config
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        config::{NodeConfig, SafetyRulesConfig},
        network_id::NetworkId,
    };

    #[test]
    fn verify_config_defaults() {
        // Verify the node config defaults
        NodeConfig::get_default_pfn_config();
        NodeConfig::get_default_validator_config();
        NodeConfig::get_default_vfn_config();

        // Verify the safety rules config default
        SafetyRulesConfig::get_default_config();
    }

    #[test]
    fn validate_invalid_network_id() {
        let mut config = NodeConfig::get_default_pfn_config();
        let network = config.full_node_networks.iter_mut().next().unwrap();
        network.network_id = NetworkId::Validator;
        assert!(matches!(
            config.validate_network_configs(),
            Err(Error::InvariantViolation(_))
        ));
    }
}
