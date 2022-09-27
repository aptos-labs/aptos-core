// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
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
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::fmt::Debug;

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

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
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
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_transactions(pg_conn, &txns)?;
            insert_user_transactions_w_sigs(pg_conn, &txn_details)?;
            insert_block_metadata_transactions(pg_conn, &txn_details)?;
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
    txns: &[TransactionModel],
) -> Result<(), diesel::result::Error> {
    use schema::transactions::dsl::*;
    let chunks = get_chunks(txns.len(), TransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::transactions::table)
                .values(&txns[start_ind..end_ind])
                .on_conflict(version)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_user_transactions_w_sigs(
    conn: &mut PgConnection,
    txn_details: &[TransactionDetail],
) -> Result<(), diesel::result::Error> {
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
                .do_update()
                .set((
                    ut_schema::block_height.eq(excluded(ut_schema::block_height)),
                    ut_schema::parent_signature_type.eq(excluded(ut_schema::parent_signature_type)),
                    ut_schema::sender.eq(excluded(ut_schema::sender)),
                    ut_schema::sequence_number.eq(excluded(ut_schema::sequence_number)),
                    ut_schema::max_gas_amount.eq(excluded(ut_schema::max_gas_amount)),
                    ut_schema::expiration_timestamp_secs
                        .eq(excluded(ut_schema::expiration_timestamp_secs)),
                    ut_schema::gas_unit_price.eq(excluded(ut_schema::gas_unit_price)),
                    ut_schema::timestamp.eq(excluded(ut_schema::timestamp)),
                    ut_schema::entry_function_id_str.eq(excluded(ut_schema::entry_function_id_str)),
                    ut_schema::inserted_at.eq(excluded(ut_schema::inserted_at)),
                )),
            None,
        )?;
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
            None,
        )?;
    }
    Ok(())
}

fn insert_block_metadata_transactions(
    conn: &mut PgConnection,
    txn_details: &[TransactionDetail],
) -> Result<(), diesel::result::Error> {
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
            None,
        )?;
    }
    Ok(())
}

fn insert_events(conn: &mut PgConnection, ev: &[EventModel]) -> Result<(), diesel::result::Error> {
    use schema::events::dsl::*;

    let chunks = get_chunks(ev.len(), EventModel::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::events::table)
                .values(&ev[start_ind..end_ind])
                .on_conflict((account_address, creation_number, sequence_number))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_write_set_changes(
    conn: &mut PgConnection,
    wscs: &[WriteSetChangeModel],
) -> Result<(), diesel::result::Error> {
    use schema::write_set_changes::dsl::*;

    let chunks = get_chunks(wscs.len(), WriteSetChangeModel::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::write_set_changes::table)
                .values(&wscs[start_ind..end_ind])
                .on_conflict((transaction_version, index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_move_modules(
    conn: &mut PgConnection,
    wsc_details: &[WriteSetChangeDetail],
) -> Result<(), diesel::result::Error> {
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
            None,
        )?;
    }
    Ok(())
}

fn insert_move_resources(
    conn: &mut PgConnection,
    wsc_details: &[WriteSetChangeDetail],
) -> Result<(), diesel::result::Error> {
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
            None,
        )?;
    }
    Ok(())
}

/// This will insert all table data within each transaction within a block
fn insert_table_data(
    conn: &mut PgConnection,
    wsc_details: &[WriteSetChangeDetail],
) -> Result<(), diesel::result::Error> {
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
            None,
        )?;
    }
    let chunks = get_chunks(metadata_nonnull.len(), TableMetadata::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_metadatas::table)
                .values(&metadata_nonnull[start_ind..end_ind])
                .on_conflict(tm::handle)
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
        let (txns, user_txns, bm_txns, events, write_set_changes) =
            TransactionModel::from_transactions(&transactions);

        let mut conn = self.get_conn();
        let tx_result = insert_to_db(
            &mut conn,
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
