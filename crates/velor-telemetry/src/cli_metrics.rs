// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{service, utils};
use velor_logger::debug;
use velor_telemetry_service::types::telemetry::TelemetryEvent;
use std::{collections::BTreeMap, time::Duration};

/// CLI metrics event name
const VELOR_CLI_METRICS: &str = "VELOR_CLI_METRICS";

/// Core metric keys
const COMMAND: &str = "command";
const LATENCY: &str = "latency";
const SUCCESS: &str = "success";
const ERROR: &str = "error";

/// Collects and sends the build information via telemetry
pub async fn send_cli_telemetry_event(
    mut build_information: BTreeMap<String, String>,
    command: String,
    latency: Duration,
    success: bool,
    error: Option<&str>,
) {
    // Collection information about the cli command
    collect_cli_info(command, latency, success, error, &mut build_information);

    // Create a new telemetry event
    let telemetry_event = TelemetryEvent {
        name: VELOR_CLI_METRICS.into(),
        params: build_information,
    };

    // TODO(joshlind): can we find a better way of identifying each CLI user?
    let user_id = uuid::Uuid::new_v4().to_string();

    // Send the event (we block on the join handle to ensure the
    // event is processed before terminating the cli command).
    let join_handle = service::prepare_and_send_telemetry_event(
        user_id,
        "NO_CHAIN".into(),
        None,
        telemetry_event,
    )
    .await;
    if let Err(error) = join_handle.await {
        debug!(
            "Failed to send telemetry event with join error: {:?}",
            error
        );
    }
}

/// Collects the cli info and appends it to the given map
pub(crate) fn collect_cli_info(
    command: String,
    latency: Duration,
    success: bool,
    error: Option<&str>,
    build_information: &mut BTreeMap<String, String>,
) {
    build_information.insert(COMMAND.into(), command);
    build_information.insert(LATENCY.into(), latency.as_millis().to_string());
    build_information.insert(SUCCESS.into(), success.to_string());
    utils::insert_optional_value(
        build_information,
        ERROR,
        error.map(|inner| inner.to_string()),
    );
}
