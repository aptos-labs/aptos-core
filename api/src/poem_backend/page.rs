// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{AptosErrorCode, BadRequestError};
use serde::Deserialize;

const DEFAULT_PAGE_SIZE: u16 = 25;
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

    pub fn start<E: BadRequestError>(&self, default: u64, max: u64) -> Result<u64, E> {
        let start = self.start.unwrap_or(default);
        if start > max {
            return Err(E::bad_request_str(&format!(
                "Given start value ({}) is higher than the highest ledger version, it must be < {}",
                start, max
            ))
            .error_code(AptosErrorCode::InvalidStartParam));
        }
        Ok(start)
    }

    pub fn limit<E: BadRequestError>(&self) -> Result<u16, E> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        if limit == 0 {
            return Err(E::bad_request_str(&format!(
                "Given limit value ({}) must not be zero",
                limit
            ))
            .error_code(AptosErrorCode::InvalidLimitParam));
        }
        if limit > MAX_PAGE_SIZE {
            return Err(E::bad_request_str(&format!(
                "Given limit value ({}) is too large, it must be < {}",
                limit, MAX_PAGE_SIZE
            ))
            .error_code(AptosErrorCode::InvalidLimitParam));
        }
        Ok(limit)
    }
}
