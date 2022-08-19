// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{HashValue, Transaction, U64};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

// TODO: Consider including this in the API.
// If we do that, change these u64s to U64.

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Block {
    pub block_height: U64,
    pub block_hash: HashValue,
    pub block_timestamp: U64,
    pub first_version: U64,
    pub last_version: U64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<Transaction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct BcsBlock {
    pub block_height: U64,
    pub block_hash: HashValue,
    pub block_timestamp: U64,
    pub first_version: U64,
    pub last_version: U64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<Transaction>>,
}
