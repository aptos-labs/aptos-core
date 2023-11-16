// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::transactions::{Transaction, TransactionQuery};
use crate::{schema::system_transactions, util::parse_timestamp};
use aptos_api_types::SystemTransaction as APISystemTransaction;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = system_transactions)]
pub struct SystemTransaction {
    pub version: i64,
    pub block_height: i64,
    pub txn_serialized: Vec<u8>,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(TransactionQuery, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = system_transactions)]
pub struct SystemTransactionQuery {
    pub version: i64,
    pub block_height: i64,
    pub txn_serialized: Vec<u8>,
    pub timestamp: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl SystemTransaction {
    pub fn from_transaction(txn: &APISystemTransaction, block_height: i64) -> Self {
        let txn_version = txn.info.version.0 as i64;
        Self {
            version: txn_version,
            block_height,
            timestamp: parse_timestamp(txn.timestamp.0, txn_version),
            txn_serialized: txn.txn_serialized.clone(),
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type SystemTransactionModel = SystemTransaction;
