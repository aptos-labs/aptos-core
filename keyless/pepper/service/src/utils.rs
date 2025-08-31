// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::PepperServiceError;

/// Attempts to read an environment variable and returns an error if it fails
pub fn read_environment_variable(variable_name: &str) -> Result<String, PepperServiceError> {
    std::env::var(variable_name).map_err(|error| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to read environment variable {}: {}",
            variable_name, error
        ))
    })
}
