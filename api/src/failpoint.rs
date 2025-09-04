// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use crate::response::InternalError;
use anyhow::{format_err, Result};
use velor_api_types::VelorErrorCode;
use poem_openapi::payload::Json;

/// Build a failpoint to intentionally crash an API for testing
#[allow(unused_variables)]
#[inline]
pub fn fail_point_poem<E: InternalError>(name: &str) -> Result<(), E> {
    fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(E::internal_with_code_no_info(
            format!("Failpoint unexpected internal error for {}", name),
            VelorErrorCode::InternalError,
        ))
    });

    Ok(())
}
