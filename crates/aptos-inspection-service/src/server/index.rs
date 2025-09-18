// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    server::utils::CONTENT_TYPE_TEXT, CONFIGURATION_PATH, CONSENSUS_HEALTH_CHECK_PATH,
    FORGE_METRICS_PATH, IDENTITY_INFORMATION_PATH, JSON_METRICS_PATH, METRICS_PATH,
    PEER_INFORMATION_PATH, SYSTEM_INFORMATION_PATH,
};
use hyper::{Body, StatusCode};

/// Handles a new index request
pub fn handle_index_request() -> (StatusCode, Body, String) {
    (
        StatusCode::OK,
        Body::from(get_index_response()),
        CONTENT_TYPE_TEXT.into(),
    )
}

/// Returns the response for the index page. The response
/// simply lists a welcome message and all available endpoints.
fn get_index_response() -> String {
    let mut index_response: Vec<String> = Vec::new();

    // Add the list of available endpoints
    index_response.push("Welcome to the Aptos Inspection Service!".into());
    index_response.push("The following endpoints are available:".into());
    index_response.push(format!("\t- {}", CONFIGURATION_PATH));
    index_response.push(format!("\t- {}", CONSENSUS_HEALTH_CHECK_PATH));
    index_response.push(format!("\t- {}", FORGE_METRICS_PATH));
    index_response.push(format!("\t- {}", IDENTITY_INFORMATION_PATH));
    index_response.push(format!("\t- {}", JSON_METRICS_PATH));
    index_response.push(format!("\t- {}", METRICS_PATH));
    index_response.push(format!("\t- {}", PEER_INFORMATION_PATH));
    index_response.push(format!("\t- {}", SYSTEM_INFORMATION_PATH));

    index_response.join("\n") // Separate each entry with a newline
}
