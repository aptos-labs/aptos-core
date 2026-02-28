// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(unused_imports)]

use crate::response::InternalError;
use anyhow::{format_err, Result};
use aptos_api_types::AptosErrorCode;

/// Build a failpoint to intentionally crash an API for testing
#[allow(unused_variables)]
#[inline]
pub fn fail_point<E: InternalError>(name: &str) -> Result<(), E> {
    fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(E::internal_with_code_no_info(
            format!("Failpoint unexpected internal error for {}", name),
            AptosErrorCode::InternalError,
        ))
    });

    Ok(())
}
