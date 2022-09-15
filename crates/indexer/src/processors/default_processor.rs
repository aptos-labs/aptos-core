// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        block_metadata_transactions::BlockMetadataTransactionModel,
        events::EventModel,
        move_modules::MoveModule,
        move_resources::MoveResource,
        move_tables::{TableItem, TableMetadata},
        signatures::Signature,
        transactions::{TransactionDetail, TransactionModel},
        user_transactions::UserTransactionModel,
        write_set_changes::{WriteSetChangeDetail, WriteSetChangeModel},
    },
    schema,
};
use aptos_api::Context;
use aptos_api_types::Transaction;
use async_trait::async_trait;
use diesel::result::Error;
use field_count::FieldCount;
use std::{fmt::Debug, sync::Arc};

pub const NAME: &str = "default_processor";
pub struct DefaultTransactionProcessor {
    connection_pool: PgDbPool,
    context: Arc<Context>,
}

impl DefaultTransactionProcessor {
    pub fn new(connection_pool: PgDbPool, context: Arc<Context>) -> Self {
        Self {
            connection_pool,
            context,
        }
    }
}

impl Debug for DefaultTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "DefaultTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db(
    conn: &PgPoolConnection,
    name: &'static str,
    start_version: i64,
    end_version: i64,
    txns: Vec<TransactionModel>,
    txn_details: Vec<TransactionDetail>,
    events: Vec<EventModel>,
    wscs: Vec<WriteSetChangeModel>,
    wsc_details: Vec<WriteSetChangeDetail>,
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    conn.build_transaction()
        .read_write()
        .run::<_, Error, _>(|| {
            insert_transactions(conn, &txns);
            insert_user_transactions_w_sigs(conn, &txn_details);
            insert_block_metadata_transactions(conn, &txn_details);
            insert_events(conn, &events);
            insert_write_set_changes(conn, &wscs);
            insert_move_modules(conn, &wsc_details);
            insert_move_resources(conn, &wsc_details);
            insert_table_data(conn, &wsc_details);
            Ok(())
        })
}

fn insert_transactions(conn: &PgPoolConnection, txns: &[TransactionModel]) {
    use schema::transactions::dsl::*;
    let chunks = get_chunks(txns.len(), TransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::transactions::table)
                .values(&txns[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
        )
        .expect("Error inserting transactions into database");
    }
}

fn insert_user_transactions_w_sigs(conn: &PgPoolConnection, txn_details: &[TransactionDetail]) {
    use schema::{signatures::dsl as sig_schema, user_transactions::dsl as ut_schema};
    let mut all_signatures = vec![];
    let mut all_user_transactions = vec![];
    for detail in txn_details {
        if let TransactionDetail::User(user_txn, sigs) = detail {
            all_signatures.append(&mut sigs.clone());
            all_user_transactions.push(user_txn.clone());
        }
    }
    let chunks = get_chunks(
        all_user_transactions.len(),
        UserTransactionModel::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::user_transactions::table)
                .values(&all_user_transactions[start_ind..end_ind])
                .on_conflict(ut_schema::version)
                .do_nothing(),
        )
        .expect("Error inserting user transactions into database");
    }
    let chunks = get_chunks(all_signatures.len(), Signature::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::signatures::table)
                .values(&all_signatures[start_ind..end_ind])
                .on_conflict((
                    sig_schema::transaction_version,
                    sig_schema::multi_agent_index,
                    sig_schema::multi_sig_index,
                    sig_schema::is_sender_primary,
                ))
                .do_nothing(),
        )
        .expect("Error inserting signatures into database");
    }
}

fn insert_block_metadata_transactions(conn: &PgPoolConnection, txn_details: &[TransactionDetail]) {
    use schema::block_metadata_transactions::dsl::*;

    let bmt = txn_details
        .iter()
        .filter_map(|detail| match detail {
            TransactionDetail::BlockMetadata(bmt) => Some(bmt.clone()),
            _ => None,
        })
        .collect::<Vec<BlockMetadataTransactionModel>>();

    let chunks = get_chunks(bmt.len(), BlockMetadataTransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::block_metadata_transactions::table)
                .values(&bmt[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
        )
        .expect("Error inserting block metadata transactions into database");
    }
}

fn insert_events(conn: &PgPoolConnection, ev: &Vec<EventModel>) {
    use schema::events::dsl::*;

    let chunks = get_chunks(ev.len(), EventModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::events::table)
                .values(&ev[start_ind..end_ind])
                .on_conflict((key, sequence_number))
                .do_nothing(),
        )
        .expect("Error inserting events into database");
    }
}

fn insert_write_set_changes(conn: &PgPoolConnection, wscs: &Vec<WriteSetChangeModel>) {
    use schema::write_set_changes::dsl::*;

    let chunks = get_chunks(wscs.len(), WriteSetChangeModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::write_set_changes::table)
                .values(&wscs[start_ind..end_ind])
                .on_conflict((transaction_version, index))
                .do_nothing(),
        )
        .expect("Error inserting write set changes into database");
    }
}

fn insert_move_modules(conn: &PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
    use schema::move_modules::dsl::*;

    let modules = wsc_details
        .iter()
        .filter_map(|detail| match detail {
            WriteSetChangeDetail::Module(module) => Some(module.clone()),
            _ => None,
        })
        .collect::<Vec<MoveModule>>();

    let chunks = get_chunks(modules.len(), MoveModule::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::move_modules::table)
                .values(&modules[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
        )
        .expect("Error inserting move modules into database");
    }
}

fn insert_move_resources(conn: &PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
    use schema::move_resources::dsl::*;

    let resources = wsc_details
        .iter()
        .filter_map(|detail| match detail {
            WriteSetChangeDetail::Resource(resource) => Some(resource.clone()),
            _ => None,
        })
        .collect::<Vec<MoveResource>>();
    let chunks = get_chunks(resources.len(), MoveResource::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::move_resources::table)
                .values(&resources[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
        )
        .expect("Error inserting move resources into database");
    }
}

/// This will insert all table data within each transaction within a block
fn insert_table_data(conn: &PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
    use schema::{table_items::dsl as ti, table_metadatas::dsl as tm};

    let (items, metadata): (Vec<TableItem>, Vec<Option<TableMetadata>>) = wsc_details
        .iter()
        .filter_map(|detail| match detail {
            WriteSetChangeDetail::Table(table_item, table_metadata) => {
                Some((table_item.clone(), table_metadata.clone()))
            }
            _ => None,
        })
        .collect::<Vec<(TableItem, Option<TableMetadata>)>>()
        .into_iter()
        .unzip();
    let mut metadata_nonnull = metadata
        .iter()
        .filter_map(|x| x.clone())
        .collect::<Vec<TableMetadata>>();
    metadata_nonnull.dedup_by(|a, b| a.handle == b.handle);
    metadata_nonnull.sort_by(|a, b| a.handle.cmp(&b.handle));

    let chunks = get_chunks(items.len(), TableItem::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_items::table)
                .values(&items[start_ind..end_ind])
                .on_conflict((ti::transaction_version, ti::write_set_change_index))
                .do_nothing(),
        )
        .expect("Error inserting table items into database");
    }
    let chunks = get_chunks(metadata_nonnull.len(), TableMetadata::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_metadatas::table)
                .values(&metadata_nonnull[start_ind..end_ind])
                .on_conflict(tm::handle)
                .do_nothing(),
        )
        .expect("Error inserting table metadata into database");
    }
}

#[async_trait]
impl TransactionProcessor for DefaultTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: i64,
        end_version: i64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (_block_start_version, _block_last_version, block_event) = self
            .context
            .db
            .get_block_info_by_version(start_version as u64)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get block_info for start version {}",
                    start_version,
                )
            });

        let (txns, user_txns, bm_txns, events, write_set_changes) =
            TransactionModel::from_transactions(&transactions, block_event.height() as i64);

        let conn = self.get_conn();
        let tx_result = insert_to_db(
            &conn,
            self.name(),
            start_version,
            end_version,
            txns,
            user_txns,
            bm_txns,
            events,
            write_set_changes,
        );
        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                start_version,
                end_version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
