// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{signatures::Signature, write_set_changes::WriteSetChangeDetail};
use crate::{
    models::{events::EventModel, write_set_changes::WriteSetChangeModel},
    schema::{block_metadata_transactions, transactions, user_transactions},
    util::u64_to_bigdecimal,
};
use aptos_protos::block_output::v1::{
    transaction_output::TxnData, BlockMetadataTransactionOutput, TransactionInfoOutput,
    TransactionOutput, UserTransactionOutput,
};
use aptos_protos::util::timestamp::Timestamp;
use aptos_rest_client::aptos_api_types::HexEncodedBytes;
use field_count::FieldCount;
use serde::Serialize;

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(version)]
#[diesel(table_name = "transactions")]
pub struct Transaction {
    pub version: i64,
    pub block_height: i64,
    pub hash: String,
    #[diesel(column_name = type)]
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
    fn from_transaction(
        transaction: &TransactionOutput,
    ) -> (
        Self,
        Option<TransactionDetail>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
        Vec<WriteSetChangeDetail>,
    ) {
        let transaction_info = transaction.transaction_info_output.as_ref().unwrap();
        let events = EventModel::from_events(&transaction.events, transaction_info.block_height);
        let (write_set_changes, wsc_details) = WriteSetChangeModel::from_write_set_changes(
            &transaction.write_set_changes,
            transaction_info.block_height,
        );
        let mut payload: Option<serde_json::Value> = None;
        let mut txn_details = None;
        if let Some(txn_data) = &transaction.txn_data {
            // TODO: add payload handling for genesis which requires adding genesis to option and user transaction
            match txn_data {
                TxnData::BlockMetadata(bm) => {
                    txn_details = Some(TransactionDetail::BlockMetadata(
                        BlockMetadataTransaction::from_transaction(
                            bm,
                            transaction_info.block_height,
                        ),
                    ));
                }
                TxnData::User(user) => {
                    let (user_txn, signatures) =
                        UserTransaction::from_transaction(user, transaction_info.block_height);
                    txn_details = Some(TransactionDetail::User(user_txn, signatures));
                    payload = Some(serde_json::from_str(&user.payload).unwrap_or_default());
                }
                TxnData::Genesis(genesis) => {
                    payload = Some(serde_json::from_str(&genesis.payload).unwrap_or_default());
                }
            }
        }
        let txn = Self::from_transaction_info(
            transaction_info,
            payload.map(|payload| serde_json::to_value(&payload).unwrap()),
            transaction_info.r#type.clone(),
            events.len(),
            write_set_changes.len(),
        );
        (txn, txn_details, events, write_set_changes, wsc_details)
    }

    fn from_transaction_info(
        info: &TransactionInfoOutput,
        payload: Option<serde_json::Value>,
        type_: String,
        num_events: usize,
        num_write_set_changes: usize,
    ) -> Self {
        Self {
            version: info.version as i64,
            block_height: info.block_height as i64,
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
            num_events: num_events as i64,
            num_write_set_changes: num_write_set_changes as i64,
        }
    }

    pub fn from_transactions(
        transactions: &[TransactionOutput],
    ) -> (
        Vec<Self>,
        Vec<TransactionDetail>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
        Vec<WriteSetChangeDetail>,
    ) {
        let mut txns = vec![];
        let mut txn_details = vec![];
        let mut events = vec![];
        let mut wscs = vec![];
        let mut wsc_details = vec![];
        for (txn, txn_detail, mut event_list, mut wsc_list, mut wsc_detail_list) in
            transactions.iter().map(Self::from_transaction)
        {
            txns.push(txn);
            if let Some(a) = txn_detail {
                txn_details.push(a);
            }
            events.append(&mut event_list);
            wscs.append(&mut wsc_list);
            wsc_details.append(&mut wsc_detail_list);
        }
        (txns, txn_details, events, wscs, wsc_details)
    }
}

pub enum TransactionDetail {
    User(UserTransaction, Vec<Signature>),
    BlockMetadata(BlockMetadataTransaction),
}

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "version")]
#[primary_key(version)]
#[diesel(table_name = "user_transactions")]
pub struct UserTransaction {
    pub version: i64,
    pub block_height: i64,
    pub parent_signature_type: String,
    pub sender: String,
    pub sequence_number: i64,
    pub max_gas_amount: bigdecimal::BigDecimal,
    pub expiration_timestamp_secs: chrono::NaiveDateTime,
    pub gas_unit_price: bigdecimal::BigDecimal,
    pub timestamp: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
    pub entry_function_id_str: String,
}

impl UserTransaction {
    pub fn from_transaction(
        txn: &UserTransactionOutput,
        block_height: u64,
    ) -> (Self, Vec<Signature>) {
        (
            Self {
                version: txn.version as i64,
                block_height: block_height as i64,
                parent_signature_type: txn.parent_signature_type.clone(),
                sender: txn.sender.clone(),
                sequence_number: txn.sequence_number as i64,
                max_gas_amount: u64_to_bigdecimal(txn.max_gas_amount),
                expiration_timestamp_secs: parse_proto_timestamp(
                    txn.expiration_timestamp_secs
                        .as_ref()
                        .expect("expiration timestamp must be there"),
                    txn.version,
                ),
                gas_unit_price: u64_to_bigdecimal(txn.gas_unit_price),
                timestamp: parse_proto_timestamp(
                    txn.timestamp.as_ref().expect("timestamp must be there"),
                    txn.version,
                ),
                inserted_at: chrono::Utc::now().naive_utc(),
                entry_function_id_str: txn.entry_function_id_str.clone(),
            },
            Signature::from_signatures(&txn.signatures, block_height),
        )
    }
}

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
    pub fn from_transaction(txn: &BlockMetadataTransactionOutput, block_height: u64) -> Self {
        Self {
            version: txn.version as i64,
            block_height: block_height as i64,
            id: txn.id.clone(),
            epoch: txn.epoch as i64,
            round: txn.round as i64,
            proposer: txn.proposer.clone(),
            failed_proposer_indices: serde_json::to_value(&txn.failed_proposer_indices).unwrap(),
            previous_block_votes_bitvec: serde_json::to_value(&txn.previous_block_votes_bitvec)
                .unwrap(),
            // time is in milliseconds, but chronos wants seconds
            timestamp: parse_proto_timestamp(
                txn.timestamp.as_ref().expect("timestamp must be there"),
                txn.version,
            ),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

// Prevent conflicts with other things named `Transaction`
pub type BlockMetadataTransactionModel = BlockMetadataTransaction;
pub type TransactionModel = Transaction;
pub type UserTransactionModel = UserTransaction;

fn parse_proto_timestamp(ts: &Timestamp, version: u64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(
        std::cmp::min(ts.seconds, chrono::NaiveDateTime::MAX.timestamp()),
        ts.nanos as u32,
    )
    .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}
