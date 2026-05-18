// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, Error, NodeConfig, SecureBackend,
};
use aptos_secure_storage::{KVStorage, Storage};
use aptos_types::{chain_id::ChainId, waypoint::Waypoint};
use poem_openapi::Enum as PoemEnum;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{fmt, fs, path::PathBuf, str::FromStr};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BaseConfig {
    pub data_dir: PathBuf,
    pub working_dir: Option<PathBuf>,
    pub role: RoleType,
    pub waypoint: WaypointConfig,
    pub enable_validator_pfn_connections: bool,
}

impl Default for BaseConfig {
    fn default() -> BaseConfig {
        BaseConfig {
            data_dir: PathBuf::from("/opt/aptos/data"),
            working_dir: None,
            role: RoleType::Validator,
            waypoint: WaypointConfig::None,
            enable_validator_pfn_connections: false, // Whether to allow direct connections between validators and PFNs
        }
    }
}

impl ConfigSanitizer for BaseConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let base_config = &node_config.base;

        // Verify the waypoint is not None
        if let WaypointConfig::None = base_config.waypoint {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The waypoint config must be set in the base config!".into(),
            ));
        }

        Ok(())
    }
}

impl ConfigOptimizer for BaseConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let base_config = &mut node_config.base;
        let local_base_config_yaml = &local_config_yaml["base"];

        let mut modified_config = false;

        // Enable validator-PFN connections for all networks except test
        // environments (e.g., local swarms, and smoke tests).
        if local_base_config_yaml["enable_validator_pfn_connections"].is_null() {
            let should_enable = chain_id.map(|id| id != ChainId::test()).unwrap_or(true);
            if should_enable {
                base_config.enable_validator_pfn_connections = true;
                modified_config = true;
            }
        }

        Ok(modified_config)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointConfig {
    FromConfig(Waypoint),
    FromFile(PathBuf),
    FromStorage(SecureBackend),
    None,
}

impl WaypointConfig {
    pub fn waypoint_from_config(&self) -> Option<Waypoint> {
        if let WaypointConfig::FromConfig(waypoint) = self {
            Some(*waypoint)
        } else {
            None
        }
    }

    pub fn waypoint(&self) -> Waypoint {
        let waypoint = match &self {
            WaypointConfig::FromConfig(waypoint) => Some(*waypoint),
            WaypointConfig::FromFile(waypoint_path) => {
                if !waypoint_path.exists() {
                    panic!(
                        "Waypoint file not found! Ensure the given path is correct: {:?}",
                        waypoint_path.display()
                    );
                }
                let content = fs::read_to_string(waypoint_path).unwrap_or_else(|error| {
                    panic!(
                        "Failed to read waypoint file {:?}. Error: {:?}",
                        waypoint_path.display(),
                        error
                    )
                });
                Some(Waypoint::from_str(content.trim()).unwrap_or_else(|error| {
                    panic!(
                        "Failed to parse waypoint: {:?}. Error: {:?}",
                        content.trim(),
                        error
                    )
                }))
            },
            WaypointConfig::FromStorage(backend) => {
                let storage: Storage = backend.into();
                let waypoint = storage
                    .get::<Waypoint>(aptos_global_constants::WAYPOINT)
                    .expect("Unable to read waypoint")
                    .value;
                Some(waypoint)
            },
            WaypointConfig::None => None,
        };
        waypoint.expect("waypoint should be present")
    }

    pub fn genesis_waypoint(&self) -> Waypoint {
        match &self {
            WaypointConfig::FromStorage(backend) => {
                let storage: Storage = backend.into();
                storage
                    .get::<Waypoint>(aptos_global_constants::GENESIS_WAYPOINT)
                    .expect("Unable to read waypoint")
                    .value
            },
            _ => self.waypoint(),
        }
    }
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, PoemEnum, Serialize)]
#[serde(rename_all = "snake_case")]
#[oai(rename_all = "snake_case")]
pub enum RoleType {
    Validator,
    FullNode,
}

impl RoleType {
    pub fn is_validator(self) -> bool {
        self == RoleType::Validator
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RoleType::Validator => "validator",
            RoleType::FullNode => "full_node",
        }
    }
}

impl FromStr for RoleType {
    type Err = ParseRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "validator" => Ok(RoleType::Validator),
            "full_node" => Ok(RoleType::FullNode),
            _ => Err(ParseRoleError(s.to_string())),
        }
    }
}

impl fmt::Debug for RoleType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for RoleType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Error)]
#[error("Invalid node role: {0}")]
pub struct ParseRoleError(String);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sanitize_valid_base_config() {
        // Create a node config with a waypoint
        let node_config = NodeConfig {
            base: BaseConfig {
                waypoint: WaypointConfig::FromConfig(Waypoint::default()),
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it passes
        BaseConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_sanitize_missing_waypoint() {
        // Create a node config with a missing waypoint
        let node_config = NodeConfig {
            base: BaseConfig {
                waypoint: WaypointConfig::None,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because of the missing waypoint
        let error =
            BaseConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn verify_role_type_conversion() {
        // Verify relationship between RoleType and as_string() is reflexive
        let validator = RoleType::Validator;
        let full_node = RoleType::FullNode;
        let converted_validator = RoleType::from_str(validator.as_str()).unwrap();
        let converted_full_node = RoleType::from_str(full_node.as_str()).unwrap();
        assert_eq!(converted_validator, validator);
        assert_eq!(converted_full_node, full_node);
    }

    #[test]
    fn verify_parse_role_error_on_invalid_role() {
        let invalid_role_type = "this is not a valid role type";
        assert!(matches!(
            RoleType::from_str(invalid_role_type),
            Err(ParseRoleError(_))
        ));
    }

    #[test]
    fn test_optimize_validator_pfn_connections() {
        // Create a node config with PFN validator connections disabled
        let mut node_config = create_config_with_validator_pfn_connections(false);

        // Optimize for a testing chain (smoke tests) and verify the flag is still disabled
        let modified = BaseConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::Validator,
            Some(ChainId::test()),
        )
        .unwrap();
        assert!(!modified);
        assert!(!node_config.base.enable_validator_pfn_connections);
    }

    #[test]
    fn test_optimize_validator_pfn_connections_for_testnet() {
        // Create a node config with PFN validator connections disabled
        let mut node_config = create_config_with_validator_pfn_connections(false);

        // Optimize for testnet and verify the flag is now enabled
        let modified = BaseConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(),
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified);
        assert!(node_config.base.enable_validator_pfn_connections);
    }

    #[test]
    fn test_optimize_validator_pfn_connections_for_mainnet() {
        // Create a node config with PFN validator connections disabled
        let mut node_config = create_config_with_validator_pfn_connections(false);

        // Optimize for mainnet and verify the flag is now enabled
        let modified = BaseConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(),
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified);
        assert!(node_config.base.enable_validator_pfn_connections);
    }

    #[test]
    fn test_optimize_validator_pfn_connections_local_yaml() {
        // Create a node config with PFN validator connections disabled
        let mut node_config = create_config_with_validator_pfn_connections(false);

        // Provide a local YAML that explicitly sets the flag to false
        let local_config_yaml = serde_yaml::from_str(
            r#"
            base:
                enable_validator_pfn_connections: false
            "#,
        )
        .unwrap();

        // Optimize for a non-production chain and verify the flag is still disabled
        let modified = BaseConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(!modified);
        assert!(!node_config.base.enable_validator_pfn_connections);
    }

    #[test]
    fn test_optimize_validator_pfn_connections_missing_chain() {
        // Create a node config with PFN validator connections disabled
        let mut node_config = create_config_with_validator_pfn_connections(false);

        // Optimize for a missing chain and verify the flag is now enabled
        let modified = BaseConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(),
            NodeType::Validator,
            None,
        )
        .unwrap();
        assert!(modified);
        assert!(node_config.base.enable_validator_pfn_connections);
    }

    /// Creates a node config with validator PFN connections as specified
    fn create_config_with_validator_pfn_connections(
        enable_validator_pfn_connections: bool,
    ) -> NodeConfig {
        let mut node_config = NodeConfig::default();
        node_config.base.enable_validator_pfn_connections = enable_validator_pfn_connections;
        node_config
    }
}
