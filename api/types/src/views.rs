// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::convert::From;
use warp::reject::Reject;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LedgerInfo {
    pub chain_id: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub ledger_version: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub ledger_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct InternalError {
    message: String,
    data: Option<Value>,
}

impl Reject for InternalError {}

impl From<anyhow::Error> for InternalError {
    fn from(e: anyhow::Error) -> Self {
        Self {
            message: e.to_string(),
            data: None,
        }
    }
}
