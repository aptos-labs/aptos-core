// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api_types::{
    mime_types::{BCS, JSON},
    LedgerInfo, X_APTOS_BLOCK_HEIGHT, X_APTOS_CHAIN_ID, X_APTOS_EPOCH,
    X_APTOS_LEDGER_OLDEST_VERSION, X_APTOS_LEDGER_TIMESTAMP, X_APTOS_LEDGER_VERSION,
    X_APTOS_OLDEST_BLOCK_HEIGHT,
};
use hyper::{header::CONTENT_TYPE, http::HeaderValue};
use serde::Serialize;

pub struct Response {
    pub ledger_info: LedgerInfo,
    pub body: Vec<u8>,
    pub is_bcs_response: bool,
}

impl Response {
    pub fn new<T: Serialize>(ledger_info: LedgerInfo, body: &T) -> Result<Self> {
        Ok(Self {
            ledger_info,
            body: serde_json::to_vec(body)?,
            is_bcs_response: false,
        })
    }

    pub fn new_bcs<T: Serialize>(ledger_info: LedgerInfo, body: &T) -> Result<Self> {
        Ok(Self {
            ledger_info,
            body: bcs::to_bytes(body).unwrap(),
            is_bcs_response: true,
        })
    }
}

impl warp::Reply for Response {
    fn into_response(self) -> warp::reply::Response {
        let mut res = warp::reply::Response::new(self.body.into());
        let headers = res.headers_mut();

        if self.is_bcs_response {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(BCS));
        } else {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON));
        }
        headers.insert(X_APTOS_CHAIN_ID, (self.ledger_info.chain_id as u16).into());
        headers.insert(
            X_APTOS_LEDGER_VERSION,
            self.ledger_info.ledger_version.0.into(),
        );
        headers.insert(
            X_APTOS_LEDGER_OLDEST_VERSION,
            self.ledger_info.oldest_ledger_version.0.into(),
        );
        headers.insert(
            X_APTOS_LEDGER_TIMESTAMP,
            self.ledger_info.ledger_timestamp.0.into(),
        );
        headers.insert(X_APTOS_EPOCH, self.ledger_info.epoch.0.into());
        headers.insert(X_APTOS_BLOCK_HEIGHT, self.ledger_info.block_height.0.into());
        headers.insert(
            X_APTOS_OLDEST_BLOCK_HEIGHT,
            self.ledger_info.oldest_block_height.0.into(),
        );

        res
    }
}
