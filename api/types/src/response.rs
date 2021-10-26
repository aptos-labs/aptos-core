// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Error, LedgerInfo};

use anyhow::Result;
use serde::Serialize;
use warp::http::header::{HeaderValue, CONTENT_TYPE};

pub const X_DIEM_CHAIN_ID: &str = "X-Diem-Chain-Id";
pub const X_DIEM_LEDGER_VERSION: &str = "X-Diem-Ledger-Version";
pub const X_DIEM_LEDGER_TIMESTAMP: &str = "X-Diem-Ledger-TimestampUsec";

pub struct Response {
    pub ledger_info: LedgerInfo,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new<T: Serialize>(ledger_info: LedgerInfo, body: &T) -> Result<Self, Error> {
        Ok(Self {
            ledger_info,
            body: serde_json::to_vec(body)?,
        })
    }
}

impl warp::Reply for Response {
    fn into_response(self) -> warp::reply::Response {
        let mut res = warp::reply::Response::new(self.body.into());
        let headers = res.headers_mut();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(X_DIEM_CHAIN_ID, (self.ledger_info.chain_id as u16).into());
        headers.insert(
            X_DIEM_LEDGER_VERSION,
            self.ledger_info.ledger_version.into(),
        );
        headers.insert(
            X_DIEM_LEDGER_TIMESTAMP,
            self.ledger_info.ledger_timestamp.into(),
        );

        res
    }
}
