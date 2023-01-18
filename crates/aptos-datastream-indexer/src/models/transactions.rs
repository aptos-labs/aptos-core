// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{schema::transactions, util::u64_to_bigdecimal};
use aptos_protos::transaction::v1::{
    transaction::{TransactionType, TxnData},
    Transaction as TransactionProto, TransactionInfo,
};
use aptos_rest_client::aptos_api_types::HexEncodedBytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Insertable, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(version))]
#[diesel(table_name = transactions)]
pub struct Transaction {
    pub version: i64,
    pub block_height: i64,
    pub hash: String,
    pub type_: String,
    pub payload: Option<serde_json::Value>,
    pub state_change_hash: String,
    pub event_root_hash: String,
    pub state_checkpoint_hash: Option<String>,
    pub gas_used: bigdecimal::BigDecimal,
    pub success: bool,
    pub vm_status: String,
    pub accumulator_root_hash: String,
    pub num_events: i64,
    pub num_write_set_changes: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Transaction {
    pub fn from_transaction(transaction: &TransactionProto) -> Self {
        let transaction_info = transaction.info.as_ref().unwrap();
        let mut payload: Option<serde_json::Value> = None;
        let mut num_events = 0;
        let num_write_set_changes = transaction_info.changes.len() as i64;
        if let Some(txn_data) = &transaction.txn_data {
            match txn_data {
                TxnData::BlockMetadata(bm) => {
                    num_events = bm.events.len() as i64;
                    // No payload for BlockMetadata Txn.
                },
                TxnData::User(user) => match &user.request {
                    Some(p) => {
                        if let Some(ref transaction_payload) = p.payload {
                            payload = Some(serde_json::to_value(transaction_payload).unwrap());
                        }
                    },
                    None => {},
                },
                TxnData::Genesis(genesis) => {
                    if let Some(ref transaction_payload) = genesis.payload {
                        payload = Some(serde_json::to_value(transaction_payload).unwrap());
                    }
                    num_events = genesis.events.len() as i64;
                },
                TxnData::StateCheckpoint(_) => {},
            }
        }
        Self::from_transaction_info(
            transaction_info,
            transaction.version as i64,
            transaction.block_height as i64,
            payload.map(|payload| serde_json::to_value(&payload).unwrap()),
            TransactionType::from_i32(transaction.r#type)
                .unwrap()
                .as_str_name()
                .to_string(),
            num_events,
            num_write_set_changes,
        )
    }

    fn from_transaction_info(
        info: &TransactionInfo,
        version: i64,
        bheight: i64,
        payload: Option<serde_json::Value>,
        type_: String,
        number_of_events: i64,
        number_of_write_set_changes: i64,
    ) -> Self {
        Self {
            version,
            block_height: bheight,
            hash: HexEncodedBytes::from(info.hash.clone()).to_string(),
            type_,
            payload,
            state_change_hash: HexEncodedBytes::from(info.state_change_hash.clone()).to_string(),
            event_root_hash: HexEncodedBytes::from(info.event_root_hash.clone()).to_string(),
            state_checkpoint_hash: info
                .state_checkpoint_hash
                .clone()
                .map(|hash| HexEncodedBytes::from(hash).to_string()),
            gas_used: u64_to_bigdecimal(info.gas_used),
            success: info.success,
            vm_status: info.vm_status.clone(),
            accumulator_root_hash: HexEncodedBytes::from(info.accumulator_root_hash.clone())
                .to_string(),
            inserted_at: chrono::Utc::now().naive_utc(),
            num_events: number_of_events,
            num_write_set_changes: number_of_write_set_changes,
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type TransactionModel = Transaction;
