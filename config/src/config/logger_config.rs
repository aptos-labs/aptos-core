// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{config_sanitizer::ConfigSanitizer, Error, NodeConfig, RoleType},
    utils,
};
use aptos_logger::{Level, CHANNEL_SIZE};
use aptos_types::chain_id::ChainId;
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerConfig {
    // channel size for the asynchronous channel for node logging.
    pub chan_size: usize,
    // Enables backtraces on error logs
    pub enable_backtrace: bool,
    // Use async logging
    pub is_async: bool,
    // The default logging level for slog.
    pub level: Level,
    pub enable_telemetry_remote_log: bool,
    pub enable_telemetry_flush: bool,
    pub telemetry_level: Level,
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
    /// Validate and process the logger config according to the given node role and chain ID
    fn sanitize(
        node_config: &mut NodeConfig,
        _node_role: RoleType,
        _chain_id: ChainId,
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

/// Returns true iff the tokio-console feature is enabled
fn is_tokio_console_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "tokio-console")] {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_missing_feature() {
        // Create a logger config with the tokio console port set
        let mut node_config = NodeConfig {
            logger: LoggerConfig {
                tokio_console_port: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that the config fails sanitization (the tokio-console feature is missing!)
        let error =
            LoggerConfig::sanitize(&mut node_config, RoleType::Validator, ChainId::testnet())
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
