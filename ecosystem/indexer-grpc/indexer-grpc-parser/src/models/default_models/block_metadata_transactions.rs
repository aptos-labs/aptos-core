// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::transactions::{Transaction, TransactionQuery};
use crate::{
    schema::block_metadata_transactions,
    utils::util::{parse_timestamp, standardize_address},
};
use aptos_protos::{
    transaction::testing1::v1::BlockMetadataTransaction as BlockMetadataTransactionPB,
    util::timestamp::Timestamp,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
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
}

/// Need a separate struct for queryable because we don't want to define the inserted_at column (letting DB fill)
#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Queryable, Serialize,
)]
#[diesel(belongs_to(TransactionQuery, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = block_metadata_transactions)]
pub struct BlockMetadataTransactionQuery {
    pub version: i64,
    pub block_height: i64,
    pub id: String,
    pub round: i64,
    pub epoch: i64,
    pub previous_block_votes_bitvec: serde_json::Value,
    pub proposer: String,
    pub failed_proposer_indices: serde_json::Value,
    pub timestamp: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl BlockMetadataTransaction {
    pub fn from_transaction(
        txn: &BlockMetadataTransactionPB,
        version: i64,
        block_height: i64,
        epoch: i64,
        timestamp: &Timestamp,
    ) -> Self {
        Self {
            version,
            block_height,
            id: txn.id.to_string(),
            epoch,
            round: txn.round as i64,
            proposer: standardize_address(txn.proposer.as_str()),
            failed_proposer_indices: serde_json::to_value(&txn.failed_proposer_indices).unwrap(),
            previous_block_votes_bitvec: serde_json::to_value(&txn.previous_block_votes_bitvec)
                .unwrap(),
            // time is in microseconds
            timestamp: parse_timestamp(timestamp, version),
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type BlockMetadataTransactionModel = BlockMetadataTransaction;
