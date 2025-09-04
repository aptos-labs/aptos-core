// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::CONTENT_TYPE_TEXT;
use velor_config::config::NodeConfig;
use hyper::{Body, StatusCode};

// The message to display when the identity information endpoint is disabled
pub const IDENTITY_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_identity_information: true";

/// Handles a new identity information request
pub fn handle_identity_information_request(node_config: &NodeConfig) -> (StatusCode, Body, String) {
    // Only return identity information if the endpoint is enabled
    let (status_code, body) = if node_config.inspection_service.expose_identity_information {
        let identity_information = get_identity_information(node_config);
        (StatusCode::OK, Body::from(identity_information))
    } else {
        (
            StatusCode::FORBIDDEN,
            Body::from(IDENTITY_INFO_DISABLED_MESSAGE),
        )
    };

    (status_code, body, CONTENT_TYPE_TEXT.into())
}

/// Returns a simple text formatted string with identity information
fn get_identity_information(node_config: &NodeConfig) -> String {
    let mut identity_information = Vec::<String>::new();
    identity_information.push("Identity Information:".into());

    // If the validator network is configured, fetch the identity information
    if let Some(validator_network) = &node_config.validator_network {
        identity_information.push(format!(
            "\t- Validator network ({}), peer ID: {}",
            validator_network.network_id,
            validator_network.peer_id()
        ));
    }

    // For each fullnode network, fetch the identity information
    for fullnode_network in &node_config.full_node_networks {
        identity_information.push(format!(
            "\t- Fullnode network ({}), peer ID: {}",
            fullnode_network.network_id,
            fullnode_network.peer_id()
        ));
    }

    identity_information.join("\n") // Separate each entry with a newline to construct the output
}
