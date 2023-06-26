// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        node_config_loader::NodeType, utils::get_config_name, Error, InspectionServiceConfig,
        LoggerConfig, MempoolConfig, NodeConfig, PeerMonitoringServiceConfig, StateSyncConfig,
    },
    network_id::NetworkId,
};
use aptos_types::chain_id::ChainId;
use serde_yaml::Value;

// Useful optimizer constants
const OPTIMIZER_STRING: &str = "Optimizer";
const VALIDATOR_NETWORK_OPTIMIZER_NAME: &str = "ValidatorNetworkConfigOptimizer";

/// A trait for optimizing node configs (and their sub-configs) by tweaking
/// config values based on node types, chain IDs and compiler features.
///
/// Note: The config optimizer respects the following order precedence when
/// determining whether or not to optimize a value:
/// 1. If a config value has been set in the local config file, that value
///    should be used (and the optimizer should not override it).
/// 2. If a config value has not been set in the local config file, the
///    optimizer may set the value (but, it is not required to do so).
/// 3. Finally, if the config optimizer chooses not to set a value, the default
///    value is used (as defined in the default implementation).
pub trait ConfigOptimizer {
    /// Get the name of the optimizer (e.g., for logging)
    fn get_optimizer_name() -> String {
        let config_name = get_config_name::<Self>().to_string();
        config_name + OPTIMIZER_STRING
    }

    /// Optimize the node config according to the given node type and chain ID
    /// and return true iff the config was modified.
    ///
    /// Note: the `local_config_yaml` contains the raw YAML string of the node
    /// config as provided by the user. This is used to check if a value
    /// should not be optimized/modified (as it has been set by the user).
    fn optimize(
        _node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<bool, Error> {
        unimplemented!("optimize() must be implemented for each optimizer!");
    }
}

impl ConfigOptimizer for NodeConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        chain_id: ChainId,
    ) -> Result<bool, Error> {
        // Optimize only the relevant sub-configs
        let mut optimizers_with_modifications = vec![];
        if InspectionServiceConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(InspectionServiceConfig::get_optimizer_name());
        }
        if LoggerConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(LoggerConfig::get_optimizer_name());
        }
        if MempoolConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(MempoolConfig::get_optimizer_name());
        }
        if PeerMonitoringServiceConfig::optimize(
            node_config,
            local_config_yaml,
            node_type,
            chain_id,
        )? {
            optimizers_with_modifications.push(PeerMonitoringServiceConfig::get_optimizer_name());
        }
        if StateSyncConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(StateSyncConfig::get_optimizer_name());
        }
        if optimize_validator_network_config(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(VALIDATOR_NETWORK_OPTIMIZER_NAME.to_string());
        }

        // Return true iff any config modifications were made
        Ok(!optimizers_with_modifications.is_empty())
    }
}

/// Optimize the validator network config according to the node type and chain ID
fn optimize_validator_network_config(
    node_config: &mut NodeConfig,
    local_config_yaml: &Value,
    _node_type: NodeType,
    _chain_id: ChainId,
) -> Result<bool, Error> {
    let mut modified_config = false;
    if let Some(validator_network_config) = &mut node_config.validator_network {
        let local_network_config_yaml = &local_config_yaml["validator_network_config"];

        // We must override the network ID to be a validator
        // network ID (as the config defaults to a public network ID).
        if local_network_config_yaml["network_id"].is_null() {
            validator_network_config.network_id = NetworkId::Validator;
            modified_config = true;
        }

        // We must enable mutual authentication for the validator network
        if local_network_config_yaml["mutual_authentication"].is_null() {
            validator_network_config.mutual_authentication = true;
            modified_config = true;
        }
    }

    Ok(modified_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::NetworkConfig, network_id::NetworkId};

    #[test]
    fn test_optimize_validator_network_config() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Optimize the validator network config and verify modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::Validator,
            ChainId::testnet(),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the network ID and mutual authentication have been changed
        let validator_network = node_config.validator_network.unwrap();
        assert_eq!(validator_network.network_id, NetworkId::Validator);
        assert!(validator_network.mutual_authentication);
    }

    #[test]
    fn test_optimize_validator_config_no_override() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create a local config with the network ID overridden
        let local_config_yaml = serde_yaml::from_str(
            r#"
            validator_network_config:
                network_id: "Public"
            "#,
        )
        .unwrap();

        // Optimize the validator network config and verify modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            ChainId::mainnet(),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the network ID has not changed but that
        // mutual authentication has been enabled.
        let validator_network = node_config.validator_network.unwrap();
        assert_eq!(validator_network.network_id, NetworkId::Public);
        assert!(validator_network.mutual_authentication);
    }

    #[test]
    fn test_optimize_validator_config_no_modifications() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create a local config with the network ID and mutual authentication set
        let local_config_yaml = serde_yaml::from_str(
            r#"
            validator_network_config:
                network_id: "Public"
                mutual_authentication: false
            "#,
        )
        .unwrap();

        // Optimize the validator network config and verify no modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            ChainId::mainnet(),
        )
        .unwrap();
        assert!(!modified_config);
    }
}
