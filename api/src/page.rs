// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::Error;

use anyhow::{format_err, Result};
use serde::Deserialize;
use std::str::FromStr;

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 1000;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Page {
    start: Option<String>,
    limit: Option<String>,
}

impl Page {
    pub fn start(&self, default: u64) -> Result<u64, Error> {
        parse_param("start", &self.start, default)
    }

    pub fn limit(&self) -> Result<u16, Error> {
        let v = parse_param("limit", &self.limit, DEFAULT_PAGE_SIZE)?;
        if v > MAX_PAGE_SIZE {
            return Err(Error::bad_request(format_err!(
                "invalid parameter: limit={}, exceed limit {}",
                v,
                MAX_PAGE_SIZE
            )));
        }
        Ok(v as u16)
    }
}

fn parse_param<T: FromStr>(
    param_name: &str,
    data: &Option<String>,
    default: T,
) -> Result<T, Error> {
    match data {
        Some(n) => n.parse::<T>().map_err(|_| {
            Error::bad_request(format_err!("invalid parameter: {}={}", param_name, n))
        }),
        None => Ok(default),
    }
}
