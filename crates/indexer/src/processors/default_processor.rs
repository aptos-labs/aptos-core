// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, get_chunks_v2, PgDbPool,
        PgPoolConnection,
    },
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
use aptos_api_types::Transaction;
use async_trait::async_trait;
use diesel::{result::Error, PgConnection};
use field_count::FieldCount;
use std::{collections::HashMap, fmt::Debug};

pub const NAME: &str = "default_processor";
pub struct DefaultTransactionProcessor {
    connection_pool: PgDbPool,
}

impl DefaultTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
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

fn prep_data(
    txns: &[TransactionModel],
    txn_details: &[TransactionDetail],
    events: &[EventModel],
    wscs: &[WriteSetChangeModel],
    wsc_details: &[WriteSetChangeDetail],
) -> (
    Vec<Vec<TransactionModel>>,
    Vec<Vec<UserTransactionModel>>,
    Vec<Vec<Signature>>,
    Vec<Vec<BlockMetadataTransactionModel>>,
    Vec<Vec<EventModel>>,
    Vec<Vec<WriteSetChangeModel>>,
    Vec<Vec<MoveModule>>,
    Vec<Vec<MoveResource>>,
    Vec<Vec<TableItem>>,
    Vec<Vec<TableMetadata>>,
) {
    let mut signatures = vec![];
    let mut user_transactions = vec![];
    let mut block_metadata_transactions = vec![];
    for detail in txn_details {
        match detail {
            TransactionDetail::User(user_txn, sigs) => {
                signatures.append(&mut sigs.clone());
                user_transactions.push(user_txn.clone());
            }
            TransactionDetail::BlockMetadata(bmt) => block_metadata_transactions.push(bmt.clone()),
        }
    }
    let mut move_modules = vec![];
    let mut move_resources = vec![];
    let mut table_items = vec![];
    let mut table_metadata = HashMap::new();
    for detail in wsc_details {
        match detail {
            WriteSetChangeDetail::Module(module) => move_modules.push(module.clone()),
            WriteSetChangeDetail::Resource(resource) => move_resources.push(resource.clone()),
            WriteSetChangeDetail::Table(item, metadata) => {
                table_items.push(item.clone());
                if let Some(meta) = metadata {
                    table_metadata.insert(meta.handle.clone(), meta.clone());
                }
            }
        }
    }
    let mut table_metadata = table_metadata.into_values().collect::<Vec<TableMetadata>>();
    table_metadata.sort_by(|a, b| a.handle.cmp(&b.handle));

    (
        get_chunks_v2(&txns, TransactionModel::field_count()),
        get_chunks_v2(&user_transactions, UserTransactionModel::field_count()),
        get_chunks_v2(&signatures, Signature::field_count()),
        get_chunks_v2(
            &block_metadata_transactions,
            BlockMetadataTransactionModel::field_count(),
        ),
        get_chunks_v2(&events, EventModel::field_count()),
        get_chunks_v2(&wscs, WriteSetChangeModel::field_count()),
        get_chunks_v2(&move_modules, MoveModule::field_count()),
        get_chunks_v2(&move_resources, MoveResource::field_count()),
        get_chunks_v2(&table_items, TableItem::field_count()),
        get_chunks_v2(&table_metadata, TableMetadata::field_count()),
    )
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    txns: Vec<TransactionModel>,
    txn_details: (
        Vec<UserTransactionModel>,
        Vec<Signature>,
        Vec<BlockMetadataTransactionModel>,
    ),
    events: Vec<EventModel>,
    wscs: Vec<WriteSetChangeModel>,
    wsc_details: (
        Vec<MoveModule>,
        Vec<MoveResource>,
        Vec<TableItem>,
        Vec<TableMetadata>,
    ),
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    let (user_transactions, signatures, block_metadata_transactions) = txn_details;
    let (move_modules, move_resources, table_items, table_metadata) = wsc_details;
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_transactions(pg_conn, &txns)?;
            insert_user_transactions(pg_conn, &user_transactions)?;
            insert_signatures(pg_conn, &signatures)?;
            insert_events(pg_conn, &events)?;
            insert_write_set_changes(pg_conn, &wscs)?;
            insert_move_modules(pg_conn, &wsc_details)?;
            insert_move_resources(pg_conn, &wsc_details)?;
            insert_table_data(pg_conn, &wsc_details)?;
            Ok(())
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let txns = clean_data_for_db(txns, true);
                let txn_details = clean_data_for_db(txn_details, true);
                let events = clean_data_for_db(events, true);
                let wscs = clean_data_for_db(wscs, true);
                let wsc_details = clean_data_for_db(wsc_details, true);

                insert_transactions(pg_conn, &txns)?;
                insert_user_transactions_w_sigs(pg_conn, &txn_details)?;
                insert_block_metadata_transactions(pg_conn, &txn_details)?;
                insert_events(pg_conn, &events)?;
                insert_write_set_changes(pg_conn, &wscs)?;
                insert_move_modules(pg_conn, &wsc_details)?;
                insert_move_resources(pg_conn, &wsc_details)?;
                insert_table_data(pg_conn, &wsc_details)?;
                Ok(())
            }),
    }
}

fn insert_transactions(
    conn: &mut PgConnection,
    items_to_insert: &[TransactionModel],
) -> Result<(), diesel::result::Error> {
    use schema::transactions::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), TransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::transactions::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_user_transactions(
    conn: &mut PgConnection,
    items_to_insert: &[UserTransactionModel],
) -> Result<(), diesel::result::Error> {
    use schema::user_transactions::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), UserTransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::user_transactions::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_signatures(
    conn: &mut PgConnection,
    items_to_insert: &[Signature],
) -> Result<(), diesel::result::Error> {
    use schema::signatures::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), Signature::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::signatures::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((
                    transaction_version,
                    multi_agent_index,
                    multi_sig_index,
                    is_sender_primary,
                ))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_block_metadata_transactions(
    conn: &mut PgConnection,
    items_to_insert: &[BlockMetadataTransactionModel],
) -> Result<(), diesel::result::Error> {
    use schema::block_metadata_transactions::dsl::*;
    let chunks = get_chunks(
        items_to_insert.len(),
        BlockMetadataTransactionModel::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::block_metadata_transactions::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_events(
    conn: &mut PgConnection,
    items_to_insert: &[EventModel],
) -> Result<(), diesel::result::Error> {
    use schema::events::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), EventModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::events::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((account_address, creation_number, sequence_number))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_write_set_changes(
    conn: &mut PgConnection,
    items_to_insert: &[WriteSetChangeModel],
) -> Result<(), diesel::result::Error> {
    use schema::write_set_changes::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), WriteSetChangeModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::write_set_changes::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_move_modules(
    conn: &mut PgConnection,
    items_to_insert: &[MoveModule],
) -> Result<(), diesel::result::Error> {
    use schema::move_modules::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), MoveModule::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::move_modules::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_move_resources(
    conn: &mut PgConnection,
    items_to_insert: &[MoveResource],
) -> Result<(), diesel::result::Error> {
    use schema::move_resources::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), MoveResource::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::move_resources::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_table_items(
    conn: &mut PgConnection,
    items_to_insert: &[TableItem],
) -> Result<(), diesel::result::Error> {
    use schema::table_items::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), TableItem::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_items::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_table_metadata(
    conn: &mut PgConnection,
    items_to_insert: &[TableMetadata],
) -> Result<(), diesel::result::Error> {
    use schema::table_metadatas::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), TableMetadata::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_metadatas::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(handle)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for DefaultTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (txns, txn_details, events, write_set_changes, wsc_details) =
            TransactionModel::from_transactions(&transactions);

        let mut signatures = vec![];
        let mut user_transactions = vec![];
        let mut block_metadata_transactions = vec![];
        for detail in txn_details {
            match detail {
                TransactionDetail::User(user_txn, sigs) => {
                    signatures.append(&mut sigs.clone());
                    user_transactions.push(user_txn.clone());
                }
                TransactionDetail::BlockMetadata(bmt) => {
                    block_metadata_transactions.push(bmt.clone())
                }
            }
        }
        let mut move_modules = vec![];
        let mut move_resources = vec![];
        let mut table_items = vec![];
        let mut table_metadata = HashMap::new();
        for detail in wsc_details {
            match detail {
                WriteSetChangeDetail::Module(module) => move_modules.push(module.clone()),
                WriteSetChangeDetail::Resource(resource) => move_resources.push(resource.clone()),
                WriteSetChangeDetail::Table(item, metadata) => {
                    table_items.push(item.clone());
                    if let Some(meta) = metadata {
                        table_metadata.insert(meta.handle.clone(), meta.clone());
                    }
                }
            }
        }
        let mut table_metadata = table_metadata.into_values().collect::<Vec<TableMetadata>>();
        table_metadata.sort_by(|a, b| a.handle.cmp(&b.handle));

        let mut conn = self.get_conn();
        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            txns,
            (user_transactions, signatures, block_metadata_transactions),
            write_set_changes,
            (move_modules, move_resources, table_items, table_metadata),
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
