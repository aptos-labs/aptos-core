// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::LATEST_PROCESSED_VERSION,
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{
        errors::BlockProcessingError,
        processing_result::ProcessingResult,
        substream_processor::{get_conn, SubstreamProcessor},
    },
    models::{
        events::EventModel,
        signatures::Signature,
        transactions::{
            BlockMetadataTransactionModel, TransactionDetail, TransactionModel, UserTransaction,
        },
        write_set_changes::{
            MoveModule, MoveResource, TableItem, TableMetadata, WriteSetChangeDetail,
            WriteSetChangeModel,
        },
    },
    proto::{module_output::Data as ModuleOutputData, BlockScopedData},
    schema,
};
use anyhow::format_err;
use aptos_protos::block_output::v1::BlockOutput;
use async_trait::async_trait;
use diesel::ExpressionMethods;
use diesel::{pg::upsert::excluded, result::Error};
use field_count::FieldCount;
use prost::Message;
use std::fmt::Debug;

pub struct BlockOutputSubstreamProcessor {
    connection_pool: PgDbPool,
    is_chain_id_verified: bool,
}

impl BlockOutputSubstreamProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self {
            connection_pool,
            is_chain_id_verified: false,
        }
    }
}

impl Debug for BlockOutputSubstreamProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "BlockOutputSubstreamProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

/// This will insert all events within all transactions within a certain block
fn handle_block(
    conn: &mut PgPoolConnection,
    substream_name: &'static str,
    block_height: u64,
    txns: Vec<TransactionModel>,
    txn_details: Vec<TransactionDetail>,
    events: Vec<EventModel>,
    wscs: Vec<WriteSetChangeModel>,
    wsc_details: Vec<WriteSetChangeDetail>,
) -> Result<(), Error> {
    aptos_logger::trace!("[{}] inserting block {}", substream_name, block_height);
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

/// This will insert all transactions within a certain block
fn insert_transactions(conn: &mut PgPoolConnection, txns: &[TransactionModel]) {
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
                    block_height.eq(excluded(block_height)),
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

/// This will insert all user transactions within a block
fn insert_user_transactions_w_sigs(conn: &mut PgPoolConnection, txn_details: &[TransactionDetail]) {
    use schema::{signatures::dsl as sig_schema, user_transactions::dsl as ut_schema};
    let mut all_signatures = vec![];
    let mut all_user_transactions = vec![];
    for detail in txn_details {
        if let TransactionDetail::User(user_txn, sigs) = detail {
            all_signatures.append(&mut sigs.clone());
            all_user_transactions.push(user_txn.clone());
        }
    }
    let chunks = get_chunks(all_user_transactions.len(), UserTransaction::field_count());
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
                    sig_schema::transaction_block_height
                        .eq(excluded(sig_schema::transaction_block_height)),
                    sig_schema::signer.eq(excluded(sig_schema::signer)),
                    sig_schema::type_.eq(excluded(sig_schema::type_)),
                    sig_schema::public_key.eq(excluded(sig_schema::public_key)),
                    sig_schema::threshold.eq(excluded(sig_schema::threshold)),
                    sig_schema::public_key_indices.eq(excluded(sig_schema::public_key_indices)),
                    sig_schema::inserted_at.eq(excluded(sig_schema::inserted_at)),
                )),
        )
        .expect("Error inserting signatures into database");
    }
}

/// This will insert all block metadata transactions within a block
fn insert_block_metadata_transactions(
    conn: &mut PgPoolConnection,
    txn_details: &[TransactionDetail],
) {
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

/// This will insert all events within each transaction within a block
fn insert_events(conn: &mut PgPoolConnection, ev: &Vec<EventModel>) {
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
                    transaction_block_height.eq(excluded(transaction_block_height)),
                    type_.eq(excluded(type_)),
                    type_str.eq(excluded(type_str)),
                    data.eq(excluded(data)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
        )
        .expect("Error inserting events into database");
    }
}

/// This will insert all write set changes within each transaction within a block
fn insert_write_set_changes(conn: &mut PgPoolConnection, wscs: &Vec<WriteSetChangeModel>) {
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
                    transaction_block_height.eq(excluded(transaction_block_height)),
                    type_.eq(excluded(type_)),
                    address.eq(excluded(address)),
                    hash.eq(excluded(hash)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
        )
        .expect("Error inserting write set changes into database");
    }
}

/// This will insert all move modules within each transaction within a block
fn insert_move_modules(conn: &mut PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
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
                    transaction_block_height.eq(excluded(transaction_block_height)),
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

/// This will insert all move resources within each transaction within a block
fn insert_move_resources(conn: &mut PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
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
                    transaction_block_height.eq(excluded(transaction_block_height)),
                    name.eq(excluded(name)),
                    address.eq(excluded(address)),
                    type_str.eq(excluded(type_str)),
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
fn insert_table_data(conn: &mut PgPoolConnection, wsc_details: &[WriteSetChangeDetail]) {
    use schema::{table_items::dsl as ti, table_metadatas::dsl as tm};

    let (items, mut metadata): (Vec<TableItem>, Vec<TableMetadata>) = wsc_details
        .iter()
        .filter_map(|detail| match detail {
            WriteSetChangeDetail::Table(table_item, table_metadata) => {
                Some((table_item.clone(), table_metadata.clone()))
            }
            _ => None,
        })
        .collect::<Vec<(TableItem, TableMetadata)>>()
        .into_iter()
        .unzip();
    metadata.dedup_by(|a, b| a.handle == b.handle);
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
                    ti::transaction_block_height.eq(excluded(ti::transaction_block_height)),
                    ti::table_handle.eq(excluded(ti::table_handle)),
                    ti::decoded_key.eq(excluded(ti::decoded_key)),
                    ti::decoded_value.eq(excluded(ti::decoded_value)),
                    ti::is_deleted.eq(excluded(ti::is_deleted)),
                    ti::inserted_at.eq(excluded(ti::inserted_at)),
                )),
        )
        .expect("Error inserting table items into database");
    }
    let chunks = get_chunks(metadata.len(), TableMetadata::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::table_metadatas::table)
                .values(&metadata[start_ind..end_ind])
                .on_conflict(tm::handle)
                .do_nothing(),
        )
        .expect("Error inserting table metadata into database");
    }
}

#[async_trait]
impl SubstreamProcessor for BlockOutputSubstreamProcessor {
    fn substream_module_name(&self) -> &'static str {
        "block_to_block_output"
    }

    fn is_chain_id_verified(&self) -> bool {
        self.is_chain_id_verified
    }

    fn set_is_chain_id_verified(&mut self) {
        self.is_chain_id_verified = true;
    }

    async fn process_substream(
        &mut self,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError> {
        let output = stream_data.outputs.first().ok_or_else(|| {
            BlockProcessingError::ParsingError((
                format_err!("expecting one module output"),
                block_height,
                self.substream_module_name(),
            ))
        })?;
        // This is the expected output of the substream
        let block_output: BlockOutput = match output.data.as_ref().unwrap() {
            ModuleOutputData::MapOutput(data) => {
                aptos_logger::debug!(block_height = block_height, "Parsing mapper for block");
                Message::decode(data.value.as_slice()).map_err(|err| {
                    BlockProcessingError::ParsingError((
                        anyhow::Error::from(err),
                        block_height,
                        self.substream_module_name(),
                    ))
                })?
            }
            ModuleOutputData::StoreDeltas(_) => {
                return Err(BlockProcessingError::ParsingError((
                    format_err!("invalid module output StoreDeltas, expecting MapOutput"),
                    block_height,
                    self.substream_module_name(),
                )));
            }
        };

        if block_height != block_output.height {
            panic!(
                "We should be on block {}, but received block {}",
                block_height, block_output.height
            );
        }
        if !self.is_chain_id_verified() {
            let input_chain_id = block_output.chain_id;
            self.check_or_update_chain_id(input_chain_id as i64);
        }

        let (txns, txn_details, events, wscs, wsc_details) =
            TransactionModel::from_transactions(&block_output.transactions);
        let last_version = txns.last().unwrap().version;

        let conn = get_conn(self.connection_pool());
        let tx_result = handle_block(
            &conn,
            self.substream_module_name(),
            block_height,
            txns,
            txn_details,
            events,
            wscs,
            wsc_details,
        );

        match tx_result {
            Ok(_) => {
                LATEST_PROCESSED_VERSION
                    .with_label_values(&[self.substream_module_name()])
                    .set(last_version);
                Ok(ProcessingResult::new(
                    self.substream_module_name(),
                    block_height,
                ))
            }
            Err(err) => Err(BlockProcessingError::BlockCommitError((
                anyhow::Error::from(err),
                block_height,
                self.substream_module_name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
