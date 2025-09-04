// Copyright © Velor Foundation
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
        move_tables::{CurrentTableItem, TableItem, TableMetadata},
        signatures::Signature,
        transactions::{TransactionDetail, TransactionModel},
        user_transactions::UserTransactionModel,
        v2_objects::{CurrentObject, Object},
        write_set_changes::{WriteSetChangeDetail, WriteSetChangeModel},
    },
    schema,
};
use velor_api_types::{Transaction, WriteSetChange};
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
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

fn insert_to_db_impl(
    conn: &mut PgConnection,
    txns: &[TransactionModel],
    txn_details: (
        &[UserTransactionModel],
        &[Signature],
        &[BlockMetadataTransactionModel],
    ),
    events: &[EventModel],
    wscs: &[WriteSetChangeModel],
    wsc_details: (
        &[MoveModule],
        &[MoveResource],
        &[TableItem],
        &[CurrentTableItem],
        &[TableMetadata],
    ),
    object_core: (&[Object], &[CurrentObject]),
) -> Result<(), diesel::result::Error> {
    let (user_transactions, signatures, block_metadata_transactions) = txn_details;
    let (move_modules, move_resources, table_items, current_table_items, table_metadata) =
        wsc_details;
    let (objects, current_objects) = object_core;
    insert_transactions(conn, txns)?;
    insert_user_transactions(conn, user_transactions)?;
    insert_signatures(conn, signatures)?;
    insert_block_metadata_transactions(conn, block_metadata_transactions)?;
    insert_events(conn, events)?;
    insert_write_set_changes(conn, wscs)?;
    insert_move_modules(conn, move_modules)?;
    insert_move_resources(conn, move_resources)?;
    insert_table_items(conn, table_items)?;
    insert_current_table_items(conn, current_table_items)?;
    insert_table_metadata(conn, table_metadata)?;
    insert_objects(conn, objects)?;
    insert_current_objects(conn, current_objects)?;
    Ok(())
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
        Vec<CurrentTableItem>,
        Vec<TableMetadata>,
    ),
    object_core: (Vec<Object>, Vec<CurrentObject>),
) -> Result<(), diesel::result::Error> {
    velor_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    let (user_transactions, signatures, block_metadata_transactions) = txn_details;
    let (move_modules, move_resources, table_items, current_table_items, table_metadata) =
        wsc_details;
    let (objects, current_objects) = object_core;
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_to_db_impl(
                pg_conn,
                &txns,
                (
                    &user_transactions,
                    &signatures,
                    &block_metadata_transactions,
                ),
                &events,
                &wscs,
                (
                    &move_modules,
                    &move_resources,
                    &table_items,
                    &current_table_items,
                    &table_metadata,
                ),
                (&objects, &current_objects),
            )
        }) {
        Ok(_) => Ok(()),
        Err(_) => {
            let txns = clean_data_for_db(txns, true);
            let user_transactions = clean_data_for_db(user_transactions, true);
            let signatures = clean_data_for_db(signatures, true);
            let block_metadata_transactions = clean_data_for_db(block_metadata_transactions, true);
            let events = clean_data_for_db(events, true);
            let wscs = clean_data_for_db(wscs, true);
            let move_modules = clean_data_for_db(move_modules, true);
            let move_resources = clean_data_for_db(move_resources, true);
            let table_items = clean_data_for_db(table_items, true);
            let current_table_items = clean_data_for_db(current_table_items, true);
            let table_metadata = clean_data_for_db(table_metadata, true);
            let objects = clean_data_for_db(objects, true);
            let current_objects = clean_data_for_db(current_objects, true);

            conn.build_transaction()
                .read_write()
                .run::<_, Error, _>(|pg_conn| {
                    insert_to_db_impl(
                        pg_conn,
                        &txns,
                        (
                            &user_transactions,
                            &signatures,
                            &block_metadata_transactions,
                        ),
                        &events,
                        &wscs,
                        (
                            &move_modules,
                            &move_resources,
                            &table_items,
                            &current_table_items,
                            &table_metadata,
                        ),
                        (&objects, &current_objects),
                    )
                })
        },
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
                .do_update()
                .set((
                    inserted_at.eq(excluded(inserted_at)),
                    event_index.eq(excluded(event_index)),
                )),
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
                .do_update()
                .set((
                    inserted_at.eq(excluded(inserted_at)),
                    state_key_hash.eq(excluded(state_key_hash)),
                )),
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

fn insert_current_table_items(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTableItem],
) -> Result<(), diesel::result::Error> {
    use schema::current_table_items::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), CurrentTableItem::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_table_items::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((table_handle, key_hash))
                .do_update()
                .set((
                    key.eq(excluded(key)),
                    decoded_key.eq(excluded(decoded_key)),
                    decoded_value.eq(excluded(decoded_value)),
                    is_deleted.eq(excluded(is_deleted)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
                Some(" WHERE current_table_items.last_transaction_version <= excluded.last_transaction_version "),
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

fn insert_objects(
    conn: &mut PgConnection,
    items_to_insert: &[Object],
) -> Result<(), diesel::result::Error> {
    use schema::objects::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), Object::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::objects::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_objects(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentObject],
) -> Result<(), diesel::result::Error> {
    use schema::current_objects::dsl::*;
    let chunks = get_chunks(items_to_insert.len(), CurrentObject::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_objects::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(object_address)
                .do_update()
                .set((
                    owner_address.eq(excluded(owner_address)),
                    state_key_hash.eq(excluded(state_key_hash)),
                    allow_ungated_transfer.eq(excluded(allow_ungated_transfer)),
                    last_guid_creation_num.eq(excluded(last_guid_creation_num)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    is_deleted.eq(excluded(is_deleted)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
                Some(" WHERE current_objects.last_transaction_version <= excluded.last_transaction_version "),
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
        let mut conn = self.get_conn();

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
                },
                TransactionDetail::BlockMetadata(bmt) => {
                    block_metadata_transactions.push(bmt.clone())
                },
            }
        }
        let mut move_modules = vec![];
        let mut move_resources = vec![];
        let mut table_items = vec![];
        let mut current_table_items = HashMap::new();
        let mut table_metadata = HashMap::new();
        for detail in wsc_details {
            match detail {
                WriteSetChangeDetail::Module(module) => move_modules.push(module.clone()),
                WriteSetChangeDetail::Resource(resource) => move_resources.push(resource.clone()),
                WriteSetChangeDetail::Table(item, current_item, metadata) => {
                    table_items.push(item.clone());
                    current_table_items.insert(
                        (
                            current_item.table_handle.clone(),
                            current_item.key_hash.clone(),
                        ),
                        current_item.clone(),
                    );
                    if let Some(meta) = metadata {
                        table_metadata.insert(meta.handle.clone(), meta.clone());
                    }
                },
            }
        }

        // TODO, merge this loop with above
        // Moving object handling here because we need a single object
        // map through transactions for lookups
        let mut all_objects = vec![];
        let mut all_current_objects = HashMap::new();
        for txn in &transactions {
            let (changes, txn_version) = match txn {
                Transaction::UserTransaction(user_txn) => (
                    user_txn.info.changes.clone(),
                    user_txn.info.version.0 as i64,
                ),
                Transaction::BlockMetadataTransaction(bmt_txn) => {
                    (bmt_txn.info.changes.clone(), bmt_txn.info.version.0 as i64)
                },
                _ => continue,
            };

            for (index, wsc) in changes.iter().enumerate() {
                let index = index as i64;
                match wsc {
                    WriteSetChange::WriteResource(inner) => {
                        if let Some((object, current_object)) =
                            &Object::from_write_resource(inner, txn_version, index).unwrap()
                        {
                            all_objects.push(object.clone());
                            all_current_objects
                                .insert(object.object_address.clone(), current_object.clone());
                        }
                    },
                    WriteSetChange::DeleteResource(inner) => {
                        // Passing all_current_objects into the function so that we can get the owner of the deleted
                        // resource if it was handled in the same batch
                        if let Some((object, current_object)) = Object::from_delete_resource(
                            inner,
                            txn_version,
                            index,
                            &all_current_objects,
                            &mut conn,
                        )
                        .unwrap()
                        {
                            all_objects.push(object.clone());
                            all_current_objects
                                .insert(object.object_address.clone(), current_object.clone());
                        }
                    },
                    _ => {},
                }
            }
        }
        // Getting list of values and sorting by pk in order to avoid postgres deadlock since we're doing multi threaded db writes
        let mut current_table_items = current_table_items
            .into_values()
            .collect::<Vec<CurrentTableItem>>();
        let mut table_metadata = table_metadata.into_values().collect::<Vec<TableMetadata>>();
        let mut all_current_objects = all_current_objects
            .into_values()
            .collect::<Vec<CurrentObject>>();

        // Sort by PK
        current_table_items
            .sort_by(|a, b| (&a.table_handle, &a.key_hash).cmp(&(&b.table_handle, &b.key_hash)));
        table_metadata.sort_by(|a, b| a.handle.cmp(&b.handle));
        all_current_objects.sort_by(|a, b| a.object_address.cmp(&b.object_address));

        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            txns,
            (user_transactions, signatures, block_metadata_transactions),
            events,
            write_set_changes,
            (
                move_modules,
                move_resources,
                table_items,
                current_table_items,
                table_metadata,
            ),
            (all_objects, all_current_objects),
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
