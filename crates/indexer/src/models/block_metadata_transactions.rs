// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::transactions::Transaction;
use crate::{schema::block_metadata_transactions, util::parse_timestamp};
use aptos_api_types::BlockMetadataTransaction as APIBlockMetadataTransaction;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations,
    Clone,
    Debug,
    Deserialize,
    FieldCount,
    Identifiable,
    Insertable,
    Queryable,
    Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = block_metadata_transactions)]
pub struct BlockMetadataTransaction {
    pub version: i64,
    pub block_height: i64,
    pub id: String,
    pub round: i64,
    pub epoch: i64,
    pub previous_block_votes_bitvec: serde_json::Value,
    pub proposer: String,
    pub failed_proposer_indices: serde_json::Value,
    pub timestamp: chrono::NaiveDateTime,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl BlockMetadataTransaction {
    pub fn from_transaction(txn: &APIBlockMetadataTransaction, block_height: i64) -> Self {
        let txn_version = txn.info.version.0 as i64;
        Self {
            version: txn_version,
            block_height,
            id: txn.id.to_string(),
            epoch: txn.epoch.0 as i64,
            round: txn.round.0 as i64,
            proposer: txn.proposer.inner().to_hex_literal(),
            failed_proposer_indices: serde_json::to_value(&txn.failed_proposer_indices).unwrap(),
            previous_block_votes_bitvec: serde_json::to_value(&txn.previous_block_votes_bitvec)
                .unwrap(),
            // time is in microseconds
            timestamp: parse_timestamp(txn.timestamp, txn_version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type BlockMetadataTransactionModel = BlockMetadataTransaction;
