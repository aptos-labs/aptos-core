// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use aptos_logger::{Level, CHANNEL_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerConfig {
    // channel size for the asychronous channel for node logging.
    pub chan_size: usize,
    // Enables backtraces on error logs
    pub enable_backtrace: bool,
    // Use async logging
    pub is_async: bool,
    // The default logging level for slog.
    pub level: Level,
    // tokio-console port
    pub console_port: Option<u16>,
    pub enable_telemetry_remote_log: bool,
    pub enable_telemetry_flush: bool,
    pub telemetry_level: Level,
}

impl Default for LoggerConfig {
    fn default() -> LoggerConfig {
        LoggerConfig {
            chan_size: CHANNEL_SIZE,
            enable_backtrace: false,
            is_async: true,
            level: Level::Info,
            // This is the default port used by tokio-console
            // setting console_port to None will disable tokio console even if aptos-console
            // feature is enabled
            console_port: Some(6669),
            enable_telemetry_remote_log: true,
            enable_telemetry_flush: true,
            telemetry_level: Level::Error,
        }
    }
}

impl LoggerConfig {
    pub fn disable_console(&mut self) {
        self.console_port = None;
    }
}

impl LoggerConfig {
    pub fn randomize_ports(&mut self) {
        self.console_port = Some(utils::get_available_port());
    }
}
