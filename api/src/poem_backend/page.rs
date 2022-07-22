// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{response::AptosInternalResult, AptosError, AptosErrorCode, AptosErrorResponse};
use poem_openapi::payload::Json;
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
}

impl Page {
    pub fn start(&self, default: u64, max: u64) -> AptosInternalResult<u64> {
        let start = self.start.unwrap_or(default);
        if start > max {
            return Err(AptosErrorResponse::BadRequest(Json(
                AptosError::new(
                    anyhow::format_err!(
                        "Given start value ({}) is too large, it must be < {}",
                        start,
                        max
                    )
                    .to_string(),
                )
                .error_code(AptosErrorCode::InvalidBcsInStorageError),
            )));
        }
        Ok(start)
    }

    pub fn limit(&self) -> AptosInternalResult<u16> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        if limit == 0 {
            return Err(AptosErrorResponse::BadRequest(Json(AptosError::new(
                anyhow::format_err!("Given limit value ({}) must not be zero", limit,).to_string(),
            ))));
        }
        if limit > MAX_PAGE_SIZE {
            return Err(AptosErrorResponse::BadRequest(Json(AptosError::new(
                anyhow::format_err!(
                    "Given limit value ({}) is too large, it must be < {}",
                    limit,
                    MAX_PAGE_SIZE
                )
                .to_string(),
            ))));
        }
        Ok(limit)
    }
}
