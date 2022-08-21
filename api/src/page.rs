// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::response::BadRequestError;
use aptos_api_types::{AptosErrorCode, LedgerInfo};
use serde::Deserialize;

const DEFAULT_PAGE_SIZE: u16 = 25;

/// This MAX_PAGE_SIZE must always be smaller than the `aptos_db::MAX_LIMIT` in the DB
const MAX_PAGE_SIZE: u16 = 1000;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Page {
    start: Option<u64>,
    limit: Option<u16>,
}

impl Page {
    pub fn new(start: Option<u64>, limit: Option<u16>) -> Self {
        Self { start, limit }
    }

    pub fn compute_start<E: BadRequestError>(
        &self,
        limit: u16,
        max: u64,
        ledger_info: &LedgerInfo,
    ) -> Result<u64, E> {
        let last_page_start = max.saturating_sub((limit.saturating_sub(1)) as u64);
        self.start(last_page_start, max, ledger_info)
    }

    pub fn start<E: BadRequestError>(
        &self,
        default: u64,
        max: u64,
        ledger_info: &LedgerInfo,
    ) -> Result<u64, E> {
        let start = self.start.unwrap_or(default);
        if start > max {
            return Err(E::bad_request_with_code(
                &format!(
                "Given start value ({}) is higher than the current ledger version, it must be < {}",
                start, max
            ),
                AptosErrorCode::InvalidStartParam,
                ledger_info,
            ));
        }
        Ok(start)
    }

    pub fn start_option(&self) -> Option<u64> {
        self.start
    }

    pub fn limit<E: BadRequestError>(&self, ledger_info: &LedgerInfo) -> Result<u16, E> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        if limit == 0 {
            return Err(E::bad_request_with_code(
                &format!("Given limit value ({}) must not be zero", limit),
                AptosErrorCode::InvalidLimitParam,
                ledger_info,
            ));
        }
        if limit > MAX_PAGE_SIZE {
            return Err(E::bad_request_with_code(
                &format!(
                    "Given limit value ({}) is too large, it must be < {}",
                    limit, MAX_PAGE_SIZE
                ),
                AptosErrorCode::InvalidLimitParam,
                ledger_info,
            ));
        }
        Ok(limit)
    }
}
