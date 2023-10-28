// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
        node_config_loader::NodeType, Error, NodeConfig,
    },
    utils,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdminServiceConfig {
    pub enabled: Option<bool>,
    pub address: String,
    pub port: u16,
    // TODO(grao): Add auth support if necessary.
}

impl Default for AdminServiceConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            address: "0.0.0.0".to_string(),
            port: 9102,
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
        _node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
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
