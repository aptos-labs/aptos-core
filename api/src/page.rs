// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::response::BadRequestError;
use aptos_api_types::{AptosErrorCode, LedgerInfo, U64};
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
    pub fn new(start: Option<U64>, limit: Option<u16>) -> Self {
        Self {
            start: start.map(|inner| inner.0),
            limit,
        }
    }

    pub fn get_version_start_param<E: BadRequestError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<u64, E> {
        let start = self.start.unwrap_or(ledger_info.oldest_ledger_version.0);
        if start > ledger_info.ledger_version.0 {
            return Err(E::bad_request_with_code(
                &format!(
                    "Given start value ({}) is higher than the current ledger version, it must be <= {}",
                    start,
                    ledger_info.ledger_version.0,
                ),
                AptosErrorCode::InvalidStartParam,
                ledger_info,
            ));
        }
        // TODO: Also check if the start is in the pruned state? Otherwise, it'll always get the versions
        // that are not pruned
        Ok(start)
    }

    pub fn get_limit_param<E: BadRequestError>(&self, ledger_info: &LedgerInfo) -> Result<u16, E> {
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
                    "Given limit value ({}) is too large, it must be <= {}",
                    limit, MAX_PAGE_SIZE
                ),
                AptosErrorCode::InvalidLimitParam,
                ledger_info,
            ));
        }
        Ok(limit)
    }

    pub fn get_version_params<E: BadRequestError>(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<(u64, u16), E> {
        let start = self.get_version_start_param(ledger_info)?;
        let limit = self.get_limit_param(ledger_info)?;

        Ok((start, limit))
    }

    pub fn start(&self) -> Option<u64> {
        self.start
    }
}
