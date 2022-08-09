// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::param::{Param, TransactionVersionParam};

use aptos_api_types::{Error, TransactionId, U64};

use anyhow::Result;
use serde::Deserialize;
use std::num::NonZeroU16;

const DEFAULT_PAGE_SIZE: u16 = 25;
const MAX_PAGE_SIZE: u16 = 1000;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Page {
    start: Option<TransactionVersionParam>,
    limit: Option<Param<NonZeroU16>>,
}

impl Page {
    pub fn compute_start(&self, limit: u16, max: u64) -> Result<u64, Error> {
        let last_page_start = max.saturating_sub((limit.saturating_sub(1)) as u64);
        self.start(last_page_start, max)
    }

    pub fn start(&self, default: u64, max: u64) -> Result<u64, Error> {
        let version = self
            .start
            .clone()
            .map(|v| v.parse("start"))
            .unwrap_or_else(|| Ok(default))?;
        if version > max {
            return Err(Error::not_found(
                "transaction",
                TransactionId::Version(U64::from(version)),
                max,
            ));
        }
        Ok(version)
    }

    pub fn start_option(&self) -> Result<Option<u64>, Error> {
        if let Some(start) = self.start.clone() {
            let version = start.parse("start")?;
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    pub fn limit(&self) -> Result<u16, Error> {
        let limit = self
            .limit
            .clone()
            .map(|v| v.parse("limit"))
            .unwrap_or_else(|| Ok(NonZeroU16::new(DEFAULT_PAGE_SIZE).unwrap()))?
            .get();
        if limit > MAX_PAGE_SIZE {
            return Err(Error::invalid_param(
                "limit",
                format!("{}, exceed limit {}", limit, MAX_PAGE_SIZE),
            ));
        }
        Ok(limit)
    }
}
