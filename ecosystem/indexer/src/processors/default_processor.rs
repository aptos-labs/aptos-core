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
use aptos_rest_client::Transaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods};
use field_count::FieldCount;
use std::fmt::Debug;

pub const NAME: &'static str = "default_processor";
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
    conn: &PgPoolConnection,
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
                .do_update()
                .set((
                    hash.eq(excluded(hash)),
                    type_.eq(excluded(type_)),
                    payload.eq(excluded(payload)),
                    state_change_hash.eq(excluded(state_change_hash)),
                    event_root_hash.eq(excluded(event_root_hash)),
                    state_checkpoint_hash.eq(excluded(state_checkpoint_hash)),
                    gas_used.eq(excluded(gas_used)),
                    success.eq(excluded(success)),
                    vm_status.eq(excluded(vm_status)),
                    accumulator_root_hash.eq(excluded(accumulator_root_hash)),
                    num_events.eq(excluded(num_events)),
                    num_write_set_changes.eq(excluded(num_write_set_changes)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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
    let chunks = get_chunks(all_user_transactions.len(), UserTransactionModel::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::user_transactions::table)
                .values(&all_user_transactions[start_ind..end_ind])
                .on_conflict(ut_schema::version)
                .do_update()
                .set((
                    ut_schema::parent_signature_type.eq(excluded(ut_schema::parent_signature_type)),
                    ut_schema::sender.eq(excluded(ut_schema::sender)),
                    ut_schema::sequence_number.eq(excluded(ut_schema::sequence_number)),
                    ut_schema::max_gas_amount.eq(excluded(ut_schema::max_gas_amount)),
                    ut_schema::expiration_timestamp_secs
                        .eq(excluded(ut_schema::expiration_timestamp_secs)),
                    ut_schema::gas_unit_price.eq(excluded(ut_schema::gas_unit_price)),
                    ut_schema::timestamp.eq(excluded(ut_schema::timestamp)),
                    ut_schema::inserted_at.eq(excluded(ut_schema::inserted_at)),
                    ut_schema::entry_function_id_str.eq(excluded(ut_schema::entry_function_id_str)),
                )),
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
                .do_update()
                .set((
                    sig_schema::signer.eq(excluded(sig_schema::signer)),
                    sig_schema::type_.eq(excluded(sig_schema::type_)),
                    sig_schema::public_key.eq(excluded(sig_schema::public_key)),
                    sig_schema::signature.eq(excluded(sig_schema::signature)),
                    sig_schema::threshold.eq(excluded(sig_schema::threshold)),
                    sig_schema::public_key_indices.eq(excluded(sig_schema::public_key_indices)),
                    sig_schema::inserted_at.eq(excluded(sig_schema::inserted_at)),
                )),
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
                .do_update()
                .set((
                    block_height.eq(excluded(block_height)),
                    id.eq(excluded(id)),
                    round.eq(excluded(round)),
                    epoch.eq(excluded(epoch)),
                    previous_block_votes_bitvec.eq(excluded(previous_block_votes_bitvec)),
                    proposer.eq(excluded(proposer)),
                    failed_proposer_indices.eq(excluded(failed_proposer_indices)),
                    timestamp.eq(excluded(timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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
                .do_update()
                .set((
                    creation_number.eq(excluded(creation_number)),
                    account_address.eq(excluded(account_address)),
                    transaction_version.eq(excluded(transaction_version)),
                    type_.eq(excluded(type_)),
                    data.eq(excluded(data)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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
                .do_update()
                .set((
                    type_.eq(excluded(type_)),
                    address.eq(excluded(address)),
                    hash.eq(excluded(hash)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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
                .do_update()
                .set((
                    name.eq(excluded(name)),
                    address.eq(excluded(address)),
                    bytecode.eq(excluded(bytecode)),
                    friends.eq(excluded(friends)),
                    exposed_functions.eq(excluded(exposed_functions)),
                    structs.eq(excluded(structs)),
                    is_deleted.eq(excluded(is_deleted)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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
                .do_update()
                .set((
                    name.eq(excluded(name)),
                    address.eq(excluded(address)),
                    module.eq(excluded(module)),
                    generic_type_params.eq(excluded(generic_type_params)),
                    data.eq(excluded(data)),
                    is_deleted.eq(excluded(is_deleted)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
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

    let chunks = get_chunks(items.len(), TableItem::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_items::table)
                .values(&items[start_ind..end_ind])
                .on_conflict((ti::transaction_version, ti::write_set_change_index))
                .do_update()
                .set((
                    ti::key.eq(excluded(ti::key)),
                    ti::table_handle.eq(excluded(ti::table_handle)),
                    ti::decoded_key.eq(excluded(ti::decoded_key)),
                    ti::decoded_value.eq(excluded(ti::decoded_value)),
                    ti::is_deleted.eq(excluded(ti::is_deleted)),
                    ti::inserted_at.eq(excluded(ti::inserted_at)),
                )),
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
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (txns, user_txns, bm_txns, events, write_set_changes) =
            TransactionModel::from_transactions(&transactions);

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
