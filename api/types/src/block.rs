// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{HashValue, Transaction, TransactionOnChainData, U64};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcsBlock {
    pub block_height: u64,
    pub block_hash: aptos_crypto::HashValue,
    pub block_timestamp: u64,
    pub first_version: u64,
    pub last_version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TransactionOnChainData>>,
}
