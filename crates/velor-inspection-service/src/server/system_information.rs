// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT};
use velor_build_info::build_information;
use velor_config::config::NodeConfig;
use hyper::{Body, StatusCode};

// The message to display when the system information endpoint is disabled
pub const SYS_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_system_information: true";

/// Handles a new system information request
pub fn handle_system_information_request(node_config: NodeConfig) -> (StatusCode, Body, String) {
    // Only return system information if the endpoint is enabled
    if node_config.inspection_service.expose_system_information {
        (
            StatusCode::OK,
            Body::from(get_system_information_json()),
            CONTENT_TYPE_JSON.into(),
        )
    } else {
        (
            StatusCode::FORBIDDEN,
            Body::from(SYS_INFO_DISABLED_MESSAGE),
            CONTENT_TYPE_TEXT.into(),
        )
    }
}

/// Returns a simple JSON formatted string with system information
fn get_system_information_json() -> String {
    // Get the system and build information
    let mut system_information = velor_telemetry::system_information::get_system_information();
    system_information.extend(build_information!());

    // Return the system information as a JSON string
    match serde_json::to_string(&system_information) {
        Ok(system_information) => system_information,
        Err(error) => format!("Failed to get system information! Error: {}", error),
    }
}
