// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        netbench::NetbenchConfig, node_config_loader::NodeConfigLoader,
        persistable_config::PersistableConfig, utils::RootPath, ApiConfig, BaseConfig,
        ConsensusConfig, Error, ExecutionConfig, IndexerConfig, IndexerGrpcConfig,
        InspectionServiceConfig, LoggerConfig, MempoolConfig, NetworkConfig,
        PeerMonitoringServiceConfig, SafetyRulesTestConfig, StateSyncConfig, StorageConfig,
    },
    network_id::NetworkId,
};
use anyhow::bail;
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
    #[serde(default)]
    pub netbench: Option<NetbenchConfig>,
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

/// Diff a config yaml with a base config yaml. Returns None if there is no diff.
pub fn diff_override_config_yaml(
    override_config: serde_yaml::Value,
    base_config: serde_yaml::Value,
) -> anyhow::Result<Option<serde_yaml::Value>> {
    match override_config.clone() {
        serde_yaml::Value::Mapping(override_mapping) => match base_config {
            serde_yaml::Value::Mapping(base_mapping) => {
                let mut overrides = serde_yaml::Mapping::new();
                for (override_key, override_value) in override_mapping {
                    match base_mapping.get(&override_key) {
                        Some(base_value) => {
                            if let Some(diff_value) =
                                diff_override_config_yaml(override_value, base_value.clone())?
                            {
                                overrides.insert(override_key, diff_value);
                            }
                        },
                        None => {
                            overrides.insert(override_key, override_value);
                        },
                    }
                }
                if overrides.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(serde_yaml::Value::Mapping(overrides)))
                }
            },
            _ => Ok(Some(override_config)),
        },
        serde_yaml::Value::Null => match base_config {
            serde_yaml::Value::Null => Ok(None),
            _ => bail!("base does not match override: Null"),
        },
        serde_yaml::Value::Bool(override_value) => match base_config {
            serde_yaml::Value::Bool(base_value) => {
                if override_value == base_value {
                    Ok(None)
                } else {
                    Ok(Some(override_config))
                }
            },
            _ => bail!(
                "base does not match override: Bool({}), {:?}",
                override_value,
                base_config
            ),
        },
        serde_yaml::Value::Number(override_value) => match base_config {
            serde_yaml::Value::Number(base_value) => {
                if override_value == base_value {
                    Ok(None)
                } else {
                    Ok(Some(override_config))
                }
            },
            _ => bail!(
                "base does not match override: Number({}), {:?}",
                override_value,
                base_config
            ),
        },
        serde_yaml::Value::String(override_value) => match base_config {
            serde_yaml::Value::String(base_value) => {
                if override_value == base_value {
                    Ok(None)
                } else {
                    Ok(Some(override_config))
                }
            },
            _ => bail!(
                "base does not match override: String({}), {:?}",
                override_value,
                base_config
            ),
        },
        serde_yaml::Value::Sequence(override_value) => match base_config {
            serde_yaml::Value::Sequence(base_value) => {
                if override_value == base_value {
                    Ok(None)
                } else {
                    Ok(Some(override_config))
                }
            },
            _ => bail!(
                "base does not match override: {:?}, {:?}",
                override_config,
                base_config
            ),
        },
    }
}

/// A utility struct for managing a node config that is overriding a base config, and outputting a
/// yaml representation of it. In most cases, the base config will be the default config, and the
/// output will be a minimal yaml diff that an aptos node can read.
///
/// In rare cases you may want to explicitly write a yaml value that is the same as the default
/// config (e.g., to avoid the config being optimized by ConfigOptimizer). To do this, change the
/// base config to a different value before calling get_yaml().
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct OverrideNodeConfig {
    config: NodeConfig,
    base: NodeConfig,
}

impl OverrideNodeConfig {
    pub fn new(config: NodeConfig, base: NodeConfig) -> Self {
        Self { config, base }
    }

    pub fn new_with_default_base(config: NodeConfig) -> Self {
        Self {
            config,
            base: NodeConfig::default(),
        }
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.config
    }

    pub fn base(&self) -> &NodeConfig {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut NodeConfig {
        &mut self.base
    }

    pub fn get_yaml(&self) -> anyhow::Result<serde_yaml::Value> {
        let config_yaml = serde_yaml::to_value(&self.config)?;
        let base_yaml = serde_yaml::to_value(&self.base)?;
        diff_override_config_yaml(config_yaml, base_yaml)
            .map(|diff_yaml| diff_yaml.unwrap_or(serde_yaml::Value::Null))
    }

    /// Save the node config to the given path
    pub fn save_to_path<P: AsRef<Path>>(&mut self, output_path: P) -> Result<(), Error> {
        self.config.save_to_path(output_path)
    }
}

impl PersistableConfig for OverrideNodeConfig {
    fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let config = NodeConfig::load_config(path)?;
        Ok(Self::new_with_default_base(config))
    }

    fn save_config<P: AsRef<Path>>(&self, output_file: P) -> Result<(), Error> {
        let yaml_value = self.get_yaml()?;
        let yaml_string = serde_yaml::to_string(&yaml_value).map_err(|e| {
            Error::Yaml(
                "Unable to serialize override config to yaml. Error: {}".to_string(),
                e,
            )
        })?;
        let yaml_bytes = yaml_string.as_bytes().to_vec();
        Self::write_file(yaml_bytes, output_file)
    }
}

#[cfg(test)]
mod test {
    use crate::config::{
        merge_node_config, persistable_config::PersistableConfig, Error, NetworkConfig, NodeConfig,
        OverrideNodeConfig, SafetyRulesConfig, WaypointConfig,
    };
    use std::{env::temp_dir, path::PathBuf};

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

    #[test]
    fn test_override_node_config_no_diff() {
        let override_config = OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let diff_yaml = override_config.get_yaml().unwrap();
        assert_eq!(diff_yaml, serde_yaml::Value::Null);
    }

    #[test]
    fn test_override_node_config_with_bool() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.config_mut();
        config.api.enabled = false;

        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
                api:
                    enabled: false
                "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_node_config_with_enum() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.config_mut();
        config.base.waypoint = WaypointConfig::FromFile(PathBuf::from("test"));
        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
                base:
                    waypoint:
                        from_file: test
                "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_node_config_with_empty_base_vector() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.config_mut();
        config.full_node_networks.push(Default::default());
        config.full_node_networks.push(Default::default());
        let diff_yaml = override_config.get_yaml().unwrap();
        let default_node_config = serde_yaml::to_value(&NetworkConfig::default()).unwrap();
        let mut expected_yaml: serde_yaml::Value = serde_yaml::Value::Null;
        expected_yaml["full_node_networks"] =
            serde_yaml::Value::Sequence(vec![default_node_config.clone(), default_node_config]);
        // TODO: why don't the actual Values match, only matches with as_str?
        assert_eq!(diff_yaml.as_str(), expected_yaml.as_str());
    }

    #[test]
    fn test_override_node_config_with_non_empty_base_vector() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.config_mut();
        config.full_node_networks.push(Default::default());
        config.full_node_networks.push(Default::default());
        let base = override_config.base_mut();
        base.full_node_networks.push(Default::default());

        // Note, the diff will include the entire vector, not just the non-equal elements
        let diff_yaml = override_config.get_yaml().unwrap();
        let default_node_config = serde_yaml::to_value(&NetworkConfig::default()).unwrap();
        let mut expected_yaml: serde_yaml::Value = serde_yaml::Value::Null;
        expected_yaml["full_node_networks"] =
            serde_yaml::Value::Sequence(vec![default_node_config.clone(), default_node_config]);
        // TODO: why don't the actual Values match, only matches with as_str?
        assert_eq!(diff_yaml.as_str(), expected_yaml.as_str());
    }

    #[test]
    fn test_override_node_config_with_base_change() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let base = override_config.base_mut();
        base.api.enabled = false;

        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
                api:
                    enabled: true
                "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_config_load_save() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.config_mut();
        config.api.enabled = false;

        let temp_file = temp_dir().join("override_config.yaml");
        override_config.save_config(temp_file.as_path()).unwrap();
        let loaded_config = OverrideNodeConfig::load_config(temp_file.as_path()).unwrap();
        assert_eq!(override_config, loaded_config);
    }
}
