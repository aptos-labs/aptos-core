// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
        node_config_loader::NodeType, Error, NodeConfig,
    },
    utils,
};
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InspectionServiceConfig {
    pub address: String,
    pub port: u16,
    pub expose_configuration: bool,
    pub expose_identity_information: bool,
    pub expose_peer_information: bool,
    pub expose_system_information: bool,
}

impl Default for InspectionServiceConfig {
    fn default() -> InspectionServiceConfig {
        InspectionServiceConfig {
            address: "0.0.0.0".to_string(),
            port: 9101,
            expose_configuration: false,
            expose_identity_information: true,
            expose_peer_information: true,
            expose_system_information: true,
        }
    }
}

impl InspectionServiceConfig {
    pub fn randomize_ports(&mut self) {
        self.port = utils::get_available_port();
    }
}

impl ConfigSanitizer for InspectionServiceConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let inspection_service_config = &node_config.inspection_service;

        // Verify that mainnet validators do not expose the configuration
        if let Some(chain_id) = chain_id {
            if node_type.is_validator()
                && chain_id.is_mainnet()
                && inspection_service_config.expose_configuration
            {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "Mainnet validators should not expose the node configuration!".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl ConfigOptimizer for InspectionServiceConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let inspection_service_config = &mut node_config.inspection_service;
        let local_inspection_config_yaml = &local_config_yaml["inspection_service"];

        // Enable all endpoints for non-mainnet nodes (to aid debugging)
        let mut modified_config = false;
        if let Some(chain_id) = chain_id {
            if !chain_id.is_mainnet() {
                if local_inspection_config_yaml["expose_configuration"].is_null() {
                    inspection_service_config.expose_configuration = true;
                    modified_config = true;
                }

                if local_inspection_config_yaml["expose_identity_information"].is_null() {
                    inspection_service_config.expose_identity_information = true;
                    modified_config = true;
                }

                if local_inspection_config_yaml["expose_peer_information"].is_null() {
                    inspection_service_config.expose_peer_information = true;
                    modified_config = true;
                }

                if local_inspection_config_yaml["expose_system_information"].is_null() {
                    inspection_service_config.expose_system_information = true;
                    modified_config = true;
                }
            }
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_mainnet_config() {
        // Create an inspection service config with all endpoints disabled
        let mut node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: false,
                expose_identity_information: false,
                expose_peer_information: false,
                expose_system_information: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config and verify no modifications are made
        let modified_config = InspectionServiceConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::PublicFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(!modified_config);

        // Verify all endpoints are still disabled
        assert!(!node_config.inspection_service.expose_configuration);
        assert!(!node_config.inspection_service.expose_identity_information);
        assert!(!node_config.inspection_service.expose_peer_information);
        assert!(!node_config.inspection_service.expose_system_information);
    }

    #[test]
    fn test_optimize_testnet_config() {
        // Create an inspection service config with all endpoints disabled
        let mut node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: false,
                expose_peer_information: false,
                expose_system_information: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config and verify modifications are made
        let modified_config = InspectionServiceConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify all endpoints are now enabled
        assert!(node_config.inspection_service.expose_configuration);
        assert!(node_config.inspection_service.expose_identity_information);
        assert!(node_config.inspection_service.expose_peer_information);
        assert!(node_config.inspection_service.expose_system_information);
    }

    #[test]
    fn test_optimize_testnet_partial_config() {
        // Create an inspection service config with all endpoints disabled
        let mut node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: false,
                expose_peer_information: false,
                expose_system_information: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // Create a local config YAML with the configuration endpoint disabled
        let local_config_yaml = serde_yaml::from_str(
            r#"
            inspection_service:
                expose_configuration: false
            "#,
        )
        .unwrap();

        // Optimize the config and verify modifications are made
        let modified_config = InspectionServiceConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify only the system information endpoint is now enabled
        assert!(!node_config.inspection_service.expose_configuration);
        assert!(node_config.inspection_service.expose_identity_information);
        assert!(node_config.inspection_service.expose_peer_information);
        assert!(node_config.inspection_service.expose_system_information);
    }

    #[test]
    fn test_sanitize_valid_service_config() {
        // Create an inspection service config with the configuration endpoint enabled
        let node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that the configuration is sanitized successfully
        InspectionServiceConfig::sanitize(
            &node_config,
            NodeType::PublicFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap()
    }

    #[test]
    fn test_sanitize_config_mainnet() {
        // Create an inspection service config with the configuration endpoint enabled
        let node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that sanitization fails for mainnet
        let error = InspectionServiceConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
