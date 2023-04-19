// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        ApiConfig, BaseConfig, ConsensusConfig, Error, ExecutionConfig, IndexerConfig,
        IndexerGrpcConfig, InspectionServiceConfig, LoggerConfig, MempoolConfig, NodeConfig,
        PeerMonitoringServiceConfig, RoleType, StateSyncConfig, StorageConfig,
    },
    network_id::NetworkId,
};
use aptos_types::chain_id::ChainId;
use cfg_if::cfg_if;
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

    /// Validate and process the config according to the given node role and chain ID
    fn sanitize(
        _node_config: &mut NodeConfig,
        _node_role: RoleType,
        _chain_id: ChainId,
    ) -> Result<(), Error> {
        unimplemented!("sanitize() must be implemented for each sanitizer!");
    }
}

impl ConfigSanitizer for NodeConfig {
    fn sanitize(
        node_config: &mut NodeConfig,
        node_role: RoleType,
        chain_id: ChainId,
    ) -> Result<(), Error> {
        // Sanitize all of the sub-configs
        ApiConfig::sanitize(node_config, node_role, chain_id)?;
        BaseConfig::sanitize(node_config, node_role, chain_id)?;
        ConsensusConfig::sanitize(node_config, node_role, chain_id)?;
        ExecutionConfig::sanitize(node_config, node_role, chain_id)?;
        sanitize_failpoints_config(node_config, node_role, chain_id)?;
        sanitize_fullnode_network_configs(node_config, node_role, chain_id)?;
        IndexerConfig::sanitize(node_config, node_role, chain_id)?;
        IndexerGrpcConfig::sanitize(node_config, node_role, chain_id)?;
        InspectionServiceConfig::sanitize(node_config, node_role, chain_id)?;
        LoggerConfig::sanitize(node_config, node_role, chain_id)?;
        MempoolConfig::sanitize(node_config, node_role, chain_id)?;
        PeerMonitoringServiceConfig::sanitize(node_config, node_role, chain_id)?;
        StateSyncConfig::sanitize(node_config, node_role, chain_id)?;
        StorageConfig::sanitize(node_config, node_role, chain_id)?;
        sanitize_validator_network_config(node_config, node_role, chain_id)?;

        Ok(()) // All configs passed validation
    }
}

/// Returns true iff failpoints are enabled
fn are_failpoints_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "failpoints")] {
            true
        } else {
            false
        }
    }
}

/// Returns the name of the given config type
fn get_config_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
        .split("::")
        .last()
        .unwrap_or("UnknownConfig")
}

/// Validate and process the failpoints config according to the node role and chain ID
fn sanitize_failpoints_config(
    node_config: &mut NodeConfig,
    _node_role: RoleType,
    chain_id: ChainId,
) -> Result<(), Error> {
    let sanitizer_name = FAILPOINTS_SANITIZER_NAME.to_string();
    let failpoints = &node_config.failpoints;

    // Verify that failpoints are not enabled in mainnet
    let failpoints_enabled = are_failpoints_enabled();
    if chain_id.is_mainnet() && failpoints_enabled {
        return Err(Error::ConfigSanitizerFailed(
            sanitizer_name,
            "Failpoints are not supported on mainnet nodes!".into(),
        ));
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
    } else if failpoints_enabled {
        return Err(Error::ConfigSanitizerFailed(
            sanitizer_name,
            "Failpoints are enabled, but the failpoints config is None!".into(),
        ));
    }

    Ok(())
}

/// Validate and process the fullnode network configs according to the node role and chain ID
fn sanitize_fullnode_network_configs(
    node_config: &mut NodeConfig,
    node_role: RoleType,
    _chain_id: ChainId,
) -> Result<(), Error> {
    let sanitizer_name = FULLNODE_NETWORKS_SANITIZER_NAME.to_string();
    let fullnode_networks = &mut node_config.full_node_networks;

    // Verify that the fullnode network configs are not empty for fullnodes
    if fullnode_networks.is_empty() && !node_role.is_validator() {
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

        // Prepare the network id
        fullnode_network_config.set_listen_address_and_prepare_identity()?;
    }

    Ok(())
}

/// Validate and process the validator network config according to the node role and chain ID
fn sanitize_validator_network_config(
    node_config: &mut NodeConfig,
    node_role: RoleType,
    _chain_id: ChainId,
) -> Result<(), Error> {
    let sanitizer_name = VALIDATOR_NETWORK_SANITIZER_NAME.to_string();
    let validator_network = &mut node_config.validator_network;

    // Verify that the validator network config is not empty for validators
    if validator_network.is_none() && node_role.is_validator() {
        return Err(Error::ConfigSanitizerFailed(
            sanitizer_name,
            "Validator network config cannot be empty for validators!".into(),
        ));
    }

    // Check the validator network config
    if let Some(validator_network_config) = validator_network {
        let network_id = validator_network_config.network_id;
        if !network_id.is_validator_network() {
            // TODO: improve the defaults!
            // We must override the network ID to be a validator
            // network ID as the config defaults to a public network ID.
            validator_network_config.network_id = NetworkId::Validator;
        }

        // Verify that the node is a validator
        if !node_role.is_validator() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The validator network config cannot be set for non-validators!".into(),
            ));
        }

        // Ensure that mutual authentication is enabled
        if !validator_network_config.mutual_authentication {
            // TODO: improve the defaults!
            // We must enable mutual authentication for validators
            validator_network_config.mutual_authentication = true;
        }

        // Prepare the network id
        validator_network_config.set_listen_address_and_prepare_identity()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::NetworkConfig, network_id::NetworkId};

    #[test]
    fn test_sanitize_missing_fullnode_network_configs() {
        // Create a fullnode config with empty fullnode network configs
        let mut node_config = NodeConfig {
            full_node_networks: vec![],
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &mut node_config,
            RoleType::FullNode,
            ChainId::testnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_network_for_fullnode() {
        // Create a fullnode config that includes a validator network
        let mut node_config = NodeConfig {
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Validator,
                ..Default::default()
            }],
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_fullnode_network_configs(
            &mut node_config,
            RoleType::FullNode,
            ChainId::testnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_duplicate_fullnode_network_configs() {
        // Create a node config with multiple fullnode network configs with the same network id
        let mut node_config = NodeConfig {
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
            &mut node_config,
            RoleType::FullNode,
            ChainId::testnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_missing_validator_network_config() {
        // Create a node config with an empty validator network config
        let mut node_config = NodeConfig {
            validator_network: None,
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = sanitize_validator_network_config(
            &mut node_config,
            RoleType::Validator,
            ChainId::testnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_network_fullnode() {
        // Create a validator network config
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Validator,
                mutual_authentication: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Sanitize the config (for a fullnode) and verify that it fails
        let error = sanitize_validator_network_config(
            &mut node_config,
            RoleType::FullNode,
            ChainId::testnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_validator_disabled_authentication() {
        // Create a validator config with disabled mutual authentication
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Validator,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Sanitize the config
        sanitize_validator_network_config(
            &mut node_config,
            RoleType::Validator,
            ChainId::testnet(),
        )
        .unwrap();

        // Verify that mutual authentication is now enabled
        let validator_network = node_config.validator_network.unwrap();
        assert!(validator_network.mutual_authentication);
    }
}
