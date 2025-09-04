// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
        node_config_loader::NodeType, utils::is_tokio_console_enabled, Error, NodeConfig,
    },
    utils,
};
use velor_logger::{Level, CHANNEL_SIZE};
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

// Useful constants for the logger config
const DEFAULT_TOKIO_CONSOLE_PORT: u16 = 6669;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerConfig {
    /// Channel size for asynchronous node logging
    pub chan_size: usize,
    /// Enables backtraces on error logs
    pub enable_backtrace: bool,
    /// Use asynchronous logging
    pub is_async: bool,
    /// The default logging level for the logger.
    pub level: Level,
    /// Whether to enable remote telemetry logging
    pub enable_telemetry_remote_log: bool,
    /// Whether to enable remote telemetry logging flushing
    pub enable_telemetry_flush: bool,
    /// Level for telemetry logging
    pub telemetry_level: Level,
    /// Tokio console port for local debugging
    pub tokio_console_port: Option<u16>,
}

impl Default for LoggerConfig {
    fn default() -> LoggerConfig {
        LoggerConfig {
            chan_size: CHANNEL_SIZE,
            enable_backtrace: false,
            is_async: true,
            level: Level::Info,
            enable_telemetry_remote_log: true,
            enable_telemetry_flush: true,
            telemetry_level: Level::Error,

            // This is the default port used by tokio-console.
            // Setting this to None will disable tokio-console
            // even if the "tokio-console" feature is enabled.
            tokio_console_port: None,
        }
    }
}

impl LoggerConfig {
    pub fn disable_tokio_console(&mut self) {
        self.tokio_console_port = None;
    }

    pub fn randomize_ports(&mut self) {
        self.tokio_console_port = Some(utils::get_available_port());
    }
}

impl ConfigSanitizer for LoggerConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let logger_config = &node_config.logger;

        // Verify that tokio console tracing is correctly configured
        if is_tokio_console_enabled() && logger_config.tokio_console_port.is_none() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The tokio-console feature is enabled but the tokio console port is not set!"
                    .into(),
            ));
        } else if !is_tokio_console_enabled() && logger_config.tokio_console_port.is_some() {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "The tokio-console feature is not enabled but the tokio console port is set!"
                    .into(),
            ));
        }

        Ok(())
    }
}

impl ConfigOptimizer for LoggerConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let logger_config = &mut node_config.logger;
        let local_logger_config_yaml = &local_config_yaml["logger"];

        // Set the tokio console port
        let mut modified_config = false;
        if local_logger_config_yaml["tokio_console_port"].is_null() {
            // If the tokio-console feature is enabled, set the default port.
            // Otherwise, disable the tokio console port.
            if is_tokio_console_enabled() {
                logger_config.tokio_console_port = Some(DEFAULT_TOKIO_CONSOLE_PORT);
            } else {
                logger_config.tokio_console_port = None;
            }
            modified_config = true;
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_tokio_console_port() {
        // Create a logger config with the tokio console port set
        let mut node_config = NodeConfig {
            logger: LoggerConfig {
                tokio_console_port: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the config and verify modifications are made
        let modified_config = LoggerConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify the tokio console port is not set
        assert!(node_config.logger.tokio_console_port.is_none());
    }

    #[test]
    fn test_optimize_tokio_console_port_no_override() {
        // Create a logger config with the tokio console port set
        let mut node_config = NodeConfig {
            logger: LoggerConfig {
                tokio_console_port: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };

        // Create a local config YAML with the tokio console port set
        let local_config_yaml = serde_yaml::from_str(
            r#"
            logger:
                tokio_console_port: 100,
            "#,
        )
        .unwrap();

        // Optimize the config and verify no modifications are made
        let modified_config = LoggerConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(!modified_config);

        // Verify the tokio console port is still set
        assert!(node_config.logger.tokio_console_port.is_some());
    }

    #[test]
    fn test_sanitize_missing_feature() {
        // Create a logger config with the tokio console port set
        let node_config = NodeConfig {
            logger: LoggerConfig {
                tokio_console_port: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that the config fails sanitization (the tokio-console feature is missing!)
        let error =
            LoggerConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::testnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
