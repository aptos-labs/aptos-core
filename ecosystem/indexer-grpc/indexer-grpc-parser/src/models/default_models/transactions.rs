// Copyright © Aptos Foundation
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
    schema::{block_metadata_transactions, transactions, user_transactions},
    utils::{
        database::PgPoolConnection,
        util::{get_clean_payload, get_clean_writeset, standardize_address, u64_to_bigdecimal},
    },
};
use aptos_protos::transaction::testing1::v1::{
    transaction::{TransactionType, TxnData},
    Transaction as TransactionPB, TransactionInfo,
};
use bigdecimal::BigDecimal;
use diesel::{
    BelongingToDsl, ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

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
        version: i64,
        type_: String,
        num_events: i64,
        block_height: i64,
        epoch: i64,
    ) -> Self {
        Self {
            type_,
            payload,
            version,
            block_height,
            hash: standardize_address(hex::encode(info.hash.as_slice()).as_str()),
            state_change_hash: standardize_address(
                hex::encode(info.state_change_hash.as_slice()).as_str(),
            ),
            event_root_hash: standardize_address(
                hex::encode(info.event_root_hash.as_slice()).as_str(),
            ),
            state_checkpoint_hash: info
                .state_checkpoint_hash
                .as_ref()
                .map(|hash| standardize_address(hex::encode(hash).as_str())),
            gas_used: u64_to_bigdecimal(info.gas_used),
            success: info.success,
            vm_status: info.vm_status.clone(),
            accumulator_root_hash: standardize_address(
                hex::encode(info.accumulator_root_hash.as_slice()).as_str(),
            ),
            num_events,
            num_write_set_changes: info.changes.len() as i64,
            epoch,
        }
    }

    pub fn from_transaction(
        transaction: &TransactionPB,
    ) -> (
        Self,
        Option<TransactionDetail>,
        Vec<EventModel>,
        Vec<WriteSetChangeModel>,
        Vec<WriteSetChangeDetail>,
    ) {
        let block_height = transaction.block_height as i64;
        let epoch = transaction.epoch as i64;
        let txn_data = transaction
            .txn_data
            .as_ref()
            .expect("Txn Data doesn't exit!");
        let version = transaction.version as i64;
        let transaction_type = TransactionType::from_i32(transaction.r#type)
            .expect("Transaction type doesn't exist!")
            .as_str_name()
            .to_string();
        let transaction_info = transaction
            .info
            .as_ref()
            .expect("Transaction info doesn't exist!");
        let timestamp = transaction
            .timestamp
            .as_ref()
            .expect("Transaction timestamp doesn't exist!");
        match txn_data {
            TxnData::User(user_txn) => {
                let (user_txn_output, signatures) = UserTransaction::from_transaction(
                    user_txn,
                    timestamp,
                    block_height,
                    epoch,
                    version,
                );

                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &transaction_info.changes,
                    version,
                    block_height,
                );
                let payload = user_txn
                    .request
                    .as_ref()
                    .expect("Getting user request failed.")
                    .payload
                    .as_ref()
                    .expect("Getting payload failed.");
                let payload_cleaned = get_clean_payload(payload, version);

                (
                    Self::from_transaction_info(
                        transaction_info,
                        payload_cleaned,
                        version,
                        transaction_type,
                        user_txn.events.len() as i64,
                        block_height,
                        epoch,
                    ),
                    Some(TransactionDetail::User(user_txn_output, signatures)),
                    EventModel::from_events(&user_txn.events, version, block_height),
                    wsc,
                    wsc_detail,
                )
            },
            TxnData::Genesis(genesis_txn) => {
                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &transaction_info.changes,
                    version,
                    block_height,
                );
                let payload = genesis_txn.payload.as_ref().unwrap();
                let payload_cleaned = get_clean_writeset(payload, version);
                (
                    Self::from_transaction_info(
                        transaction_info,
                        payload_cleaned,
                        version,
                        transaction_type,
                        0,
                        block_height,
                        epoch,
                    ),
                    None,
                    EventModel::from_events(&genesis_txn.events, version, block_height),
                    wsc,
                    wsc_detail,
                )
            },
            TxnData::BlockMetadata(block_metadata_txn) => {
                let (wsc, wsc_detail) = WriteSetChangeModel::from_write_set_changes(
                    &transaction_info.changes,
                    version,
                    block_height,
                );
                (
                    Self::from_transaction_info(
                        transaction_info,
                        None,
                        version,
                        transaction_type,
                        0,
                        block_height,
                        epoch,
                    ),
                    Some(TransactionDetail::BlockMetadata(
                        BlockMetadataTransaction::from_transaction(
                            block_metadata_txn,
                            version,
                            block_height,
                            epoch,
                            timestamp,
                        ),
                    )),
                    EventModel::from_events(&block_metadata_txn.events, version, block_height),
                    wsc,
                    wsc_detail,
                )
            },
            TxnData::StateCheckpoint(_state_checkpoint_txn) => (
                Self::from_transaction_info(
                    transaction_info,
                    None,
                    version,
                    transaction_type,
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
        transactions: &[TransactionPB],
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
