// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::internal_indexer_db_config::InternalIndexerDBConfig;
use crate::config::{
    node_config_loader::NodeType,
    utils::{are_failpoints_enabled, get_config_name},
    AdminServiceConfig, ApiConfig, BaseConfig, ConsensusConfig, DagConsensusConfig, Error,
    ExecutionConfig, IndexerGrpcConfig, InspectionServiceConfig, LoggerConfig, MempoolConfig,
    NetbenchConfig, NodeConfig, StateSyncConfig, StorageConfig,
};
use velor_types::chain_id::ChainId;
use std::collections::HashSet;

// Useful sanitizer constants
const FAILPOINTS_SANITIZER_NAME: &str = "FailpointsConfigSanitizer";
const FULLNODE_NETWORKS_SANITIZER_NAME: &str = "FullnodeNetworksConfigSanitizer";
const SANITIZER_STRING: &str = "Sanitizer";
const VALIDATOR_NETWORK_SANITIZER_NAME: &str = "ValidatorNetworkConfigSanitizer";

/// A trait for validating and sanitizing node configs (and their sub-configs)
pub trait ConfigSanitizer {
    /// Get the name of the sanitizer (e.g., for logging and error strings)
    fn get_sanitizer_name() -> String {
        let config_name = get_config_name::<Self>().to_string();
        config_name + SANITIZER_STRING
    }

    /// Validate and process the config according to the given node type and chain ID
    fn sanitize(
        _node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        unimplemented!("sanitize() must be implemented for each sanitizer!");
    }
}

impl ConfigSanitizer for NodeConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        // If config sanitization is disabled, don't do anything!
        if node_config.node_startup.skip_config_sanitizer {
            return Ok(());
        }

        // Sanitize all of the sub-configs
        AdminServiceConfig::sanitize(node_config, node_type, chain_id)?;
        ApiConfig::sanitize(node_config, node_type, chain_id)?;
        BaseConfig::sanitize(node_config, node_type, chain_id)?;
        ConsensusConfig::sanitize(node_config, node_type, chain_id)?;
        DagConsensusConfig::sanitize(node_config, node_type, chain_id)?;
        ExecutionConfig::sanitize(node_config, node_type, chain_id)?;
        sanitize_failpoints_config(node_config, node_type, chain_id)?;
        sanitize_fullnode_network_configs(node_config, node_type, chain_id)?;
        IndexerGrpcConfig::sanitize(node_config, node_type, chain_id)?;
        InspectionServiceConfig::sanitize(node_config, node_type, chain_id)?;
        LoggerConfig::sanitize(node_config, node_type, chain_id)?;
        MempoolConfig::sanitize(node_config, node_type, chain_id)?;
        NetbenchConfig::sanitize(node_config, node_type, chain_id)?;
        StateSyncConfig::sanitize(node_config, node_type, chain_id)?;
        StorageConfig::sanitize(node_config, node_type, chain_id)?;
        InternalIndexerDBConfig::sanitize(node_config, node_type, chain_id)?;
        sanitize_validator_network_config(node_config, node_type, chain_id)?;

        Ok(()) // All configs passed validation
    }
}

/// Sanitize the failpoints config according to the node role and chain ID
fn sanitize_failpoints_config(
    node_config: &NodeConfig,
    _node_type: NodeType,
    chain_id: Option<ChainId>,
) -> Result<(), Error> {
    let sanitizer_name = FAILPOINTS_SANITIZER_NAME.to_string();
    let failpoints = &node_config.failpoints;

    // Verify that failpoints are not enabled in mainnet
    let failpoints_enabled = are_failpoints_enabled();
    if let Some(chain_id) = chain_id {
        if chain_id.is_mainnet() && failpoints_enabled {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Failpoints are not supported on mainnet nodes!".into(),
            ));
        }
    }

    // Ensure that the failpoints config is populated appropriately
    if let Some(failpoints) = failpoints {
        if failpoints_enabled && failpoints.is_empty() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Failpoints are enabled, but the failpoints config is empty?".into(),
            ));
        } else if !failpoints_enabled && !failpoints.is_empty() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Failpoints are disabled, but the failpoints config is not empty!".into(),
            ));
        }
    }

    Ok(())
}

/// Sanitize the fullnode network configs according to the node role and chain ID
fn sanitize_fullnode_network_configs(
    node_config: &NodeConfig,
    node_type: NodeType,
    _chain_id: Option<ChainId>,
) -> Result<(), Error> {
    let sanitizer_name = FULLNODE_NETWORKS_SANITIZER_NAME.to_string();
    let fullnode_networks = &node_config.full_node_networks;

    // Verify that the fullnode network configs are not empty for fullnodes
    if fullnode_networks.is_empty() && !node_type.is_validator() {
        return Err(Error::ConfigSanitizerFailed(
            sanitizer_name,
            "Fullnode networks cannot be empty for fullnodes!".into(),
        ));
    }

    // Check each fullnode network config and ensure uniqueness
    let mut fullnode_network_ids = HashSet::new();
    for fullnode_network_config in fullnode_networks {
        let network_id = fullnode_network_config.network_id;

        // Verify that the fullnode network config is not a validator network config
        if network_id.is_validator_network() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Fullnode network configs cannot include a validator network!".into(),
            ));
        }

        // Verify that the fullnode network config is unique
        if !fullnode_network_ids.insert(network_id) {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                format!(
                    "Each fullnode network config must be unique! Found duplicate: {}",
                    network_id
                ),
            ));
        }
    }

    Ok(())
}

/// Sanitize the validator network config according to the node role and chain ID
fn sanitize_validator_network_config(
    node_config: &NodeConfig,
    node_type: NodeType,
    _chain_id: Option<ChainId>,
) -> Result<(), Error> {
    let sanitizer_name = VALIDATOR_NETWORK_SANITIZER_NAME.to_string();
    let validator_network = &node_config.validator_network;

    // Verify that the validator network config is not empty for validators
    if validator_network.is_none() && node_type.is_validator() {
        return Err(Error::ConfigSanitizerFailed(
            sanitizer_name,
            "Validator network config cannot be empty for validators!".into(),
        ));
    }

    // Check the validator network config
    if let Some(validator_network_config) = validator_network {
        let network_id = validator_network_config.network_id;
        if !network_id.is_validator_network() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The validator network config must have a validator network ID!".into(),
            ));
        }

        // Verify that the node is a validator
        if !node_type.is_validator() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The validator network config cannot be set for non-validators!".into(),
            ));
        }

        // Ensure that mutual authentication is enabled
        if !validator_network_config.mutual_authentication {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Mutual authentication must be enabled for the validator network!".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{node_startup_config::NodeStartupConfig, NetworkConfig},
        network_id::NetworkId,
    };

    #[test]
    fn test_disable_config_sanitizer() {
        // Create a default node config (with sanitization enabled)
        let mut node_config = NodeConfig::default();

        // Set a bad node config for mainnet
        node_config.execution.paranoid_hot_potato_verification = false;

        // Sanitize the config and verify the sanitizer fails
        let error =
            NodeConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Create a node config with the sanitizer disabled
        let mut node_config = NodeConfig {
            node_startup: NodeStartupConfig {
                skip_config_sanitizer: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Set a bad node config for mainnet
        node_config.execution.paranoid_hot_potato_verification = false;

        // Sanitize the config and verify the sanitizer passes
        NodeConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_sanitize_missing_pfn_network_configs() {
        // Create a PFN config with empty fullnode network configs
        let node_config = NodeConfig {
            full_node_networks: vec![],
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &node_config,
            NodeType::PublicFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_missing_vfn_network_configs() {
        // Create a VFN config with empty fullnode network configs
        let node_config = NodeConfig {
            full_node_networks: vec![],
            ..Default::default()
        };

        // Sanitize the PFN config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_network_for_fullnode() {
        // Create a fullnode config that includes a validator network
        let node_config = NodeConfig {
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Validator,
                ..Default::default()
            }],
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &node_config,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_duplicate_fullnode_network_configs() {
        // Create a node config with multiple fullnode network configs with the same network id
        let node_config = NodeConfig {
            full_node_networks: vec![
                NetworkConfig {
                    network_id: NetworkId::Public,
                    ..Default::default()
                },
                NetworkConfig {
                    network_id: NetworkId::Public,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_missing_validator_network_config() {
        // Create a node config with an empty validator network config
        let node_config = NodeConfig {
            validator_network: None,
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_validator_network_config(
            &node_config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_network_fullnode() {
        // Create a validator network config
        let node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Validator,
                mutual_authentication: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Sanitize the config (for a fullnode) and verify that it fails
        let error = sanitize_validator_network_config(
            &node_config,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_disabled_authentication() {
        // Create a validator config with disabled mutual authentication
        let node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Validator,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_validator_network_config(
            &node_config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_incorrect_network_id() {
        // Create a validator config with the wrong network ID
        let node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_validator_network_config(
            &node_config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
