// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::CONTENT_TYPE_TEXT;
use velor_config::config::NodeConfig;
use hyper::{Body, StatusCode};

// The message to display when the configuration endpoint is disabled
pub const CONFIGURATION_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_configuration: true";

/// Handles a new configuration request
pub fn handle_configuration_request(node_config: &NodeConfig) -> (StatusCode, Body, String) {
    // Only return configuration if the endpoint is enabled
    let (status_code, body) = if node_config.inspection_service.expose_configuration {
        // We format the configuration using debug formatting. This is important to
        // prevent secret/private keys from being serialized and leaked (i.e.,
        // all secret keys are marked with SilentDisplay and SilentDebug).
        let encoded_configuration = format!("{:?}", node_config);
        (StatusCode::OK, Body::from(encoded_configuration))
    } else {
        (
            StatusCode::FORBIDDEN,
            Body::from(CONFIGURATION_DISABLED_MESSAGE),
        )
    };

    (status_code, body, CONTENT_TYPE_TEXT.into())
}
