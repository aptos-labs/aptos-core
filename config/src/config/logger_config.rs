// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use aptos_logger::{Level, CHANNEL_SIZE};
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
