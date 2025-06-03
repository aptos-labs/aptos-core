// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{DagConsensusConfig, IndexerTableInfoConfig};
use crate::{
    config::{
        consensus_observer_config::ConsensusObserverConfig, dkg_config::DKGConfig,
        internal_indexer_db_config::InternalIndexerDBConfig,
        jwk_consensus_config::JWKConsensusConfig, netbench_config::NetbenchConfig,
        node_config_loader::NodeConfigLoader, node_startup_config::NodeStartupConfig,
        persistable_config::PersistableConfig, transaction_filter_config::TransactionFilterConfig,
        utils::RootPath, AdminServiceConfig, ApiConfig, BaseConfig, ConsensusConfig, Error,
        ExecutionConfig, IndexerConfig, IndexerGrpcConfig, InspectionServiceConfig, LoggerConfig,
        MempoolConfig, NetworkConfig, PeerMonitoringServiceConfig, SafetyRulesTestConfig,
        StateSyncConfig, StorageConfig,
    },
    network_id::NetworkId,
};
use aptos_crypto::x25519;
use aptos_logger::info;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress as PeerId;
use rand::{prelude::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
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
    pub admin_service: AdminServiceConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub consensus: ConsensusConfig,
    #[serde(default)]
    pub consensus_observer: ConsensusObserverConfig,
    #[serde(default)]
    pub dag_consensus: DagConsensusConfig,
    #[serde(default)]
    pub dkg: DKGConfig,
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
    pub indexer_table_info: IndexerTableInfoConfig,
    #[serde(default)]
    pub inspection_service: InspectionServiceConfig,
    #[serde(default)]
    pub jwk_consensus: JWKConsensusConfig,
    #[serde(default)]
    pub logger: LoggerConfig,
    #[serde(default)]
    pub mempool: MempoolConfig,
    #[serde(default)]
    pub netbench: Option<NetbenchConfig>,
    #[serde(default)]
    pub node_startup: NodeStartupConfig,
    #[serde(default)]
    pub peer_monitoring_service: PeerMonitoringServiceConfig,
    /// In a randomness stall, set this to be on-chain `RandomnessConfigSeqNum` + 1.
    /// Once enough nodes restarted with the new value, the chain should unblock with randomness disabled.
    #[serde(default)]
    pub randomness_override_seq_num: u64,
    #[serde(default)]
    pub state_sync: StateSyncConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub transaction_filter: TransactionFilterConfig,
    #[serde(default)]
    pub validator_network: Option<NetworkConfig>,
    #[serde(default)]
    pub indexer_db_config: InternalIndexerDBConfig,
}

impl NodeConfig {
    /// Logs the node config using INFO level logging. This is useful for
    /// working around the length restrictions in the logger.
    pub fn log_all_configs(&self) {
        // Parse the node config as serde JSON
        let config_value =
            serde_json::to_value(self).expect("Failed to serialize the node config!");
        let config_map = config_value
            .as_object()
            .expect("Failed to get the config map!");

        // Log each config entry
        for (config_name, config_value) in config_map {
            let config_string =
                serde_json::to_string(config_value).expect("Failed to parse the config value!");
            info!("Using {} config: {}", config_name, config_string);
        }
    }

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
        self.base.data_dir.clone_from(&data_dir);

        // Set the data directory for each sub-module
        self.consensus.set_data_dir(data_dir.clone());
        self.storage.set_data_dir(data_dir);
    }

    /// Load the node config from the given path and perform several processing
    /// steps. Note: paths used in the node config are either absolute or
    /// relative to the config location.
    pub fn load_from_path<P: AsRef<Path>>(input_path: P) -> Result<Self, Error> {
        let node_config_loader = NodeConfigLoader::new(input_path);
        node_config_loader.load_and_sanitize_config()
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

    /// Save the node config to the given path
    pub fn save_to_path<P: AsRef<Path>>(&mut self, output_path: P) -> Result<(), Error> {
        // Save the execution config to disk.
        let output_dir = RootPath::new(&output_path);
        self.execution.save_to_path(&output_dir)?;

        // Write the node config to disk. Note: this must be called last
        // as calling save_to_path() on subconfigs may change fields.
        self.save_config(&output_path)?;

        Ok(())
    }

    /// Randomizes the various ports of the node config
    pub fn randomize_ports(&mut self) {
        // Randomize the ports for the services
        self.admin_service.randomize_ports();
        self.api.randomize_ports();
        self.inspection_service.randomize_ports();
        self.storage.randomize_ports();
        self.logger.disable_tokio_console();

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
    NodeConfig::parse_serialized_config(serialized_config).unwrap_or_else(|error| {
        panic!(
            "Failed to parse node config! Caller: {}, Error: {}",
            caller, error
        )
    })
}

/// Merges node_config with a config config override
pub fn merge_node_config(
    node_config: NodeConfig,
    override_node_config: serde_yaml::Value,
) -> Result<NodeConfig, Error> {
    serde_merge::tmerge::<NodeConfig, serde_yaml::Value, NodeConfig>(
        node_config,
        override_node_config,
    )
    .map_err(|e| {
        Error::Unexpected(format!(
            "Unable to merge default config with override. Error: {}",
            e
        ))
    })
}

#[cfg(test)]
mod test {
    use crate::config::{merge_node_config, Error, NodeConfig, SafetyRulesConfig};

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
    fn verify_merge_node_config() {
        let node_config = NodeConfig::get_default_pfn_config();
        let override_node_config = serde_yaml::from_str(
            r#"
            api:
                enabled: false
            "#,
        )
        .unwrap();
        let merged_node_config = merge_node_config(node_config, override_node_config).unwrap();
        assert!(!merged_node_config.api.enabled);
    }

    #[test]
    fn verify_bad_merge_node_config() {
        let node_config = NodeConfig::get_default_pfn_config();
        let override_node_config = serde_yaml::from_str(
            r#"
            blablafakenodeconfigkeyblabla:
                enabled: false
            "#,
        )
        .unwrap();
        let merged_node_config = merge_node_config(node_config, override_node_config);
        assert!(matches!(merged_node_config, Err(Error::Unexpected(_))));
    }
}
