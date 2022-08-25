// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    database::PgPoolConnection,
    models::{events::EventModel, write_set_changes::WriteSetChangeModel},
    schema::{block_metadata_transactions, transactions, user_transactions},
    util::u64_to_bigdecimal,
};
use aptos_rest_client::aptos_api_types::{
    Address, BlockMetadataTransaction as APIBlockMetadataTransaction,
    Transaction as APITransaction, TransactionInfo, UserTransaction as APIUserTransaction, U64,
};
use diesel::{
    BelongingToDsl, ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl,
};
use field_count::FieldCount;
use futures::future::Either;
use serde::Serialize;

static SECONDS_IN_10_YEARS: i64 = 60 * 60 * 24 * 365 * 10;

#[derive(AsChangeset, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(hash)]
#[diesel(table_name = "transactions")]
pub struct Transaction {
    #[diesel(column_name = type)]
    pub type_: String,
    pub payload: serde_json::Value,
    pub version: bigdecimal::BigDecimal,
    pub hash: String,
    pub state_root_hash: String,
    pub event_root_hash: String,
    pub gas_used: bigdecimal::BigDecimal,
    pub success: bool,
    pub vm_status: String,
    pub accumulator_root_hash: String,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Transaction {
    pub fn get_many_by_version(
        start_version: u64,
        number_to_get: i64,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<
        Vec<(
            Transaction,
            Option<UserTransaction>,
            Option<BlockMetadataTransaction>,
            Vec<EventModel>,
            Vec<WriteSetChangeModel>,
        )>,
    > {
        let mut txs = transactions::table
            .filter(transactions::version.ge(u64_to_bigdecimal(start_version)))
            .order(transactions::version.asc())
            .limit(number_to_get as i64)
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

        let mut write_set_changes: Vec<Vec<WriteSetChangeModel>> =
            WriteSetChangeModel::belonging_to(&txs)
                .load::<WriteSetChangeModel>(connection)?
                .grouped_by(&txs);

        // Convert to the nice result tuple
        let mut result = vec![];
        while !txs.is_empty() {
            result.push((
                txs.pop().unwrap(),
                user_transactions.pop().unwrap().pop(),
                block_metadata_transactions.pop().unwrap().pop(),
                events.pop().unwrap(),
                write_set_changes.pop().unwrap(),
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
        Vec<WriteSetChangeModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::version.eq(u64_to_bigdecimal(version)))
            .first::<Transaction>(connection)?;

        let (user_transaction, block_metadata_transaction, events, write_set_changes) =
            transaction.get_details_for_transaction(connection)?;

        Ok((
            transaction,
            user_transaction,
            block_metadata_transaction,
            events,
            write_set_changes,
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
        Vec<WriteSetChangeModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::hash.eq(&transaction_hash))
            .first::<Transaction>(connection)?;

        let (user_transaction, block_metadata_transaction, events, write_set_changes) =
            transaction.get_details_for_transaction(connection)?;

        Ok((
            transaction,
            user_transaction,
            block_metadata_transaction,
            events,
            write_set_changes,
        ))
    }

    fn get_details_for_transaction(
        &self,
        connection: &PgPoolConnection,
    ) -> diesel::QueryResult<(
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
    )> {
        let mut user_transaction: Option<UserTransaction> = None;
        let mut block_metadata_transaction: Option<BlockMetadataTransaction> = None;

        let events = crate::schema::events::table
            .filter(crate::schema::events::transaction_hash.eq(&self.hash))
            .load::<EventModel>(connection)?;

        let write_set_changes = crate::schema::write_set_changes::table
            .filter(crate::schema::write_set_changes::transaction_hash.eq(&self.hash))
            .load::<WriteSetChangeModel>(connection)?;

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
            "state_checkpoint_transaction" => {}
            _ => unreachable!("Unknown transaction type: {}", &self.type_),
        };
        Ok((
            user_transaction,
            block_metadata_transaction,
            events,
            write_set_changes,
        ))
    }

    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> (
        Transaction,
        Option<Either<UserTransaction, BlockMetadataTransaction>>,
        Option<Vec<EventModel>>,
        Option<Vec<WriteSetChangeModel>>,
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
                WriteSetChangeModel::from_write_set_changes(
                    tx.info.hash.to_string(),
                    &tx.info.changes,
                ),
            ),
            APITransaction::GenesisTransaction(tx) => (
                Self::from_transaction_info(
                    &tx.info,
                    serde_json::to_value(&tx.payload).unwrap(),
                    transaction.type_str().to_string(),
                ),
                None,
                EventModel::from_events(tx.info.hash.to_string(), &tx.events),
                WriteSetChangeModel::from_write_set_changes(
                    tx.info.hash.to_string(),
                    &tx.info.changes,
                ),
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
                EventModel::from_events(tx.info.hash.to_string(), &tx.events),
                WriteSetChangeModel::from_write_set_changes(
                    tx.info.hash.to_string(),
                    &tx.info.changes,
                ),
            ),
            APITransaction::StateCheckpointTransaction(tx) => (
                Self::from_transaction_info(
                    &tx.info,
                    serde_json::Value::Null,
                    transaction.type_str().to_string(),
                ),
                None,
                None,
                None,
            ),
            APITransaction::PendingTransaction(..) => {
                unreachable!()
            }
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
            version: u64_to_bigdecimal(*info.version.inner()),
            hash: info.hash.to_string(),
            state_root_hash: info.state_root_hash.to_string(),
            event_root_hash: info.event_root_hash.to_string(),
            gas_used: u64_to_bigdecimal(*info.gas_used.inner()),
            success: info.success,
            vm_status: info.vm_status.clone(),
            accumulator_root_hash: info.accumulator_root_hash.to_string(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_transactions(
        transactions: &[APITransaction],
    ) -> (
        Vec<Self>,
        Vec<UserTransaction>,
        Vec<BlockMetadataTransaction>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
    ) {
        let mut txns = vec![];
        let mut user_txns = vec![];
        let mut bm_txns = vec![];
        let mut events = vec![];
        let mut wscs = vec![];
        for (txn, user_or_bmt, maybe_event_list, maybe_wsc_list) in
            transactions.iter().map(Self::from_transaction)
        {
            txns.push(txn);
            match user_or_bmt {
                Some(Either::Left(user_transaction_model)) => {
                    user_txns.push(user_transaction_model);
                }
                Some(Either::Right(bmt_model)) => {
                    bm_txns.push(bmt_model);
                }
                _ => (),
            }
            if let Some(mut event_list) = maybe_event_list {
                events.append(&mut event_list);
            }
            if let Some(mut wsc_list) = maybe_wsc_list {
                wscs.append(&mut wsc_list);
            }
        }
        (txns, user_txns, bm_txns, events, wscs)
    }
}

#[derive(
    AsChangeset, Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[primary_key(hash)]
#[diesel(table_name = "user_transactions")]
pub struct UserTransaction {
    pub hash: String,
    pub signature: serde_json::Value,
    pub sender: String,
    pub sequence_number: bigdecimal::BigDecimal,
    pub max_gas_amount: bigdecimal::BigDecimal,

    // from UserTransactionRequest
    pub expiration_timestamp_secs: chrono::NaiveDateTime,
    pub gas_unit_price: bigdecimal::BigDecimal,

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
            sequence_number: u64_to_bigdecimal(tx.request.sequence_number.0),
            max_gas_amount: u64_to_bigdecimal(tx.request.max_gas_amount.0),
            expiration_timestamp_secs: parse_timestamp_secs(
                tx.request.expiration_timestamp_secs,
                tx.info.version,
            ),
            gas_unit_price: u64_to_bigdecimal(tx.request.gas_unit_price.0),
            timestamp: parse_timestamp(tx.timestamp, tx.info.version),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(
    AsChangeset, Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "hash")]
#[primary_key("hash")]
#[diesel(table_name = "block_metadata_transactions")]
pub struct BlockMetadataTransaction {
    pub hash: String,
    pub id: String,
    pub round: bigdecimal::BigDecimal,
    pub previous_block_votes: serde_json::Value,
    pub proposer: String,
    pub timestamp: chrono::NaiveDateTime,

    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
    pub epoch: bigdecimal::BigDecimal,
    pub previous_block_votes_bitvec: serde_json::Value,
    pub failed_proposer_indices: serde_json::Value,
}

impl BlockMetadataTransaction {
    pub fn from_transaction(tx: &APIBlockMetadataTransaction) -> Self {
        Self {
            hash: tx.info.hash.to_string(),
            id: tx.id.to_string(),
            round: u64_to_bigdecimal(tx.round.0),
            // TODO: Deprecated, use previous_block_votes_bitmap instead. Column kept to not break indexer users (e.g., explorer), writing an empty vector.
            previous_block_votes: serde_json::to_value(vec![] as Vec<Address>).unwrap(),
            proposer: tx.proposer.inner().to_hex_literal(),
            // time is in milliseconds, but chronos wants seconds
            timestamp: parse_timestamp(tx.timestamp, tx.info.version),
            inserted_at: chrono::Utc::now().naive_utc(),
            epoch: u64_to_bigdecimal(tx.epoch.0),
            previous_block_votes_bitvec: serde_json::to_value(&tx.previous_block_votes_bitvec)
                .unwrap(),
            failed_proposer_indices: serde_json::to_value(&tx.failed_proposer_indices).unwrap(),
        }
    }
}

fn parse_timestamp(ts: U64, version: U64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(*ts.inner() as i64 / 1000000, 0)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

fn parse_timestamp_secs(ts: U64, version: U64) -> chrono::NaiveDateTime {
    let mut timestamp = ts.0 as i64;
    let timestamp_in_10_years = chrono::offset::Utc::now().timestamp() + SECONDS_IN_10_YEARS;
    if timestamp > timestamp_in_10_years {
        timestamp = timestamp_in_10_years;
    }
    chrono::NaiveDateTime::from_timestamp_opt(timestamp, 0)
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
        let current_year = chrono::offset::Utc::now().year();

        let ts = parse_timestamp(U64::from(1649560602763949), U64::from(1));
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), current_year);

        let ts2 = parse_timestamp_secs(U64::from(600000000000000), U64::from(2));
        assert_eq!(ts2.year(), current_year + 10);

        let ts3 = parse_timestamp_secs(U64::from(1659386386), U64::from(2));
        assert_eq!(ts3.timestamp(), 1659386386);
    }
}
