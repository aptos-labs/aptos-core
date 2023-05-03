// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{config_sanitizer::ConfigSanitizer, Error, NodeConfig, RoleType},
    utils,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InspectionServiceConfig {
    pub address: String,
    pub port: u16,
    pub expose_configuration: bool,
    pub expose_system_information: bool,
}

impl Default for InspectionServiceConfig {
    fn default() -> InspectionServiceConfig {
        InspectionServiceConfig {
            address: "0.0.0.0".to_string(),
            port: 9101,
            expose_configuration: false,
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
    /// Validate and process the inspection service config according to the given node role and chain ID
    fn sanitize(
        node_config: &mut NodeConfig,
        node_role: RoleType,
        chain_id: ChainId,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let inspection_service_config = &node_config.inspection_service;

        // Verify that mainnet validators do not expose the configuration
        if node_role.is_validator()
            && chain_id.is_mainnet()
            && inspection_service_config.expose_configuration
        {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Mainnet validators should not expose the node configuration!".to_string(),
            ));
        }

        // TODO: Verify that system information is not exposed for mainnet validators

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_valid_service_config() {
        // Create an inspection service config with the configuration endpoint enabled
        let mut node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that the configuration is sanitized successfully
        InspectionServiceConfig::sanitize(&mut node_config, RoleType::FullNode, ChainId::mainnet())
            .unwrap()
    }

    #[test]
    fn test_sanitize_config_mainnet() {
        // Create an inspection service config with the configuration endpoint enabled
        let mut node_config = NodeConfig {
            inspection_service: InspectionServiceConfig {
                expose_configuration: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that sanitization fails for mainnet
        let error = InspectionServiceConfig::sanitize(
            &mut node_config,
            RoleType::Validator,
            ChainId::mainnet(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
