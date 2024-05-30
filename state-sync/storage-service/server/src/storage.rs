// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, metrics};
use aptos_config::config::StorageServiceConfig;
use aptos_logger::debug;
use aptos_storage_interface::{AptosDbError, DbReader, Result as StorageResult};
use aptos_storage_service_types::responses::{
    CompleteDataRange, DataResponse, DataSummary, TransactionOrOutputListWithProof,
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    proof::{TransactionAccumulatorRangeProof, TransactionInfoListWithProof},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionListWithProof,
        TransactionOutput, TransactionOutputListWithProof, Version,
    },
    write_set::WriteSet,
};
use serde::Serialize;
use std::{cmp::min, sync::Arc, time::Instant};

/// The interface into local storage (e.g., the Aptos DB) used by the storage
/// server to handle client requests and responses.
pub trait StorageReaderInterface: Clone + Send + 'static {
    /// Returns a data summary of the underlying storage state.
    fn get_data_summary(&self) -> aptos_storage_service_types::Result<DataSummary, Error>;

    /// Returns a list of transactions with a proof relative to the
    /// `proof_version`. The transaction list is expected to start at
    /// `start_version` and end at `end_version` (inclusive). In some cases,
    /// less transactions may be returned (e.g., due to network or chunk
    /// limits). If `include_events` is true, events are also returned.
    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
    ) -> aptos_storage_service_types::Result<TransactionListWithProof, Error>;

    /// Returns a list of epoch ending ledger infos, starting at `start_epoch`
    /// and ending at the `expected_end_epoch` (inclusive). For example, if
    /// `start_epoch` is 0 and `end_epoch` is 1, this will return 2 epoch
    /// ending ledger infos (ending epoch 0 and 1, respectively).  In some
    /// cases, less ledger infos may be returned (e.g., due to network or
    /// chunk limits).
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> aptos_storage_service_types::Result<EpochChangeProof, Error>;

    /// Returns a list of transaction outputs with a proof relative to the
    /// `proof_version`. The transaction output list is expected to start at
    /// `start_version` and end at `end_version` (inclusive). In some cases,
    /// less transaction outputs may be returned (e.g., due to network or
    /// chunk limits).
    fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> aptos_storage_service_types::Result<TransactionOutputListWithProof, Error>;

    /// Returns a list of transaction or outputs with a proof relative to the
    /// `proof_version`. The data list is expected to start at `start_version`
    /// and end at `end_version` (inclusive). In some cases, less data may be
    /// returned (e.g., due to network or chunk limits). If `include_events`
    /// is true, events are also returned. `max_num_output_reductions` specifies
    /// how many output reductions can occur before transactions are returned.
    fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_num_output_reductions: u64,
    ) -> aptos_storage_service_types::Result<TransactionOrOutputListWithProof, Error>;

    /// Returns the number of states in the state tree at the specified version.
    fn get_number_of_states(&self, version: u64)
        -> aptos_storage_service_types::Result<u64, Error>;

    /// Returns a chunk holding a list of state values starting at the
    /// specified `start_index` and ending at `end_index` (inclusive). In
    /// some cases, less state values may be returned (e.g., due to network
    /// or chunk limits).
    fn get_state_value_chunk_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> aptos_storage_service_types::Result<StateValueChunkWithProof, Error>;
}

/// The underlying implementation of the StorageReaderInterface, used by the
/// storage server.
#[derive(Clone)]
pub struct StorageReader {
    config: StorageServiceConfig,
    storage: Arc<dyn DbReader>,
    time_service: TimeService,
}

impl StorageReader {
    pub fn new(
        config: StorageServiceConfig,
        storage: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        // Create a timed storage reader
        let storage = Arc::new(TimedStorageReader::new(storage));

        Self {
            config,
            storage,
            time_service,
        }
    }

    /// Returns the state values range held in the database (lowest to highest).
    /// Note: it is currently assumed that if a node contains a transaction at a
    /// version, V, the node also contains all state values at V.
    fn fetch_state_values_range(
        &self,
        latest_version: Version,
        transactions_range: &Option<CompleteDataRange<Version>>,
    ) -> aptos_storage_service_types::Result<Option<CompleteDataRange<Version>>, Error> {
        let pruner_enabled = self.storage.is_state_merkle_pruner_enabled()?;
        if !pruner_enabled {
            return Ok(*transactions_range);
        }
        let pruning_window = self.storage.get_epoch_snapshot_prune_window()?;

        if latest_version > pruning_window as Version {
            // lowest_state_version = latest_version - pruning_window + 1;
            let mut lowest_state_version = latest_version
                .checked_sub(pruning_window as Version)
                .ok_or_else(|| {
                    Error::UnexpectedErrorEncountered("Lowest state version has overflown!".into())
                })?;
            lowest_state_version = lowest_state_version.checked_add(1).ok_or_else(|| {
                Error::UnexpectedErrorEncountered("Lowest state version has overflown!".into())
            })?;

            // Create the state range
            let state_range = CompleteDataRange::new(lowest_state_version, latest_version)
                .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
            return Ok(Some(state_range));
        }

        // No pruning has occurred. Return the transactions range.
        Ok(*transactions_range)
    }

    /// Returns the transaction range held in the database (lowest to highest).
    fn fetch_transaction_range(
        &self,
        latest_version: Version,
    ) -> aptos_storage_service_types::Result<Option<CompleteDataRange<Version>>, Error> {
        let first_transaction_version = self.storage.get_first_txn_version()?;
        if let Some(first_transaction_version) = first_transaction_version {
            let transaction_range =
                CompleteDataRange::new(first_transaction_version, latest_version)
                    .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
            Ok(Some(transaction_range))
        } else {
            Ok(None)
        }
    }

    /// Returns the transaction output range held in the database (lowest to highest).
    fn fetch_transaction_output_range(
        &self,
        latest_version: Version,
    ) -> aptos_storage_service_types::Result<Option<CompleteDataRange<Version>>, Error> {
        let first_output_version = self.storage.get_first_write_set_version()?;
        if let Some(first_output_version) = first_output_version {
            let output_range = CompleteDataRange::new(first_output_version, latest_version)
                .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
            Ok(Some(output_range))
        } else {
            Ok(None)
        }
    }

    /// Returns true iff the serialized data size has exceeded the maximum allowed
    fn overflow_network_chunk_size(&self, serialized_data_size: u64) -> bool {
        serialized_data_size >= self.config.max_network_chunk_bytes
    }

    /// Returns true iff the storage read time has exceeded the maximum allowed duration
    fn overflow_storage_read_time(&self, storage_read_start_time: &Instant) -> bool {
        // Calculate the duration of the storage read so far
        let time_now = self.time_service.now();
        let storage_read_duration_ms = time_now
            .duration_since(*storage_read_start_time)
            .as_millis();

        // Check if the storage read duration has exceeded the maximum
        storage_read_duration_ms >= self.config.max_total_storage_read_time_ms
    }

    /// Updates the truncation logs and metrics based on the given reason and data response
    fn update_truncation_logs_and_metrics(
        &self,
        data_response_label: &str,
        num_items_to_fetch: u64,
        num_items_fetched: usize,
        serialized_data_size: u64,
        storage_read_start_time: Instant,
    ) {
        // Only update the logs and metrics if the data was truncated
        if num_items_to_fetch as usize <= num_items_fetched {
            return;
        }

        // Log the truncated data chunk
        debug!(
            "The response for {:?} was truncated! Num expected items: {:?}, num actual items: {:?}, \
                total serialized size: {:?} bytes, total storage read duration: {:?} ms!",
            data_response_label,
            num_items_to_fetch,
            num_items_fetched,
            serialized_data_size,
            storage_read_start_time.elapsed().as_millis(),
        );

        // Update the metrics, based on the truncation reason
        let truncation_reason = if self.overflow_network_chunk_size(serialized_data_size) {
            metrics::TRUNCATION_FOR_SIZE
        } else {
            metrics::TRUNCATION_FOR_TIME
        };
        metrics::increment_chunk_truncation_counter(truncation_reason, data_response_label);
    }
}

impl StorageReaderInterface for StorageReader {
    fn get_data_summary(&self) -> aptos_storage_service_types::Result<DataSummary, Error> {
        // Fetch the latest ledger info
        let latest_ledger_info_with_sigs = self.storage.get_latest_ledger_info()?;

        // Fetch the epoch ending ledger info range
        let latest_ledger_info = latest_ledger_info_with_sigs.ledger_info();
        let epoch_ending_ledger_infos = if latest_ledger_info.ends_epoch() {
            let highest_ending_epoch = latest_ledger_info.epoch();
            Some(CompleteDataRange::from_genesis(highest_ending_epoch))
        } else if latest_ledger_info.epoch() > 0 {
            let highest_ending_epoch =
                latest_ledger_info.epoch().checked_sub(1).ok_or_else(|| {
                    Error::UnexpectedErrorEncountered("Highest ending epoch overflowed!".into())
                })?;
            Some(CompleteDataRange::from_genesis(highest_ending_epoch))
        } else {
            None // We haven't seen an epoch change yet
        };

        // Fetch the transaction and transaction output ranges
        let latest_version = latest_ledger_info.version();
        let transactions = self.fetch_transaction_range(latest_version)?;
        let transaction_outputs = self.fetch_transaction_output_range(latest_version)?;

        // Fetch the state values range
        let states = self.fetch_state_values_range(latest_version, &transactions)?;

        // Return the relevant data summary
        let data_summary = DataSummary {
            synced_ledger_info: Some(latest_ledger_info_with_sigs),
            epoch_ending_ledger_infos,
            transactions,
            transaction_outputs,
            states,
        };

        Ok(data_summary)
    }

    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
    ) -> aptos_storage_service_types::Result<TransactionListWithProof, Error> {
        // Calculate the number of transactions to fetch
        let expected_num_transactions = inclusive_range_len(start_version, end_version)?;
        let max_num_transactions = self.config.max_transaction_chunk_size;
        let num_transactions_to_fetch = min(expected_num_transactions, max_num_transactions);

        // Get the iterators for the transaction, info and events
        let mut transaction_iterator = self
            .storage
            .get_transaction_iterator(start_version, num_transactions_to_fetch)?;
        let mut transaction_info_iterator = self
            .storage
            .get_transaction_info_iterator(start_version, num_transactions_to_fetch)?;
        let mut transaction_events_iterator = self
            .storage
            .get_events_iterator(start_version, num_transactions_to_fetch)?;

        // Initialize the fetched data items
        let mut transactions = vec![];
        let mut transaction_infos = vec![];
        let mut transaction_events = vec![];

        // Initialize the serialized data size and storage read start time
        let mut serialized_data_size = 0;
        let storage_read_start_time = self.time_service.now();

        // Fetch as many transactions, infos and events as possible without
        // exceeding the network chunk size or the max storage read time.
        while transactions.len() < num_transactions_to_fetch as usize
            && !self.overflow_network_chunk_size(serialized_data_size)
            && !self.overflow_storage_read_time(&storage_read_start_time)
        {
            // Process the next set of data items
            match (
                transaction_iterator.next(),
                transaction_info_iterator.next(),
                transaction_events_iterator.next(),
            ) {
                (Some(Ok(transaction)), Some(Ok(info)), Some(Ok(events))) => {
                    // Calculate the number of serialized bytes for the data items
                    let num_transaction_bytes = get_num_serialized_bytes(&transaction)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_info_bytes = get_num_serialized_bytes(&info)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_events_bytes = get_num_serialized_bytes(&events)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the transaction, info and events to the lists
                    transactions.push(transaction);
                    transaction_infos.push(info);
                    transaction_events.push(events);

                    // Update the total serialized data size
                    serialized_data_size +=
                        num_transaction_bytes + num_info_bytes + num_events_bytes;
                },
                (Some(Err(error)), _, _) | (_, Some(Err(error)), _) | (_, _, Some(Err(error))) => {
                    // One of the iterators encountered an error
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                (None, None, None) => {
                    break; // No more data items to fetch
                },
                (transaction, info, events) => {
                    return Err(Error::UnexpectedErrorEncountered(format!(
                        "The transaction, info and events iterators are out of sync! \
                            Transaction: {:?}, transaction info: {:?}, transaction events: {:?}",
                        transaction, info, events,
                    )));
                },
            }
        }

        // Create the transaction info list with proof
        let num_fetched_transactions = transactions.len();
        let accumulator_range_proof = self.storage.get_transaction_accumulator_range_proof(
            start_version,
            num_fetched_transactions as u64,
            proof_version,
        )?;
        let info_list_with_proof =
            TransactionInfoListWithProof::new(accumulator_range_proof, transaction_infos);

        // Create the transaction list with proof
        let transaction_events = if include_events {
            Some(transaction_events)
        } else {
            None
        };
        let transaction_list_with_proof = TransactionListWithProof::new(
            transactions,
            transaction_events,
            Some(start_version),
            info_list_with_proof,
        );

        // Update the truncation logs and metrics
        self.update_truncation_logs_and_metrics(
            DataResponse::get_transactions_with_proof_label(),
            num_transactions_to_fetch,
            num_fetched_transactions,
            serialized_data_size,
            storage_read_start_time,
        );

        Ok(transaction_list_with_proof)
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> aptos_storage_service_types::Result<EpochChangeProof, Error> {
        // Calculate the number of ledger infos to fetch
        let expected_num_ledger_infos = inclusive_range_len(start_epoch, expected_end_epoch)?;
        let max_num_ledger_infos = self.config.max_epoch_chunk_size;
        let num_ledger_infos_to_fetch = min(expected_num_ledger_infos, max_num_ledger_infos);

        // Calculate the end epoch for storage. This is required because the DbReader
        // interface returns the epochs up to: `end_epoch - 1`. However, we wish to
        // fetch epoch endings up to expected_end_epoch (inclusive).
        let end_epoch = start_epoch
            .checked_add(num_ledger_infos_to_fetch)
            .ok_or_else(|| Error::UnexpectedErrorEncountered("End epoch has overflown!".into()))?;

        // Get the epoch ending ledger info iterator
        let mut epoch_ending_ledger_info_iterator = self
            .storage
            .get_epoch_ending_ledger_info_iterator(start_epoch, end_epoch)?;

        // Initialize the fetched epoch ending ledger infos
        let mut epoch_ending_ledger_infos = vec![];

        // Initialize the serialized data size and storage read start time
        let mut serialized_data_size = 0;
        let storage_read_start_time = self.time_service.now();

        // Fetch as many epoch ending ledger infos as possible without exceeding the
        // network chunk size or the max storage read time.
        while epoch_ending_ledger_infos.len() < num_ledger_infos_to_fetch as usize
            && !self.overflow_network_chunk_size(serialized_data_size)
            && !self.overflow_storage_read_time(&storage_read_start_time)
        {
            // Process the next epoch ending ledger info
            match epoch_ending_ledger_info_iterator.next() {
                Some(Ok(epoch_ending_ledger_info)) => {
                    // Calculate the number of serialized bytes for the epoch ending ledger info
                    let num_serialized_bytes = get_num_serialized_bytes(&epoch_ending_ledger_info)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the ledger info to the list and update the serialized data size
                    epoch_ending_ledger_infos.push(epoch_ending_ledger_info);

                    // Update the total serialized data size
                    serialized_data_size += num_serialized_bytes;
                },
                Some(Err(error)) => {
                    // The iterator encountered an error
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    break; // No more epoch ending ledger infos to fetch
                },
            }
        }

        // Create the epoch change proof
        let epoch_change_proof = EpochChangeProof::new(epoch_ending_ledger_infos, false);

        // Update the truncation logs and metrics
        let num_fetched_ledger_infos = epoch_change_proof.ledger_info_with_sigs.len();
        self.update_truncation_logs_and_metrics(
            DataResponse::get_epoch_ending_ledger_info_label(),
            num_ledger_infos_to_fetch,
            num_fetched_ledger_infos,
            serialized_data_size,
            storage_read_start_time,
        );

        Ok(epoch_change_proof)
    }

    fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> aptos_storage_service_types::Result<TransactionOutputListWithProof, Error> {
        // Calculate the number of transaction outputs to fetch
        let expected_num_outputs = inclusive_range_len(start_version, end_version)?;
        let max_num_outputs = self.config.max_transaction_output_chunk_size;
        let num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // Get the iterators for the transaction, info, write set, events and auxiliary data
        let mut transaction_iterator = self
            .storage
            .get_transaction_iterator(start_version, num_outputs_to_fetch)?;
        let mut transaction_info_iterator = self
            .storage
            .get_transaction_info_iterator(start_version, num_outputs_to_fetch)?;
        let mut transaction_write_set_iterator = self
            .storage
            .get_write_set_iterator(start_version, num_outputs_to_fetch)?;
        let mut transaction_events_iterator = self
            .storage
            .get_events_iterator(start_version, num_outputs_to_fetch)?;
        let mut transaction_auxiliary_data_iterator = self
            .storage
            .get_auxiliary_data_iterator(start_version, num_outputs_to_fetch)?;

        // Initialize the fetched data items
        let mut transactions_and_outputs = vec![];
        let mut transaction_infos = vec![];

        // Initialize the serialized data size and storage read start time
        let mut serialized_data_size = 0;
        let storage_read_start_time = self.time_service.now();

        // Fetch as many transactions, infos, write sets, events and auxiliary data as
        // possible without exceeding the network chunk size or the max storage read time.
        while transactions_and_outputs.len() < num_outputs_to_fetch as usize
            && !self.overflow_network_chunk_size(serialized_data_size)
            && !self.overflow_storage_read_time(&storage_read_start_time)
        {
            // Process the next set of data items
            match (
                transaction_iterator.next(),
                transaction_info_iterator.next(),
                transaction_write_set_iterator.next(),
                transaction_events_iterator.next(),
                transaction_auxiliary_data_iterator.next(),
            ) {
                (
                    Some(Ok(transaction)),
                    Some(Ok(info)),
                    Some(Ok(write_set)),
                    Some(Ok(events)),
                    Some(Ok(auxiliary_data)),
                ) => {
                    // Create the transaction output
                    let output = TransactionOutput::new(
                        write_set,
                        events,
                        info.gas_used(),
                        info.status().clone().into(),
                        auxiliary_data,
                    );

                    // Calculate the number of serialized bytes for the data items
                    let num_transaction_bytes = get_num_serialized_bytes(&transaction)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_info_bytes = get_num_serialized_bytes(&info)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_output_bytes = get_num_serialized_bytes(&output)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the data items to the lists
                    transactions_and_outputs.push((transaction, output));
                    transaction_infos.push(info);

                    // Update the total serialized data size
                    serialized_data_size +=
                        num_transaction_bytes + num_info_bytes + num_output_bytes;
                },
                (Some(Err(error)), _, _, _, _)
                | (_, Some(Err(error)), _, _, _)
                | (_, _, Some(Err(error)), _, _)
                | (_, _, _, Some(Err(error)), _)
                | (_, _, _, _, Some(Err(error))) => {
                    // One of the iterators encountered an error
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                (None, None, None, None, None) => {
                    break; // No more data items to fetch
                },
                (transaction, info, write_set, events, auxiliary_data) => {
                    return Err(Error::UnexpectedErrorEncountered(format!(
                        "The transaction, info, write set, events and auxiliary data iterators are out of sync! \
                            Transaction: {:?}, transaction info: {:?}, transaction write set: {:?}, \
                            transaction events: {:?}, transaction auxiliary data: {:?}",
                        transaction, info, write_set, events, auxiliary_data,
                    )));
                },
            }
        }

        // Create the transaction output list with proof
        let num_fetched_outputs = transactions_and_outputs.len();
        let accumulator_range_proof = self.storage.get_transaction_accumulator_range_proof(
            start_version,
            num_fetched_outputs as u64,
            proof_version,
        )?;
        let transaction_info_list_with_proof =
            TransactionInfoListWithProof::new(accumulator_range_proof, transaction_infos);
        let transaction_output_list_with_proof = TransactionOutputListWithProof::new(
            transactions_and_outputs,
            Some(start_version),
            transaction_info_list_with_proof,
        );

        // Update the truncation logs and metrics
        let num_fetched_outputs = transaction_output_list_with_proof
            .transactions_and_outputs
            .len();
        self.update_truncation_logs_and_metrics(
            DataResponse::get_transaction_outputs_with_proof_label(),
            num_outputs_to_fetch,
            num_fetched_outputs,
            serialized_data_size,
            storage_read_start_time,
        );

        Ok(transaction_output_list_with_proof)
    }

    fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_num_output_reductions: u64,
    ) -> aptos_storage_service_types::Result<TransactionOrOutputListWithProof, Error> {
        // Calculate the number of transaction outputs to fetch
        let expected_num_outputs = inclusive_range_len(start_version, end_version)?;
        let max_num_outputs = self.config.max_transaction_output_chunk_size;
        let num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // Get the transaction outputs with proof
        let output_list_with_proof =
            self.get_transaction_outputs_with_proof(proof_version, start_version, end_version)?;

        // If the number of transaction outputs is too small, return transactions instead.
        // The minimum number of outputs we can return is defined as the max number of
        // times the data can be halved before transactions are returned.
        let num_fetched_outputs = output_list_with_proof.transactions_and_outputs.len();
        let output_reduction_factor = 2_u64
            .checked_pow(max_num_output_reductions as u32)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered("Max num output divisions has overflown!".into())
            })?;
        let min_num_outputs = num_outputs_to_fetch
            .checked_div(output_reduction_factor)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered("Num min outputs has overflown!".into())
            })?;
        if num_fetched_outputs < min_num_outputs as usize {
            // Too few outputs were fetched. Fetch and return transactions instead.
            let transactions_with_proof = self.get_transactions_with_proof(
                proof_version,
                start_version,
                end_version,
                include_events,
            )?;
            Ok((Some(transactions_with_proof), None))
        } else {
            // The number of outputs fetched is sufficient. Return the outputs.
            Ok((None, Some(output_list_with_proof)))
        }
    }

    fn get_number_of_states(
        &self,
        version: u64,
    ) -> aptos_storage_service_types::Result<u64, Error> {
        let number_of_states = self.storage.get_state_leaf_count(version)?;
        Ok(number_of_states as u64)
    }

    fn get_state_value_chunk_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> aptos_storage_service_types::Result<StateValueChunkWithProof, Error> {
        // Calculate the number of state values to fetch
        let expected_num_state_values = inclusive_range_len(start_index, end_index)?;
        let max_num_state_values = self.config.max_state_chunk_size;
        let num_state_values_to_fetch = min(expected_num_state_values, max_num_state_values);

        // Get the state value chunk iterator
        let mut state_value_iterator = self.storage.get_state_value_chunk_iter(
            version,
            start_index as usize,
            num_state_values_to_fetch as usize,
        )?;

        // Initialize the fetched state values
        let mut state_values = vec![];

        // Initialize the serialized data size and storage read start time
        let mut serialized_data_size = 0;
        let storage_read_start_time = self.time_service.now();

        // Fetch as many state values as possible without exceeding the
        // network chunk size or the max storage read time.
        while state_values.len() < num_state_values_to_fetch as usize
            && !self.overflow_network_chunk_size(serialized_data_size)
            && !self.overflow_storage_read_time(&storage_read_start_time)
        {
            // Process the next state value
            match state_value_iterator.next() {
                Some(Ok(state_value)) => {
                    // Calculate the number of serialized bytes for the state value
                    let num_serialized_bytes = get_num_serialized_bytes(&state_value)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the state value to the list
                    state_values.push(state_value);

                    // Update the total serialized data size
                    serialized_data_size += num_serialized_bytes;
                },
                Some(Err(error)) => {
                    // The iterator encountered an error
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    break; // No more state values to fetch
                },
            }
        }

        // Create the state value chunk with proof
        let state_value_chunk_with_proof = self.storage.get_state_value_chunk_proof(
            version,
            start_index as usize,
            state_values,
        )?;

        // If the data was truncated, update the logs and metrics
        let num_fetch_state_values = state_value_chunk_with_proof.raw_values.len();
        self.update_truncation_logs_and_metrics(
            DataResponse::get_state_value_chunk_with_proof_label(),
            num_state_values_to_fetch,
            num_fetch_state_values,
            serialized_data_size,
            storage_read_start_time,
        );

        Ok(state_value_chunk_with_proof)
    }
}

// A simple macro that wraps each storage read call with a timer
macro_rules! timed_read {
    ($(
        $(#[$($attr:meta)*])*
        fn $name:ident(&self $(, $arg: ident : $ty: ty $(,)?)*) -> $return_type:ty;
    )+) => {
        $(
            $(#[$($attr)*])*
            fn $name(&self, $($arg: $ty),*) -> $return_type {
                let read_operation = || {
                    self.storage.$name($($arg),*).map_err(|e| e.into())
                };
                let result = crate::utils::execute_and_time_duration(
                    &crate::metrics::STORAGE_DB_READ_LATENCY,
                    None,
                    Some(stringify!($name).into()),
                    read_operation,
                    None,
                );
                result.map_err(|e| AptosDbError::Other(e.to_string()))
            }
        )+
    };
}

/// A simple wrapper around a DbReader that implements and
/// times the required interface calls.
pub struct TimedStorageReader {
    storage: Arc<dyn DbReader>,
}

impl TimedStorageReader {
    pub fn new(storage: Arc<dyn DbReader>) -> Self {
        Self { storage }
    }
}

impl DbReader for TimedStorageReader {
    timed_read!(
        fn is_state_merkle_pruner_enabled(&self) -> StorageResult<bool>;

        fn get_epoch_snapshot_prune_window(&self) -> StorageResult<usize>;

        fn get_first_txn_version(&self) -> StorageResult<Option<Version>>;

        fn get_first_write_set_version(&self) -> StorageResult<Option<Version>>;

        fn get_latest_ledger_info(&self) -> StorageResult<LedgerInfoWithSignatures>;

        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> StorageResult<TransactionListWithProof>;

        fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> StorageResult<EpochChangeProof>;

        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> StorageResult<TransactionOutputListWithProof>;

        fn get_state_leaf_count(&self, version: Version) -> StorageResult<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> StorageResult<StateValueChunkWithProof>;

        fn get_epoch_ending_ledger_info_iterator(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<LedgerInfoWithSignatures>> + '_>>;

        fn get_transaction_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<Transaction>> + '_>>;

        fn get_transaction_info_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<TransactionInfo>> + '_>>;

        fn get_events_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<Vec<ContractEvent>>> + '_>>;

        fn get_write_set_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<WriteSet>> + '_>>;

        fn get_auxiliary_data_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<TransactionAuxiliaryData>> + '_>>;

        fn get_transaction_accumulator_range_proof(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> StorageResult<TransactionAccumulatorRangeProof>;

        fn get_state_value_chunk_iter(
            &self,
            version: Version,
            first_index: usize,
            chunk_size: usize,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<(StateKey, StateValue)>> + '_>>;

        fn get_state_value_chunk_proof(
            &self,
            version: Version,
            first_index: usize,
            state_key_values: Vec<(StateKey, StateValue)>,
        ) -> StorageResult<StateValueChunkWithProof>;
    );
}

/// Calculate `(start..=end).len()`. Returns an error if `end < start` or
/// `end == u64::MAX`.
fn inclusive_range_len(start: u64, end: u64) -> aptos_storage_service_types::Result<u64, Error> {
    // len = end - start + 1
    let len = end.checked_sub(start).ok_or_else(|| {
        Error::InvalidRequest(format!("end ({}) must be >= start ({})", end, start))
    })?;
    let len = len
        .checked_add(1)
        .ok_or_else(|| Error::InvalidRequest(format!("end ({}) must not be u64::MAX", end)))?;
    Ok(len)
}

/// Serializes the given data and returns the number of serialized bytes
fn get_num_serialized_bytes<T: ?Sized + Serialize>(
    data: &T,
) -> aptos_storage_service_types::Result<u64, Error> {
    let num_serialized_bytes = bcs::serialized_size(data)
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
    Ok(num_serialized_bytes as u64)
}
