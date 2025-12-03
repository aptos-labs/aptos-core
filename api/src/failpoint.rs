#![allow(unused_imports)] // Copyright (c) Aptos Foundation
                          // Copyright (c) Aptos Foundation
                          // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::response::InternalError;
use anyhow::{format_err, Result};
use aptos_api_types::AptosErrorCode;
use poem_openapi::payload::Json;

/// Build a failpoint to intentionally crash an API for testing
#[allow(unused_variables)]
#[inline]
pub fn fail_point_poem<E: InternalError>(name: &str) -> Result<(), E> {
    fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(E::internal_with_code_no_info(
            format!("Failpoint unexpected internal error for {}", name),
            AptosErrorCode::InternalError,
        ))
    });

    Ok(())
}
