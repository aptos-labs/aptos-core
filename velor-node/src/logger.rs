// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::mpsc::Receiver;
use velor_build_info::build_information;
use velor_config::config::NodeConfig;
use velor_logger::{
    velor_logger::FileWriter, info, telemetry_log_writer::TelemetryLog, LoggerFilterUpdater,
};
use futures::channel::mpsc;
use std::path::PathBuf;

const TELEMETRY_LOG_INGEST_BUFFER_SIZE: usize = 128;

// Simple macro to help print out feature configurations
macro_rules! log_feature_info {
    ($($feature:literal),*) => {
        $(
        if cfg!(feature = $feature) {
            info!("Running with {} feature enabled", $feature);
        } else {
            info!("Running with {} feature disabled", $feature);
        }
        )*
    }
}

/// Creates the logger and returns the remote log receiver alongside
/// the logger filter updater.
pub fn create_logger(
    node_config: &NodeConfig,
    log_file: Option<PathBuf>,
) -> (Option<Receiver<TelemetryLog>>, LoggerFilterUpdater) {
    // Create the logger builder
    let mut logger_builder = velor_logger::Logger::builder();
    let mut remote_log_receiver = None;
    logger_builder
        .channel_size(node_config.logger.chan_size)
        .is_async(node_config.logger.is_async)
        .level(node_config.logger.level)
        .telemetry_level(node_config.logger.telemetry_level)
        .enable_telemetry_flush(node_config.logger.enable_telemetry_flush)
        .tokio_console_port(node_config.logger.tokio_console_port);
    if node_config.logger.enable_backtrace {
        logger_builder.enable_backtrace();
    }
    if let Some(log_file) = log_file {
        logger_builder.printer(Box::new(FileWriter::new(log_file)));
    }
    if node_config.logger.enable_telemetry_remote_log {
        let (tx, rx) = mpsc::channel(TELEMETRY_LOG_INGEST_BUFFER_SIZE);
        logger_builder.remote_log_tx(tx);
        remote_log_receiver = Some(rx);
    }

    // Create the logger and the logger filter updater
    let logger = logger_builder.build();
    let logger_filter_updater = LoggerFilterUpdater::new(logger, logger_builder);

    // Log the build information and the config
    log_config_and_build_information(node_config);

    (remote_log_receiver, logger_filter_updater)
}

/// Logs the node config and build information
fn log_config_and_build_information(node_config: &NodeConfig) {
    // Log the build information
    info!("Build information:");
    let build_info = build_information!();
    for (key, value) in build_info {
        info!("{}: {}", key, value);
    }

    // Log the feature information. Note: this should be kept up-to-date
    // with the features defined in the velor-node Cargo.toml file.
    info!("Feature information:");
    log_feature_info!(
        "assert-private-keys-not-cloneable",
        "check-vm-features",
        "consensus-only-perf-test",
        "default",
        "failpoints",
        "indexer",
        "tokio-console"
    );

    // Log the node config
    let mut config = node_config;
    let mut masked_config;
    if let Some(u) = &node_config.indexer.postgres_uri {
        let mut parsed_url = url::Url::parse(u).expect("Invalid postgres uri");
        if parsed_url.password().is_some() {
            masked_config = node_config.clone();
            parsed_url.set_password(Some("*")).unwrap();
            masked_config.indexer.postgres_uri = Some(parsed_url.to_string());
            config = &masked_config;
        }
    }

    info!("Loaded node config: {:?}", config);
}
