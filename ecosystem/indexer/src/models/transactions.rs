// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::database::PgPoolConnection;
use crate::{
    models::events::{Event, EventModel},
    schema::{block_metadata_transactions, transactions, user_transactions},
};
use aptos_rest_client::aptos_api_types::{
    BlockMetadataTransaction as APIBlockMetadataTransaction, Transaction as APITransaction,
    TransactionInfo, UserTransaction as APIUserTransaction, U64,
};
use diesel::RunQueryDsl;
use futures::future::Either;

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = "transactions")]
pub struct Transaction {
    #[diesel(column_name = type)]
    pub type_: String,
    pub payload: serde_json::Value,
    pub version: i64,
    pub hash: String,
    pub state_root_hash: String,
    pub event_root_hash: String,
    pub gas_used: i64,
    pub success: bool,
    pub vm_status: String,
    pub accumulator_root_hash: String,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Transaction {
    pub fn get_by_hash(transaction_hash: String, connection: &PgPoolConnection) {
        let transactions = transactions::table.load::<Transaction>(connection)?;
        let user_transactions = UserTransaction::belonging_to(&transactions)
            .load::<UserTransaction>(&connection)?
            .grouped_by(&users);
        let block_metadata_transactions = BlockMetadataTransaction::belonging_to(&transactions)
            .load::<BlockMetadataTransaction>(&connection)?
            .grouped_by(&users);
        (transactions, user_transactions, block_metadata_transactions)
    }

    pub fn get_by_version(version: u64) {}

    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> (
        Transaction,
        Option<Either<UserTransaction, BlockMetadataTransaction>>,
        Option<Vec<Event>>,
    ) {
        match transaction {
            APITransaction::UserTransaction(tx) => (
                Self::from_transaction_info(
                    &tx.info,
                    serde_json::to_value(&tx.request.payload).unwrap(),
                    transaction.type_str().to_string(),
                ),
                Some(Either::Left(UserTransaction::from_transaction(tx))),
                EventModel::from_events(tx.info.hash.to_string(), &tx.events),
            ),
            APITransaction::GenesisTransaction(tx) => (
                Self::from_transaction_info(
                    &tx.info,
                    serde_json::to_value(&tx.payload).unwrap(),
                    transaction.type_str().to_string(),
                ),
                None,
                EventModel::from_events(tx.info.hash.to_string(), &tx.events),
            ),
            APITransaction::BlockMetadataTransaction(tx) => (
                Self::from_transaction_info(
                    &tx.info,
                    serde_json::Value::Null,
                    transaction.type_str().to_string(),
                ),
                Some(Either::Right(BlockMetadataTransaction::from_transaction(
                    tx,
                ))),
                None,
            ),
            _ => unreachable!(),
        }
    }

    fn from_transaction_info(
        info: &TransactionInfo,
        payload: serde_json::Value,
        type_: String,
    ) -> Self {
        Self {
            type_,
            payload,
            version: *info.version.inner() as i64,
            hash: info.hash.to_string(),
            state_root_hash: info.state_root_hash.to_string(),
            event_root_hash: info.event_root_hash.to_string(),
            gas_used: *info.gas_used.inner() as i64,
            success: info.success,
            vm_status: info.vm_status.clone(),
            accumulator_root_hash: info.accumulator_root_hash.to_string(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset, Associations)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[diesel(table_name = "user_transactions")]
pub struct UserTransaction {
    pub hash: String,
    pub signature: serde_json::Value,
    pub sender: String,
    pub sequence_number: i64,
    pub max_gas_amount: i64,

    // from UserTransactionRequest
    pub expiration_timestamp_secs: chrono::NaiveDateTime,

    // from UserTransaction
    pub timestamp: chrono::NaiveDateTime,

    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl UserTransaction {
    pub fn from_transaction(tx: &APIUserTransaction) -> Self {
        Self {
            hash: tx.info.hash.to_string(),
            signature: serde_json::to_value(&tx.request.signature).unwrap(),
            sender: tx.request.sender.inner().to_hex_literal(),
            sequence_number: *tx.request.sequence_number.inner() as i64,
            max_gas_amount: *tx.request.max_gas_amount.inner() as i64,
            expiration_timestamp_secs: chrono::NaiveDateTime::from_timestamp(
                *tx.request.expiration_timestamp_secs.inner() as i64,
                0,
            ),
            timestamp: parse_timestamp(tx.timestamp, tx.info.version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset, Associations)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[diesel(table_name = "block_metadata_transactions")]
pub struct BlockMetadataTransaction {
    pub hash: String,
    pub id: String,
    pub round: i64,
    pub previous_block_votes: serde_json::Value,
    pub proposer: String,
    pub timestamp: chrono::NaiveDateTime,

    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl BlockMetadataTransaction {
    pub fn from_transaction(tx: &APIBlockMetadataTransaction) -> Self {
        Self {
            hash: tx.info.hash.to_string(),
            id: tx.id.to_string(),
            round: *tx.round.inner() as i64,
            previous_block_votes: serde_json::to_value(&tx.previous_block_votes).unwrap(),
            proposer: tx.proposer.inner().to_hex_literal(),
            // time is in milliseconds, but chronos wants seconds
            timestamp: parse_timestamp(tx.timestamp, tx.info.version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

fn parse_timestamp(ts: U64, version: U64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(*ts.inner() as i64 / 1000000, 0)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

// Prevent conflicts with other things named `Transaction`
pub type BlockMetadataTransactionModel = BlockMetadataTransaction;
pub type TransactionModel = Transaction;
pub type UserTransactionModel = UserTransaction;
