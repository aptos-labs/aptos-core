// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    block_metadata_transactions::{BlockMetadataTransaction, BlockMetadataTransactionQuery},
    events::{EventModel, EventQuery},
    signatures::Signature,
    user_transactions::{UserTransaction, UserTransactionQuery},
    write_set_changes::{WriteSetChangeDetail, WriteSetChangeModel, WriteSetChangeQuery},
};
use crate::{
    database::PgPoolConnection,
    schema::{block_metadata_transactions, transactions, user_transactions},
    util::u64_to_bigdecimal,
};
use velor_api_types::{Transaction as APITransaction, TransactionInfo};
use bigdecimal::BigDecimal;
use diesel::{
    BelongingToDsl, ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

const DEFAULT_ACCOUNT_ADDRESS: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
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
    pub epoch: i64,
}

/// Need a separate struct for queryable because we don't want to define the inserted_at column (letting DB fill)
#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(version))]
#[diesel(table_name = transactions)]
pub struct TransactionQuery {
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
    pub inserted_at: chrono::NaiveDateTime,
    pub epoch: i64,
}

impl Transaction {
    fn from_transaction_info(
        info: &TransactionInfo,
        payload: Option<serde_json::Value>,
        type_: String,
        num_events: i64,
        block_height: i64,
        epoch: i64,
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
            epoch,
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
        let epoch = transaction.transaction_info().unwrap().epoch.unwrap().0 as i64;
        match transaction {
            APITransaction::UserTransaction(user_txn) => {
                let (user_txn_output, signatures) =
                    UserTransaction::from_transaction(user_txn, block_height, epoch);
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
                        epoch,
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
            },
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
                        epoch,
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
            },
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
                        epoch,
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
            },
            APITransaction::StateCheckpointTransaction(state_checkpoint_txn) => (
                Self::from_transaction_info(
                    &state_checkpoint_txn.info,
                    None,
                    transaction.type_str().to_string(),
                    0,
                    block_height,
                    epoch,
                ),
                None,
                vec![],
                vec![],
                vec![],
            ),
            APITransaction::BlockEpilogueTransaction(block_epilogue_txn) => (
                Self::from_transaction_info(
                    &block_epilogue_txn.info,
                    None,
                    transaction.type_str().to_string(),
                    0,
                    block_height,
                    epoch,
                ),
                None,
                vec![],
                vec![],
                vec![],
            ),
            APITransaction::PendingTransaction(..) => {
                unreachable!()
            },
            APITransaction::ValidatorTransaction(validator_txn) => (
                Self::from_transaction_info(
                    validator_txn.transaction_info(),
                    None,
                    transaction.type_str().to_string(),
                    0,
                    block_height,
                    epoch,
                ),
                None,
                vec![],
                vec![],
                vec![],
            ),
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
            let (txn, txn_detail, event_list, mut wsc_list, mut wsc_detail_list) =
                Self::from_transaction(txn);
            let mut event_v1_list = event_list
                .into_iter()
                .filter(|e| {
                    !(e.sequence_number == 0
                        && e.creation_number == 0
                        && e.account_address == DEFAULT_ACCOUNT_ADDRESS)
                })
                .collect::<Vec<_>>();
            txns.push(txn);
            if let Some(a) = txn_detail {
                txn_details.push(a);
            }
            events.append(&mut event_v1_list);
            wscs.append(&mut wsc_list);
            wsc_details.append(&mut wsc_detail_list);
        }
        (txns, txn_details, events, wscs, wsc_details)
    }
}

impl TransactionQuery {
    pub fn get_many_by_version(
        start_version: u64,
        number_to_get: i64,
        conn: &mut PgPoolConnection,
    ) -> diesel::QueryResult<
        Vec<(
            Self,
            Option<UserTransactionQuery>,
            Option<BlockMetadataTransactionQuery>,
            Vec<EventQuery>,
            Vec<WriteSetChangeQuery>,
        )>,
    > {
        let mut txs = transactions::table
            .filter(transactions::version.ge(start_version as i64))
            .order(transactions::version.asc())
            .limit(number_to_get)
            .load::<Self>(conn)?;

        let mut user_transactions: Vec<Vec<UserTransactionQuery>> =
            UserTransactionQuery::belonging_to(&txs)
                .load::<UserTransactionQuery>(conn)?
                .grouped_by(&txs);

        let mut block_metadata_transactions: Vec<Vec<BlockMetadataTransactionQuery>> =
            BlockMetadataTransactionQuery::belonging_to(&txs)
                .load::<BlockMetadataTransactionQuery>(conn)?
                .grouped_by(&txs);

        let mut events: Vec<Vec<EventQuery>> = EventQuery::belonging_to(&txs)
            .load::<EventQuery>(conn)?
            .grouped_by(&txs);

        let mut write_set_changes: Vec<Vec<WriteSetChangeQuery>> =
            WriteSetChangeQuery::belonging_to(&txs)
                .load::<WriteSetChangeQuery>(conn)?
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
        Self,
        Option<UserTransactionQuery>,
        Option<BlockMetadataTransactionQuery>,
        Vec<EventQuery>,
        Vec<WriteSetChangeQuery>,
    )> {
        let transaction = transactions::table
            .filter(transactions::version.eq(version as i64))
            .first::<Self>(conn)?;

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
        Self,
        Option<UserTransactionQuery>,
        Option<BlockMetadataTransactionQuery>,
        Vec<EventQuery>,
        Vec<WriteSetChangeQuery>,
    )> {
        let transaction = transactions::table
            .filter(transactions::hash.eq(&transaction_hash))
            .first::<Self>(conn)?;

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
        Option<UserTransactionQuery>,
        Option<BlockMetadataTransactionQuery>,
        Vec<EventQuery>,
        Vec<WriteSetChangeQuery>,
    )> {
        let mut user_transaction: Option<UserTransactionQuery> = None;
        let mut block_metadata_transaction: Option<BlockMetadataTransactionQuery> = None;

        let events = crate::schema::events::table
            .filter(crate::schema::events::transaction_version.eq(&self.version))
            .load::<EventQuery>(conn)?;

        let write_set_changes = crate::schema::write_set_changes::table
            .filter(crate::schema::write_set_changes::transaction_version.eq(&self.version))
            .load::<WriteSetChangeQuery>(conn)?;

        match self.type_.as_str() {
            "user_transaction" => {
                user_transaction = user_transactions::table
                    .filter(user_transactions::version.eq(&self.version))
                    .first::<UserTransactionQuery>(conn)
                    .optional()?;
            },
            "block_metadata_transaction" => {
                block_metadata_transaction = block_metadata_transactions::table
                    .filter(block_metadata_transactions::version.eq(&self.version))
                    .first::<BlockMetadataTransactionQuery>(conn)
                    .optional()?;
            },
            "genesis_transaction" => {},
            "state_checkpoint_transaction" => {},
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
