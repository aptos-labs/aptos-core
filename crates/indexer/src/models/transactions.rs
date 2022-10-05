// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    models::{events::EventModel, write_set_changes::WriteSetChangeModel},
    schema::{block_metadata_transactions, transactions, user_transactions},
    util::u64_to_bigdecimal,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

use crate::database::PgPoolConnection;
use aptos_api_types::{Transaction as APITransaction, TransactionInfo};
use bigdecimal::BigDecimal;
use diesel::{
    BelongingToDsl, ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl,
};

use super::{
    block_metadata_transactions::BlockMetadataTransaction, signatures::Signature,
    user_transactions::UserTransaction, write_set_changes::WriteSetChangeDetail,
};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
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
    pub gas_used: BigDecimal,
    pub success: bool,
    pub vm_status: String,
    pub accumulator_root_hash: String,
    pub num_events: i64,
    pub num_write_set_changes: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Transaction {
    fn from_transaction_info(
        info: &TransactionInfo,
        payload: Option<serde_json::Value>,
        type_: String,
        num_events: i64,
        block_height: i64,
    ) -> Self {
        Self {
            type_,
            payload,
            version: info.version.0 as i64,
            block_height,
            hash: info.hash.to_string(),
            state_change_hash: info.state_change_hash.to_string(),
            event_root_hash: info.event_root_hash.to_string(),
            state_checkpoint_hash: info.state_checkpoint_hash.map(|h| h.to_string()),
            gas_used: u64_to_bigdecimal(info.gas_used.0),
            success: info.success,
            vm_status: info.vm_status.clone(),
            accumulator_root_hash: info.accumulator_root_hash.to_string(),
            num_events,
            num_write_set_changes: info.changes.len() as i64,
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> (
        Self,
        Option<TransactionDetail>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
        Vec<WriteSetChangeDetail>,
    ) {
        let block_height = transaction
            .transaction_info()
            .unwrap()
            .block_height
            .unwrap()
            .0 as i64;
        match transaction {
            APITransaction::UserTransaction(user_txn) => {
                let (user_txn_output, signatures) =
                    UserTransaction::from_transaction(user_txn, block_height);
                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &user_txn.info.changes,
                    user_txn.info.version.0 as i64,
                    block_height,
                );
                (
                    Self::from_transaction_info(
                        &user_txn.info,
                        Some(
                            serde_json::to_value(&user_txn.request.payload)
                                .expect("Unable to deserialize transaction payload"),
                        ),
                        transaction.type_str().to_string(),
                        user_txn.events.len() as i64,
                        block_height,
                    ),
                    Some(TransactionDetail::User(user_txn_output, signatures)),
                    EventModel::from_events(
                        &user_txn.events,
                        user_txn.info.version.0 as i64,
                        block_height,
                    ),
                    wsc,
                    wsc_detail,
                )
            }
            APITransaction::GenesisTransaction(genesis_txn) => {
                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &genesis_txn.info.changes,
                    genesis_txn.info.version.0 as i64,
                    block_height,
                );
                (
                    Self::from_transaction_info(
                        &genesis_txn.info,
                        Some(
                            serde_json::to_value(&genesis_txn.payload)
                                .expect("Unable to deserialize Genesis transaction"),
                        ),
                        transaction.type_str().to_string(),
                        0,
                        block_height,
                    ),
                    None,
                    EventModel::from_events(
                        &genesis_txn.events,
                        genesis_txn.info.version.0 as i64,
                        block_height,
                    ),
                    wsc,
                    wsc_detail,
                )
            }
            APITransaction::BlockMetadataTransaction(block_metadata_txn) => {
                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &block_metadata_txn.info.changes,
                    block_metadata_txn.info.version.0 as i64,
                    block_height,
                );
                (
                    Self::from_transaction_info(
                        &block_metadata_txn.info,
                        None,
                        transaction.type_str().to_string(),
                        0,
                        block_height,
                    ),
                    Some(TransactionDetail::BlockMetadata(
                        BlockMetadataTransaction::from_transaction(
                            block_metadata_txn,
                            block_height,
                        ),
                    )),
                    EventModel::from_events(
                        &block_metadata_txn.events,
                        block_metadata_txn.info.version.0 as i64,
                        block_height,
                    ),
                    wsc,
                    wsc_detail,
                )
            }
            APITransaction::StateCheckpointTransaction(state_checkpoint_txn) => (
                Self::from_transaction_info(
                    &state_checkpoint_txn.info,
                    None,
                    transaction.type_str().to_string(),
                    0,
                    block_height,
                ),
                None,
                vec![],
                vec![],
                vec![],
            ),
            APITransaction::PendingTransaction(..) => {
                unreachable!()
            }
        }
    }

    pub fn from_transactions(
        transactions: &[APITransaction],
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

        for txn in transactions {
            let (txn, txn_detail, mut event_list, mut wsc_list, mut wsc_detail_list) =
                Self::from_transaction(txn);
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

    pub fn get_many_by_version(
        start_version: u64,
        number_to_get: i64,
        conn: &mut PgPoolConnection,
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
            .filter(transactions::version.ge(start_version as i64))
            .order(transactions::version.asc())
            .limit(number_to_get as i64)
            .load::<Transaction>(conn)?;

        let mut user_transactions: Vec<Vec<UserTransaction>> = UserTransaction::belonging_to(&txs)
            .load::<UserTransaction>(conn)?
            .grouped_by(&txs);

        let mut block_metadata_transactions: Vec<Vec<BlockMetadataTransaction>> =
            BlockMetadataTransaction::belonging_to(&txs)
                .load::<BlockMetadataTransaction>(conn)?
                .grouped_by(&txs);

        let mut events: Vec<Vec<EventModel>> = EventModel::belonging_to(&txs)
            .load::<EventModel>(conn)?
            .grouped_by(&txs);

        let mut write_set_changes: Vec<Vec<WriteSetChangeModel>> =
            WriteSetChangeModel::belonging_to(&txs)
                .load::<WriteSetChangeModel>(conn)?
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
        conn: &mut PgPoolConnection,
    ) -> diesel::QueryResult<(
        Transaction,
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::version.eq(version as i64))
            .first::<Transaction>(conn)?;

        let (user_transaction, block_metadata_transaction, events, write_set_changes) =
            transaction.get_details_for_transaction(conn)?;

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
        conn: &mut PgPoolConnection,
    ) -> diesel::QueryResult<(
        Transaction,
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
    )> {
        let transaction = transactions::table
            .filter(transactions::hash.eq(&transaction_hash))
            .first::<Transaction>(conn)?;

        let (user_transaction, block_metadata_transaction, events, write_set_changes) =
            transaction.get_details_for_transaction(conn)?;

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
        conn: &mut PgPoolConnection,
    ) -> diesel::QueryResult<(
        Option<UserTransaction>,
        Option<BlockMetadataTransaction>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
    )> {
        let mut user_transaction: Option<UserTransaction> = None;
        let mut block_metadata_transaction: Option<BlockMetadataTransaction> = None;

        let events = crate::schema::events::table
            .filter(crate::schema::events::transaction_version.eq(&self.version))
            .load::<EventModel>(conn)?;

        let write_set_changes = crate::schema::write_set_changes::table
            .filter(crate::schema::write_set_changes::transaction_version.eq(&self.version))
            .load::<WriteSetChangeModel>(conn)?;

        match self.type_.as_str() {
            "user_transaction" => {
                user_transaction = user_transactions::table
                    .filter(user_transactions::version.eq(&self.version))
                    .first::<UserTransaction>(conn)
                    .optional()?;
            }
            "block_metadata_transaction" => {
                block_metadata_transaction = block_metadata_transactions::table
                    .filter(block_metadata_transactions::version.eq(&self.version))
                    .first::<BlockMetadataTransaction>(conn)
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
}

#[derive(Deserialize, Serialize)]
pub enum TransactionDetail {
    User(UserTransaction, Vec<Signature>),
    BlockMetadata(BlockMetadataTransaction),
}

// Prevent conflicts with other things named `Transaction`
pub type TransactionModel = Transaction;
