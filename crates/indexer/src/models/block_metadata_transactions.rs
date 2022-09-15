// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::transactions::Transaction;
use crate::{schema::block_metadata_transactions, util::parse_timestamp};
use anyhow::Context;
use aptos_api_types::{
    BlockMetadataTransaction as APIBlockMetadataTransaction, Transaction as APITransaction, U64,
};
use field_count::FieldCount;
use serde::Serialize;

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "version")]
#[primary_key("version")]
#[diesel(table_name = "block_metadata_transactions")]
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
        let txn_version = *txn.info.version.inner() as i64;
        Self {
            version: txn_version,
            block_height,
            id: txn.id.to_string(),
            epoch: *txn.epoch.inner() as i64,
            round: *txn.round.inner() as i64,
            proposer: txn.proposer.inner().to_hex_literal(),
            failed_proposer_indices: serde_json::to_value(&txn.failed_proposer_indices).unwrap(),
            previous_block_votes_bitvec: serde_json::to_value(&txn.previous_block_votes_bitvec)
                .unwrap(),
            // time is in microseconds
            timestamp: parse_timestamp(txn.timestamp, txn_version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn get_block_height_from_txn(
        txn: &APITransaction,
        txn_version: U64,
    ) -> anyhow::Result<Option<i64>> {
        if let APITransaction::BlockMetadataTransaction(bmt) = txn {
            for event in &bmt.events {
                if event.typ.to_string() == "0x1::block::NewBlockEvent" {
                    return Ok(Some(
                        event.data["height"]
                            .as_str()
                            .map(|s| s.parse::<i64>())
                            .context(format!(
                                "version {} failed! height missing from event.BlockResource {:?}",
                                txn_version, event.data
                            ))?
                            .context(format!(
                                "version {} failed! failed to parse block height {:?}",
                                txn_version, event.data["height"]
                            ))?,
                    ));
                }
            }
            panic!("Block metadata must contain a 0x1::block::BlockResource event");
        }
        {
            Ok(None)
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type BlockMetadataTransactionModel = BlockMetadataTransaction;
