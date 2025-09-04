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
pub struct AdminServiceConfig {
    pub enabled: Option<bool>,
    pub address: String,
    pub port: u16,
    // If empty, will allow all requests without authentication. (Not allowed on mainnet.)
    pub authentication_configs: Vec<AuthenticationConfig>,
    pub malloc_stats_max_len: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationConfig {
    // This will allow authentication through query parameter.
    // e.g. `/profilez?passcode=abc`.
    //
    // To calculate sha256, use sha256sum tool, or other online tools.
    //
    // e.g.
    //
    // printf abc |sha256sum
    PasscodeSha256(String),
    // TODO(grao): Add SSL support if necessary.
}

impl Default for AdminServiceConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            address: "0.0.0.0".to_string(),
            port: 9102,
            authentication_configs: vec![],
            malloc_stats_max_len: 2 * 1024 * 1024,
        }
    }
}

impl AdminServiceConfig {
    pub fn randomize_ports(&mut self) {
        self.port = utils::get_available_port();
    }
}

impl ConfigSanitizer for AdminServiceConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();

        if node_config.admin_service.enabled == Some(true) {
            if let Some(chain_id) = chain_id {
                if chain_id.is_mainnet()
                    && node_config.admin_service.authentication_configs.is_empty()
                {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        "Must enable authentication for AdminService on mainnet.".into(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl ConfigOptimizer for AdminServiceConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let mut modified_config = false;

        if node_config.admin_service.enabled.is_none() {
            // Only enable the admin service if the chain is not mainnet
            let admin_service_enabled = if let Some(chain_id) = chain_id {
                !chain_id.is_mainnet()
            } else {
                false // We cannot determine the chain ID, so we disable the admin service
            };
            node_config.admin_service.enabled = Some(admin_service_enabled);

            modified_config = true; // The config was modified
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_admin_service_disabled() {
        // Create a node config with the admin service disabled
        let mut node_config = NodeConfig {
            admin_service: AdminServiceConfig {
                enabled: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config and verify that it succeeds
        let modified_config = AdminServiceConfig::optimize(
            &mut node_config,
            &Value::Null,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();

        // Verify that the admin service is disabled and that the config was not modified
        assert_eq!(node_config.admin_service.enabled, Some(false));
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_admin_service_enabled() {
        // Create a node config with the admin service enabled
        let mut node_config = NodeConfig {
            admin_service: AdminServiceConfig {
                enabled: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config and verify that it succeeds
        let modified_config = AdminServiceConfig::optimize(
            &mut node_config,
            &Value::Null,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();

        // Verify that the admin service is enabled and and that the config was not modified
        assert_eq!(node_config.admin_service.enabled, Some(true));
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_admin_service_not_set_mainnet() {
        // Create a node config with the admin service not specified
        let mut node_config = NodeConfig {
            admin_service: AdminServiceConfig {
                enabled: None,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config (for mainnet) and verify that it succeeds
        let modified_config = AdminServiceConfig::optimize(
            &mut node_config,
            &Value::Null,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();

        // Verify that the admin service is disabled and that the config was modified
        assert_eq!(node_config.admin_service.enabled, Some(false));
        assert!(modified_config);
    }

    #[test]
    fn test_optimize_admin_service_not_set_testnet() {
        // Create a node config with the admin service not specified
        let mut node_config = NodeConfig {
            admin_service: AdminServiceConfig {
                enabled: None,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config (for testnet) and verify that it succeeds
        let modified_config = AdminServiceConfig::optimize(
            &mut node_config,
            &Value::Null,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();

        // Verify that the admin service is enabled and that the config was modified
        assert_eq!(node_config.admin_service.enabled, Some(true));
        assert!(modified_config);
    }

    #[test]
    fn test_optimize_admin_service_not_set_unknown() {
        // Create a node config with the admin service not specified
        let mut node_config = NodeConfig {
            admin_service: AdminServiceConfig {
                enabled: None,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config (for an unknown network) and verify that it succeeds
        let modified_config =
            AdminServiceConfig::optimize(&mut node_config, &Value::Null, NodeType::Validator, None)
                .unwrap();

        // Verify that the admin service is disabled and that the config was modified
        assert_eq!(node_config.admin_service.enabled, Some(false));
        assert!(modified_config);
    }
}
