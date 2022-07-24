// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use anyhow::{format_err, Result};
use aptos_api_types::Error;

use crate::poem_backend::{AptosError, AptosErrorResponse};
use poem_openapi::payload::Json;

#[allow(unused_variables)]
#[inline]
pub fn fail_point(name: &str) -> Result<(), Error> {
    Ok(fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(format_err!("unexpected internal error for {}", name).into())
    }))
}

#[allow(unused_variables)]
#[inline]
pub fn fail_point_poem(name: &str) -> Result<(), AptosErrorResponse> {
    Ok(fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(AptosErrorResponse::InternalServerError(Json(
            AptosError::new(format!("unexpected internal error for {}", name)),
        )))
    }))
}
