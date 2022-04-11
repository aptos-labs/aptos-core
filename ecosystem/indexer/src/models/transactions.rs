// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::unused_unit)]

use crate::{
    database::PgPoolConnection,
    models::events::{Event, EventModel},
    schema::{block_metadata_transactions, transactions, user_transactions},
};
use aptos_rest_client::aptos_api_types::{
    BlockMetadataTransaction as APIBlockMetadataTransaction, Transaction as APITransaction,
    TransactionInfo, UserTransaction as APIUserTransaction, U64,
};
use diesel::{
    BelongingToDsl, ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl,
};
use futures::future::Either;
use serde::Serialize;

#[derive(AsChangeset, Debug, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(hash)]
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
    pub fn get_many_by_version(
        start_version: i64,
        number_to_get: i64,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<
        Vec<(
            Transaction,
            Option<UserTransaction>,
            Option<BlockMetadataTransaction>,
            Vec<EventModel>,
        )>,
    > {
        let mut txs = transactions::table
            .filter(transactions::version.ge(start_version))
            .order(transactions::version.asc())
            .limit(number_to_get)
            .load::<Transaction>(connection)?;

        let mut user_transactions: Vec<Vec<UserTransaction>> = UserTransaction::belonging_to(&txs)
            .load::<UserTransaction>(connection)?
            .grouped_by(&txs);

        let mut block_metadata_transactions: Vec<Vec<BlockMetadataTransaction>> =
            BlockMetadataTransaction::belonging_to(&txs)
                .load::<BlockMetadataTransaction>(connection)?
                .grouped_by(&txs);

        let mut events: Vec<Vec<EventModel>> = EventModel::belonging_to(&txs)
            .load::<EventModel>(connection)?
            .grouped_by(&txs);

        // Convert to the nice result tuple
        let mut result = vec![];
        while !txs.is_empty() {
            result.push((
                txs.pop().unwrap(),
                user_transactions.pop().unwrap().pop(),
                block_metadata_transactions.pop().unwrap().pop(),
                events.pop().unwrap(),
            ))
        }

        Ok(result)
    }

    pub fn get_by_version(
        version: u64,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<(
        Transaction,
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::version.eq(version as i64))
            .first::<Transaction>(connection)?;

        let (user_transaction, block_metadata_transaction, events) =
            transaction.get_details_for_transaction(connection)?;

        Ok((
            transaction,
            user_transaction,
            block_metadata_transaction,
            events,
        ))
    }

    pub fn get_by_hash(
        transaction_hash: &str,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<(
        Transaction,
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::hash.eq(&transaction_hash))
            .first::<Transaction>(connection)?;

        let (user_transaction, block_metadata_transaction, events) =
            transaction.get_details_for_transaction(connection)?;

        Ok((
            transaction,
            user_transaction,
            block_metadata_transaction,
            events,
        ))
    }

    fn get_details_for_transaction(
        &self,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<(
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
    )> {
        let mut user_transaction: Option<UserTransaction> = None;
        let mut block_metadata_transaction: Option<BlockMetadataTransaction> = None;

        let events = crate::schema::events::table
            .filter(crate::schema::events::transaction_hash.eq(&self.hash))
            .load::<EventModel>(connection)?;

        match self.type_.as_str() {
            "user_transaction" => {
                user_transaction = user_transactions::table
                    .filter(user_transactions::hash.eq(&self.hash))
                    .first::<UserTransaction>(connection)
                    .optional()?;
            }
            "block_metadata_transaction" => {
                block_metadata_transaction = block_metadata_transactions::table
                    .filter(block_metadata_transactions::hash.eq(&self.hash))
                    .first::<BlockMetadataTransaction>(connection)
                    .optional()?;
            }
            "genesis_transaction" => {}
            _ => unreachable!("Unknown transaction type: {}", &self.type_),
        };
        Ok((user_transaction, block_metadata_transaction, events))
    }

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

#[derive(AsChangeset, Associations, Debug, Identifiable, Insertable, Queryable, Serialize)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[primary_key(hash)]
#[diesel(table_name = "user_transactions")]
pub struct UserTransaction {
    pub hash: String,
    pub signature: serde_json::Value,
    pub sender: String,
    pub sequence_number: i64,
    pub max_gas_amount: i64,

    // from UserTransactionRequest
    pub expiration_timestamp_secs: chrono::NaiveDateTime,
    pub gas_unit_price: i64,

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
            gas_unit_price: *tx.request.gas_unit_price.inner() as i64,
            timestamp: parse_timestamp(tx.timestamp, tx.info.version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(AsChangeset, Associations, Debug, Identifiable, Insertable, Queryable, Serialize)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[primary_key("hash")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp(U64::from(1649560602763949), U64::from(1));
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), 2022);
    }
}
