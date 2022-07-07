// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{service, service::TelemetryEvent, utils};
use aptos_logger::error;
use std::{collections::BTreeMap, time::Duration};

/// CLI metrics event name
const APTOS_CLI_METRICS: &str = "APTOS_CLI_METRICS";

/// Core metric keys
const COMMAND: &str = "command";
const LATENCY: &str = "latency";
const SUCCESS: &str = "success";
const ERROR: &str = "error";

/// Collects and sends the build information via telemetry
pub async fn send_cli_telemetry_event(
    mut cli_information: BTreeMap<String, String>,
    command: String,
    latency: Duration,
    success: bool,
    error: Option<String>,
) {
    println!("{:?}", cli_information);
    // Collection information about the cli command
    cli_information.insert(COMMAND.into(), command);
    cli_information.insert(LATENCY.into(), latency.as_millis().to_string());
    cli_information.insert(SUCCESS.into(), success.to_string());
    utils::insert_optional_value(&mut cli_information, ERROR, error);

    // Create a new telemetry event
    let telemetry_event = TelemetryEvent {
        name: APTOS_CLI_METRICS.into(),
        params: cli_information,
    };

    // TODO(joshlind): can we find a better way of identifying each CLI user?
    let user_id = uuid::Uuid::new_v4().to_string();

    // Send the event (we block on the join handle to ensure the
    // event is processed before terminating the cli command).
    let join_handle = service::send_telemetry_event_with_ip(user_id, telemetry_event).await;
    if let Err(error) = join_handle.await {
        error!(
            "Failed to send telemetry event with join error: {:?}",
            error
        );
    }
}
