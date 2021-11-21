// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#[allow(unused_imports)]
use anyhow::{format_err, Result};
use diem_api_types::Error;

#[allow(unused_variables)]
#[inline]
pub fn fail_point(name: &str) -> Result<(), Error> {
    Ok(fail::fail_point!(format!("api::{}", name).as_str(), |_| {
        Err(format_err!("unexpected internal error for {}", name).into())
    }))
}
