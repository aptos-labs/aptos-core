// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, metrics};
use aptos_config::config::StorageServiceConfig;
use aptos_logger::{debug, warn};
use aptos_storage_interface::{AptosDbError, DbReader, Result as StorageResult};
use aptos_storage_service_types::{
    requests::{GetTransactionDataWithProofRequest, TransactionDataRequestType},
    responses::{
        CompleteDataRange, DataResponse, DataSummary, TransactionDataResponseType,
        TransactionDataWithProofResponse,
    },
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        AccumulatorRangeProof, TransactionAccumulatorRangeProof, TransactionInfoListWithProof,
    },
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        PersistedAuxiliaryInfo, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionListWithAuxiliaryInfos, TransactionListWithProof, TransactionListWithProofV2,
        TransactionOutput, TransactionOutputListWithAuxiliaryInfos, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, Version,
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
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error>;

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
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error>;

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
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error>;

    /// Returns transaction data with a proof for the given request
    fn get_transaction_data_with_proof(
        &self,
        transaction_data_with_proof_request: &GetTransactionDataWithProofRequest,
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error>;

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

    /// Returns an epoch ending ledger info response (bound by the max response size in bytes)
    fn get_epoch_ending_ledger_infos_by_size(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
        max_response_size: u64,
        use_size_and_time_aware_chunking: bool,
    ) -> Result<EpochChangeProof, Error> {
        // Calculate the number of ledger infos to fetch
        let expected_num_ledger_infos = inclusive_range_len(start_epoch, expected_end_epoch)?;
        let max_num_ledger_infos = self.config.max_epoch_chunk_size;
        let num_ledger_infos_to_fetch = min(expected_num_ledger_infos, max_num_ledger_infos);

        // If size and time-aware chunking are disabled, use the legacy implementation
        if !use_size_and_time_aware_chunking {
            return self.get_epoch_ending_ledger_infos_by_size_legacy(
                start_epoch,
                expected_end_epoch,
                num_ledger_infos_to_fetch,
                max_response_size,
            );
        }

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

        // Create a response progress tracker
        let mut response_progress_tracker = ResponseDataProgressTracker::new(
            num_ledger_infos_to_fetch,
            max_response_size,
            self.config.max_storage_read_wait_time_ms,
            self.time_service.clone(),
        );

        // Fetch as many epoch ending ledger infos as possible
        while !response_progress_tracker.is_response_complete() {
            match epoch_ending_ledger_info_iterator.next() {
                Some(Ok(epoch_ending_ledger_info)) => {
                    // Calculate the number of serialized bytes for the epoch ending ledger info
                    let num_serialized_bytes = get_num_serialized_bytes(&epoch_ending_ledger_info)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the ledger info to the list
                    if response_progress_tracker
                        .data_items_fits_in_response(true, num_serialized_bytes)
                    {
                        epoch_ending_ledger_infos.push(epoch_ending_ledger_info);
                        response_progress_tracker.add_data_item(num_serialized_bytes);
                    } else {
                        break; // Cannot add any more data items
                    }
                },
                Some(Err(error)) => {
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    // Log a warning that the iterator did not contain all the expected data
                    warn!(
                        "The epoch ending ledger info iterator is missing data! \
                        Start epoch: {:?}, expected end epoch: {:?}, num ledger infos to fetch: {:?}",
                        start_epoch, expected_end_epoch, num_ledger_infos_to_fetch
                    );
                    break;
                },
            }
        }

        // Create the epoch change proof
        let epoch_change_proof = EpochChangeProof::new(epoch_ending_ledger_infos, false);

        // Update the data truncation metrics
        response_progress_tracker
            .update_data_truncation_metrics(DataResponse::get_epoch_ending_ledger_info_label());

        Ok(epoch_change_proof)
    }

    /// Returns an epoch ending ledger info response (bound by the max response size in bytes).
    /// This is the legacy implementation (that does not use size and time-aware chunking).
    fn get_epoch_ending_ledger_infos_by_size_legacy(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
        mut num_ledger_infos_to_fetch: u64,
        max_response_size: u64,
    ) -> Result<EpochChangeProof, Error> {
        while num_ledger_infos_to_fetch >= 1 {
            // The DbReader interface returns the epochs up to: `end_epoch - 1`.
            // However, we wish to fetch epoch endings up to end_epoch (inclusive).
            let end_epoch = start_epoch
                .checked_add(num_ledger_infos_to_fetch)
                .ok_or_else(|| {
                    Error::UnexpectedErrorEncountered("End epoch has overflown!".into())
                })?;
            let epoch_change_proof = self
                .storage
                .get_epoch_ending_ledger_infos(start_epoch, end_epoch)?;
            if num_ledger_infos_to_fetch == 1 {
                return Ok(epoch_change_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) =
                check_overflow_network_frame(&epoch_change_proof, max_response_size)?;
            if !overflow_frame {
                return Ok(epoch_change_proof);
            } else {
                metrics::increment_chunk_truncation_counter(
                    metrics::TRUNCATION_FOR_SIZE,
                    DataResponse::EpochEndingLedgerInfos(epoch_change_proof).get_label(),
                );
                let new_num_ledger_infos_to_fetch = num_ledger_infos_to_fetch / 2;
                debug!("The request for {:?} ledger infos was too large (num bytes: {:?}, limit: {:?}). Retrying with {:?}.",
                    num_ledger_infos_to_fetch, num_bytes, max_response_size, new_num_ledger_infos_to_fetch);
                num_ledger_infos_to_fetch = new_num_ledger_infos_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_epoch_ending_ledger_infos request! Start epoch: {:?}, \
            expected end epoch: {:?}. The data cannot fit into a single network frame!",
            start_epoch, expected_end_epoch
        )))
    }

    /// Returns a transaction with proof response (bound by the max response size in bytes)
    fn get_transactions_with_proof_by_size(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_response_size: u64,
        use_size_and_time_aware_chunking: bool,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        // Calculate the number of transactions to fetch
        let expected_num_transactions = inclusive_range_len(start_version, end_version)?;
        let max_num_transactions = self.config.max_transaction_chunk_size;
        let num_transactions_to_fetch = min(expected_num_transactions, max_num_transactions);

        // If size and time-aware chunking are disabled, use the legacy implementation
        if !use_size_and_time_aware_chunking {
            return self.get_transactions_with_proof_by_size_legacy(
                proof_version,
                start_version,
                end_version,
                num_transactions_to_fetch,
                include_events,
                max_response_size,
            );
        }

        // Get the iterators for the transaction, info, events and persisted auxiliary infos
        let transaction_iterator = self
            .storage
            .get_transaction_iterator(start_version, num_transactions_to_fetch)?;
        let transaction_info_iterator = self
            .storage
            .get_transaction_info_iterator(start_version, num_transactions_to_fetch)?;
        let transaction_events_iterator = if include_events {
            self.storage
                .get_events_iterator(start_version, num_transactions_to_fetch)?
        } else {
            // If events are not included, create a fake iterator (they will be dropped anyway)
            Box::new(std::iter::repeat(Ok(vec![])).take(num_transactions_to_fetch as usize))
        };
        let persisted_auxiliary_info_iterator =
            self.storage.get_persisted_auxiliary_info_iterator(
                start_version,
                num_transactions_to_fetch as usize,
            )?;

        let mut multizip_iterator = itertools::multizip((
            transaction_iterator,
            transaction_info_iterator,
            transaction_events_iterator,
            persisted_auxiliary_info_iterator,
        ));

        // Initialize the fetched data items
        let mut transactions = vec![];
        let mut transaction_infos = vec![];
        let mut transaction_events = vec![];
        let mut persisted_auxiliary_infos = vec![];

        // Create a response progress tracker
        let mut response_progress_tracker = ResponseDataProgressTracker::new(
            num_transactions_to_fetch,
            max_response_size,
            self.config.max_storage_read_wait_time_ms,
            self.time_service.clone(),
        );

        // Fetch as many transactions as possible
        while !response_progress_tracker.is_response_complete() {
            match multizip_iterator.next() {
                Some((Ok(transaction), Ok(info), Ok(events), Ok(persisted_auxiliary_info))) => {
                    // Calculate the number of serialized bytes for the data items
                    let num_transaction_bytes = get_num_serialized_bytes(&transaction)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_info_bytes = get_num_serialized_bytes(&info)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_events_bytes = get_num_serialized_bytes(&events)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                    let num_auxiliary_info_bytes =
                        get_num_serialized_bytes(&persisted_auxiliary_info).map_err(|error| {
                            Error::UnexpectedErrorEncountered(error.to_string())
                        })?;

                    // Add the data items to the lists
                    let total_serialized_bytes = num_transaction_bytes
                        + num_info_bytes
                        + num_events_bytes
                        + num_auxiliary_info_bytes;
                    if response_progress_tracker
                        .data_items_fits_in_response(true, total_serialized_bytes)
                    {
                        transactions.push(transaction);
                        transaction_infos.push(info);
                        transaction_events.push(events);
                        persisted_auxiliary_infos.push(persisted_auxiliary_info);

                        response_progress_tracker.add_data_item(total_serialized_bytes);
                    } else {
                        break; // Cannot add any more data items
                    }
                },
                Some((Err(error), _, _, _))
                | Some((_, Err(error), _, _))
                | Some((_, _, Err(error), _))
                | Some((_, _, _, Err(error))) => {
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    // Log a warning that the iterators did not contain all the expected data
                    warn!(
                        "The iterators for transactions, transaction infos, events and \
                        persisted auxiliary infos are missing data! Start version: {:?}, \
                        end version: {:?}, num transactions to fetch: {:?}, num fetched: {:?}.",
                        start_version,
                        end_version,
                        num_transactions_to_fetch,
                        transactions.len()
                    );
                    break;
                },
            }
        }

        // Create the transaction info list with proof
        let accumulator_range_proof = self.storage.get_transaction_accumulator_range_proof(
            start_version,
            transactions.len() as u64,
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

        // Update the data truncation metrics
        response_progress_tracker
            .update_data_truncation_metrics(DataResponse::get_transactions_with_proof_v2_label());

        // Create the transaction data with proof response
        let transaction_list_with_proof_v2 =
            TransactionListWithProofV2::new(TransactionListWithAuxiliaryInfos::new(
                transaction_list_with_proof,
                persisted_auxiliary_infos,
            ));
        let response = TransactionDataWithProofResponse {
            transaction_data_response_type: TransactionDataResponseType::TransactionData,
            transaction_list_with_proof: Some(transaction_list_with_proof_v2),
            transaction_output_list_with_proof: None,
        };
        Ok(response)
    }

    /// Returns a transaction with proof response (bound by the max response size in bytes).
    /// This is the legacy implementation (that does not use size and time-aware chunking).
    fn get_transactions_with_proof_by_size_legacy(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        mut num_transactions_to_fetch: u64,
        include_events: bool,
        max_response_size: u64,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        while num_transactions_to_fetch >= 1 {
            let transaction_list_with_proof = self.storage.get_transactions(
                start_version,
                num_transactions_to_fetch,
                proof_version,
                include_events,
            )?;
            let response = TransactionDataWithProofResponse {
                transaction_data_response_type: TransactionDataResponseType::TransactionData,
                transaction_list_with_proof: Some(transaction_list_with_proof),
                transaction_output_list_with_proof: None,
            };
            if num_transactions_to_fetch == 1 {
                return Ok(response); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) =
                check_overflow_network_frame(&response, max_response_size)?;
            if !overflow_frame {
                return Ok(response);
            } else {
                metrics::increment_chunk_truncation_counter(
                    metrics::TRUNCATION_FOR_SIZE,
                    DataResponse::TransactionDataWithProof(response).get_label(),
                );
                let new_num_transactions_to_fetch = num_transactions_to_fetch / 2;
                debug!("The request for {:?} transactions was too large (num bytes: {:?}, limit: {:?}). Retrying with {:?}.",
                    num_transactions_to_fetch, num_bytes, max_response_size, new_num_transactions_to_fetch);
                num_transactions_to_fetch = new_num_transactions_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_transactions_with_proof request! Proof version: {:?}, \
            start version: {:?}, end version: {:?}, include events: {:?}. The data cannot fit into \
            a single network frame!",
            proof_version, start_version, end_version, include_events,
        )))
    }

    /// Returns a transaction output with proof response (bound by the max response size in bytes)
    fn get_transaction_outputs_with_proof_by_size(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        max_response_size: u64,
        is_transaction_or_output_request: bool,
        use_size_and_time_aware_chunking: bool,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        // Calculate the number of transaction outputs to fetch
        let expected_num_outputs = inclusive_range_len(start_version, end_version)?;
        let max_num_outputs = self.config.max_transaction_output_chunk_size;
        let num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // If size and time-aware chunking are disabled, use the legacy implementation
        if !use_size_and_time_aware_chunking {
            return self.get_transaction_outputs_with_proof_by_size_legacy(
                proof_version,
                start_version,
                end_version,
                num_outputs_to_fetch,
                max_response_size,
            );
        }

        // Get the iterators for the transaction, info, write set, events,
        // auxiliary data and persisted auxiliary infos.
        let transaction_iterator = self
            .storage
            .get_transaction_iterator(start_version, num_outputs_to_fetch)?;
        let transaction_info_iterator = self
            .storage
            .get_transaction_info_iterator(start_version, num_outputs_to_fetch)?;
        let transaction_write_set_iterator = self
            .storage
            .get_write_set_iterator(start_version, num_outputs_to_fetch)?;
        let transaction_events_iterator = self
            .storage
            .get_events_iterator(start_version, num_outputs_to_fetch)?;
        let transaction_auxiliary_data_iterator = self
            .storage
            .get_auxiliary_data_iterator(start_version, num_outputs_to_fetch)?;
        let persisted_auxiliary_info_iterator = self
            .storage
            .get_persisted_auxiliary_info_iterator(start_version, num_outputs_to_fetch as usize)?;
        let mut multizip_iterator = itertools::multizip((
            transaction_iterator,
            transaction_info_iterator,
            transaction_write_set_iterator,
            transaction_events_iterator,
            transaction_auxiliary_data_iterator,
            persisted_auxiliary_info_iterator,
        ));

        // Initialize the fetched data items
        let mut transactions_and_outputs = vec![];
        let mut transaction_infos = vec![];
        let mut persisted_auxiliary_infos = vec![];

        // Create a response progress tracker
        let mut response_progress_tracker = ResponseDataProgressTracker::new(
            num_outputs_to_fetch,
            max_response_size,
            self.config.max_storage_read_wait_time_ms,
            self.time_service.clone(),
        );

        // Fetch as many transaction outputs as possible
        while !response_progress_tracker.is_response_complete() {
            match multizip_iterator.next() {
                Some((
                    Ok(transaction),
                    Ok(info),
                    Ok(write_set),
                    Ok(events),
                    Ok(auxiliary_data),
                    Ok(persisted_auxiliary_info),
                )) => {
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
                    let num_auxiliary_info_bytes =
                        get_num_serialized_bytes(&persisted_auxiliary_info).map_err(|error| {
                            Error::UnexpectedErrorEncountered(error.to_string())
                        })?;

                    // Add the data items to the lists
                    let total_serialized_bytes = num_transaction_bytes
                        + num_info_bytes
                        + num_output_bytes
                        + num_auxiliary_info_bytes;
                    if response_progress_tracker.data_items_fits_in_response(
                        !is_transaction_or_output_request,
                        total_serialized_bytes,
                    ) {
                        transactions_and_outputs.push((transaction, output));
                        transaction_infos.push(info);
                        persisted_auxiliary_infos.push(persisted_auxiliary_info);

                        response_progress_tracker.add_data_item(total_serialized_bytes);
                    } else {
                        break; // Cannot add any more data items
                    }
                },
                Some((Err(error), _, _, _, _, _))
                | Some((_, Err(error), _, _, _, _))
                | Some((_, _, Err(error), _, _, _))
                | Some((_, _, _, Err(error), _, _))
                | Some((_, _, _, _, Err(error), _))
                | Some((_, _, _, _, _, Err(error))) => {
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    // Log a warning that the iterators did not contain all the expected data
                    warn!(
                        "The iterators for transactions, transaction infos, write sets, events, \
                        auxiliary data and persisted auxiliary infos are missing data! Start version: {:?}, \
                        end version: {:?}, num outputs to fetch: {:?}, num fetched: {:?}.",
                        start_version, end_version, num_outputs_to_fetch, transactions_and_outputs.len()
                    );
                    break;
                },
            }
        }

        // Create the transaction output list with proof
        let num_fetched_outputs = transactions_and_outputs.len();
        let accumulator_range_proof = if num_fetched_outputs == 0 {
            AccumulatorRangeProof::new_empty() // Return an empty proof if no outputs were fetched
        } else {
            self.storage.get_transaction_accumulator_range_proof(
                start_version,
                num_fetched_outputs as u64,
                proof_version,
            )?
        };
        let transaction_info_list_with_proof =
            TransactionInfoListWithProof::new(accumulator_range_proof, transaction_infos);
        let transaction_output_list_with_proof = TransactionOutputListWithProof::new(
            transactions_and_outputs,
            Some(start_version),
            transaction_info_list_with_proof,
        );

        // Update the data truncation metrics
        response_progress_tracker.update_data_truncation_metrics(
            DataResponse::get_transaction_outputs_with_proof_v2_label(),
        );

        // Create the transaction data with proof response
        let output_list_with_proof_v2 =
            TransactionOutputListWithProofV2::new(TransactionOutputListWithAuxiliaryInfos::new(
                transaction_output_list_with_proof,
                persisted_auxiliary_infos,
            ));
        let response = TransactionDataWithProofResponse {
            transaction_data_response_type: TransactionDataResponseType::TransactionOutputData,
            transaction_list_with_proof: None,
            transaction_output_list_with_proof: Some(output_list_with_proof_v2),
        };

        Ok(response)
    }

    /// Returns a transaction output with proof response (bound by the max response size in bytes).
    /// This is the legacy implementation (that does not use size and time-aware chunking).
    fn get_transaction_outputs_with_proof_by_size_legacy(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        mut num_outputs_to_fetch: u64,
        max_response_size: u64,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        while num_outputs_to_fetch >= 1 {
            let output_list_with_proof = self.storage.get_transaction_outputs(
                start_version,
                num_outputs_to_fetch,
                proof_version,
            )?;
            let response = TransactionDataWithProofResponse {
                transaction_data_response_type: TransactionDataResponseType::TransactionOutputData,
                transaction_list_with_proof: None,
                transaction_output_list_with_proof: Some(output_list_with_proof),
            };
            if num_outputs_to_fetch == 1 {
                return Ok(response); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) =
                check_overflow_network_frame(&response, max_response_size)?;
            if !overflow_frame {
                return Ok(response);
            } else {
                metrics::increment_chunk_truncation_counter(
                    metrics::TRUNCATION_FOR_SIZE,
                    DataResponse::TransactionDataWithProof(response).get_label(),
                );
                let new_num_outputs_to_fetch = num_outputs_to_fetch / 2;
                debug!("The request for {:?} outputs was too large (num bytes: {:?}, limit: {:?}). Retrying with {:?}.",
                    num_outputs_to_fetch, num_bytes, max_response_size, new_num_outputs_to_fetch);
                num_outputs_to_fetch = new_num_outputs_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_transaction_outputs_with_proof request! Proof version: {:?}, \
            start version: {:?}, end version: {:?}. The data cannot fit into a single network frame!",
            proof_version, start_version, end_version
        )))
    }

    /// Returns a transaction or output with proof response (bound by the max response size in bytes)
    fn get_transactions_or_outputs_with_proof_by_size(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_num_output_reductions: u64,
        max_response_size: u64,
        use_size_and_time_aware_chunking: bool,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        // Calculate the number of transaction outputs to fetch
        let expected_num_outputs = inclusive_range_len(start_version, end_version)?;
        let max_num_outputs = self.config.max_transaction_output_chunk_size;
        let num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // If size and time-aware chunking are disabled, use the legacy implementation
        if !use_size_and_time_aware_chunking {
            return self.get_transactions_or_outputs_with_proof_by_size_legacy(
                proof_version,
                start_version,
                end_version,
                num_outputs_to_fetch,
                include_events,
                max_num_output_reductions,
                max_response_size,
            );
        }

        // Fetch the transaction outputs with proof
        let response = self.get_transaction_outputs_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            max_response_size,
            true, // This is a transaction or output request
            use_size_and_time_aware_chunking,
        )?;

        // If the request was fully satisfied (all items were fetched), return the response
        if let Some(output_list_with_proof) = response.transaction_output_list_with_proof.as_ref() {
            if num_outputs_to_fetch == output_list_with_proof.get_num_outputs() as u64 {
                return Ok(response);
            }
        }

        // Otherwise, return as many transactions as possible
        self.get_transactions_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            include_events,
            max_response_size,
            use_size_and_time_aware_chunking,
        )
    }

    /// Returns a transaction or output with proof response (bound by the max response size in bytes).
    /// This is the legacy implementation (that does not use size and time-aware chunking).
    fn get_transactions_or_outputs_with_proof_by_size_legacy(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        mut num_outputs_to_fetch: u64,
        include_events: bool,
        max_num_output_reductions: u64,
        max_response_size: u64,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        let mut num_output_reductions = 0;
        while num_output_reductions <= max_num_output_reductions {
            let output_list_with_proof = self.storage.get_transaction_outputs(
                start_version,
                num_outputs_to_fetch,
                proof_version,
            )?;
            let response = TransactionDataWithProofResponse {
                transaction_data_response_type: TransactionDataResponseType::TransactionOutputData,
                transaction_list_with_proof: None,
                transaction_output_list_with_proof: Some(output_list_with_proof),
            };

            let (overflow_frame, num_bytes) =
                check_overflow_network_frame(&response, max_response_size)?;

            if !overflow_frame {
                return Ok(response);
            } else if num_outputs_to_fetch == 1 {
                break; // We cannot return less than a single item. Fallback to transactions
            } else {
                metrics::increment_chunk_truncation_counter(
                    metrics::TRUNCATION_FOR_SIZE,
                    DataResponse::TransactionDataWithProof(response).get_label(),
                );
                let new_num_outputs_to_fetch = num_outputs_to_fetch / 2;
                debug!("The request for {:?} outputs was too large (num bytes: {:?}, limit: {:?}). Current number of data reductions: {:?}",
                    num_outputs_to_fetch, num_bytes, max_response_size, num_output_reductions);
                num_outputs_to_fetch = new_num_outputs_to_fetch; // Try again with half the amount of data
                num_output_reductions += 1;
            }
        }

        // Return transactions only
        self.get_transactions_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            include_events,
            max_response_size,
            self.config.enable_size_and_time_aware_chunking,
        )
    }

    /// Returns a state value chunk with proof response (bound by the max response size in bytes)
    fn get_state_value_chunk_with_proof_by_size(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
        max_response_size: u64,
        use_size_and_time_aware_chunking: bool,
    ) -> Result<StateValueChunkWithProof, Error> {
        // Calculate the number of state values to fetch
        let expected_num_state_values = inclusive_range_len(start_index, end_index)?;
        let max_num_state_values = self.config.max_state_chunk_size;
        let num_state_values_to_fetch = min(expected_num_state_values, max_num_state_values);

        // If size and time-aware chunking are disabled, use the legacy implementation
        if !use_size_and_time_aware_chunking {
            return self.get_state_value_chunk_with_proof_by_size_legacy(
                version,
                start_index,
                end_index,
                num_state_values_to_fetch,
                max_response_size,
            );
        }

        // Get the state value chunk iterator
        let mut state_value_iterator = self.storage.get_state_value_chunk_iter(
            version,
            start_index as usize,
            num_state_values_to_fetch as usize,
        )?;

        // Initialize the fetched state values
        let mut state_values = vec![];

        // Create a response progress tracker
        let mut response_progress_tracker = ResponseDataProgressTracker::new(
            num_state_values_to_fetch,
            max_response_size,
            self.config.max_storage_read_wait_time_ms,
            self.time_service.clone(),
        );

        // Fetch as many state values as possible
        while !response_progress_tracker.is_response_complete() {
            match state_value_iterator.next() {
                Some(Ok(state_value)) => {
                    // Calculate the number of serialized bytes for the state value
                    let num_serialized_bytes = get_num_serialized_bytes(&state_value)
                        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;

                    // Add the state value to the list
                    if response_progress_tracker
                        .data_items_fits_in_response(true, num_serialized_bytes)
                    {
                        state_values.push(state_value);
                        response_progress_tracker.add_data_item(num_serialized_bytes);
                    } else {
                        break; // Cannot add any more data items
                    }
                },
                Some(Err(error)) => {
                    return Err(Error::StorageErrorEncountered(error.to_string()));
                },
                None => {
                    // Log a warning that the iterator did not contain all the expected data
                    warn!(
                        "The state value iterator is missing data! Version: {:?}, \
                        start index: {:?}, end index: {:?}, num state values to fetch: {:?}",
                        version, start_index, end_index, num_state_values_to_fetch
                    );
                    break;
                },
            }
        }

        // Create the state value chunk with proof
        let state_value_chunk_with_proof = self.storage.get_state_value_chunk_proof(
            version,
            start_index as usize,
            state_values,
        )?;

        // Update the data truncation metrics
        response_progress_tracker
            .update_data_truncation_metrics(DataResponse::get_state_value_chunk_with_proof_label());

        Ok(state_value_chunk_with_proof)
    }

    /// Returns a state value chunk with proof response (bound by the max response size in bytes).
    /// This is the legacy implementation (that does not use size and time-aware chunking).
    fn get_state_value_chunk_with_proof_by_size_legacy(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
        mut num_state_values_to_fetch: u64,
        max_response_size: u64,
    ) -> Result<StateValueChunkWithProof, Error> {
        while num_state_values_to_fetch >= 1 {
            let state_value_chunk_with_proof = self.storage.get_state_value_chunk_with_proof(
                version,
                start_index as usize,
                num_state_values_to_fetch as usize,
            )?;
            if num_state_values_to_fetch == 1 {
                return Ok(state_value_chunk_with_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) =
                check_overflow_network_frame(&state_value_chunk_with_proof, max_response_size)?;
            if !overflow_frame {
                return Ok(state_value_chunk_with_proof);
            } else {
                metrics::increment_chunk_truncation_counter(
                    metrics::TRUNCATION_FOR_SIZE,
                    DataResponse::StateValueChunkWithProof(state_value_chunk_with_proof)
                        .get_label(),
                );
                let new_num_state_values_to_fetch = num_state_values_to_fetch / 2;
                debug!("The request for {:?} state values was too large (num bytes: {:?}, limit: {:?}). Retrying with {:?}.",
                    num_state_values_to_fetch, num_bytes, max_response_size, new_num_state_values_to_fetch);
                num_state_values_to_fetch = new_num_state_values_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_state_value_chunk_with_proof request! Version: {:?}, \
            start index: {:?}, end index: {:?}. The data cannot fit into a single network frame!",
            version, start_index, end_index
        )))
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
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error> {
        self.get_transactions_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            include_events,
            self.config.max_network_chunk_bytes,
            self.config.enable_size_and_time_aware_chunking,
        )
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> aptos_storage_service_types::Result<EpochChangeProof, Error> {
        self.get_epoch_ending_ledger_infos_by_size(
            start_epoch,
            expected_end_epoch,
            self.config.max_network_chunk_bytes,
            self.config.enable_size_and_time_aware_chunking,
        )
    }

    fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error> {
        self.get_transaction_outputs_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            self.config.max_network_chunk_bytes,
            false,
            self.config.enable_size_and_time_aware_chunking,
        )
    }

    fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_num_output_reductions: u64,
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error> {
        self.get_transactions_or_outputs_with_proof_by_size(
            proof_version,
            start_version,
            end_version,
            include_events,
            max_num_output_reductions,
            self.config.max_network_chunk_bytes,
            self.config.enable_size_and_time_aware_chunking,
        )
    }

    fn get_transaction_data_with_proof(
        &self,
        transaction_data_with_proof_request: &GetTransactionDataWithProofRequest,
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error> {
        // Extract the data versions from the request
        let proof_version = transaction_data_with_proof_request.proof_version;
        let start_version = transaction_data_with_proof_request.start_version;
        let end_version = transaction_data_with_proof_request.end_version;

        // Calculate the max response size to use
        let max_response_bytes = min(
            transaction_data_with_proof_request.max_response_bytes,
            self.config.max_network_chunk_bytes_v2,
        );

        // Fetch the transaction data based on the request type
        match transaction_data_with_proof_request.transaction_data_request_type {
            TransactionDataRequestType::TransactionData(request) => {
                // Get the transaction list with proof
                self.get_transactions_with_proof_by_size(
                    proof_version,
                    start_version,
                    end_version,
                    request.include_events,
                    max_response_bytes,
                    self.config.enable_size_and_time_aware_chunking,
                )
            },
            TransactionDataRequestType::TransactionOutputData => {
                // Get the transaction output list with proof
                self.get_transaction_outputs_with_proof_by_size(
                    proof_version,
                    start_version,
                    end_version,
                    max_response_bytes,
                    false,
                    self.config.enable_size_and_time_aware_chunking,
                )
            },
            TransactionDataRequestType::TransactionOrOutputData(request) => {
                // Get the transaction or output list with proof
                self.get_transactions_or_outputs_with_proof_by_size(
                    proof_version,
                    start_version,
                    end_version,
                    request.include_events,
                    0, // Fetch all outputs, or return transactions
                    max_response_bytes,
                    self.config.enable_size_and_time_aware_chunking,
                )
            },
        }
    }

    fn get_number_of_states(
        &self,
        version: u64,
    ) -> aptos_storage_service_types::Result<u64, Error> {
        let number_of_states = self.storage.get_state_item_count(version)?;
        Ok(number_of_states as u64)
    }

    fn get_state_value_chunk_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> aptos_storage_service_types::Result<StateValueChunkWithProof, Error> {
        self.get_state_value_chunk_with_proof_by_size(
            version,
            start_index,
            end_index,
            self.config.max_network_chunk_bytes,
            self.config.enable_size_and_time_aware_chunking,
        )
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
        ) -> StorageResult<TransactionListWithProofV2>;

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
        ) -> StorageResult<TransactionOutputListWithProofV2>;

        fn get_state_item_count(&self, version: Version) -> StorageResult<usize>;

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

        fn get_persisted_auxiliary_info_iterator(
            &self,
            start_version: Version,
            num_persisted_auxiliary_info: usize,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<PersistedAuxiliaryInfo>> + '_>>;
    );
}

/// A simple struct to track the progress of data fetching operations for each response
pub struct ResponseDataProgressTracker {
    num_items_to_fetch: u64,
    max_response_size: u64,
    max_storage_read_wait_time_ms: u64,
    time_service: TimeService,

    num_items_fetched: u64,
    serialized_data_size: u64,
    storage_read_start_time: Instant,
}

impl ResponseDataProgressTracker {
    pub fn new(
        num_items_to_fetch: u64,
        max_response_size: u64,
        max_storage_read_wait_time_ms: u64,
        time_service: TimeService,
    ) -> Self {
        let storage_read_start_time = time_service.now();
        Self {
            num_items_to_fetch,
            max_response_size,
            max_storage_read_wait_time_ms,
            time_service,
            num_items_fetched: 0,
            serialized_data_size: 0,
            storage_read_start_time,
        }
    }

    /// Adds a data item to the response, updating the number of items
    /// fetched and the cumulative serialized data size.
    pub fn add_data_item(&mut self, serialized_data_size: u64) {
        self.num_items_fetched += 1;
        self.serialized_data_size += serialized_data_size;
    }

    /// Returns true iff the given data item fits in the response
    /// (i.e., it does not overflow the maximum response size).
    ///
    /// Note: If `always_allow_first_item` is true, the first item is
    /// always allowed (even if it overflows the maximum response size).
    pub fn data_items_fits_in_response(
        &self,
        always_allow_first_item: bool,
        serialized_data_size: u64,
    ) -> bool {
        if always_allow_first_item && self.num_items_fetched == 0 {
            true // We always include at least one item
        } else {
            let new_serialized_data_size = self
                .serialized_data_size
                .saturating_add(serialized_data_size);
            new_serialized_data_size < self.max_response_size
        }
    }

    /// Checks if the response is complete based on the number of items fetched, the
    /// cumulative serialized data size, and the cumulative storage read duration.
    pub fn is_response_complete(&self) -> bool {
        // If we have fetched all the items, the response is complete
        if self.num_items_fetched >= self.num_items_to_fetch {
            return true;
        }

        // If the serialized data size exceeds the maximum, the response is complete
        if self.serialized_data_size >= self.max_response_size {
            return true;
        }

        // If the storage read duration exceeds the maximum, the response is complete
        if self.overflowed_storage_read_duration() {
            return true;
        }

        // Otherwise, the response is not yet complete
        false
    }

    /// Checks if the storage read duration has overflowed the maximum wait time
    fn overflowed_storage_read_duration(&self) -> bool {
        let time_now = self.time_service.now();
        let time_elapsed_ms = time_now
            .duration_since(self.storage_read_start_time)
            .as_millis() as u64;

        time_elapsed_ms >= self.max_storage_read_wait_time_ms
    }

    /// Updates the truncation logs and metrics if the data was truncated
    fn update_data_truncation_metrics(&self, data_response_label: &str) {
        // Only update the metrics if the data was truncated
        if self.num_items_fetched >= self.num_items_to_fetch {
            return;
        }

        // Update the metrics based on the truncation reason
        let truncation_reason = if self.overflowed_storage_read_duration() {
            debug!(
                "Truncated data response for {:?} by time, after fetching {:?} out of {:?} \
                items (time waited: {:?} ms, maximum wait time: {:?}).",
                data_response_label,
                self.num_items_fetched,
                self.num_items_to_fetch,
                self.storage_read_start_time.elapsed().as_millis(),
                self.max_storage_read_wait_time_ms,
            );

            metrics::TRUNCATION_FOR_TIME
        } else {
            debug!(
                "Truncated data response for {:?} by size, after fetching {:?} out of {:?} \
                items (response size: {:?} bytes, maximum size: {:?}).",
                data_response_label,
                self.num_items_fetched,
                self.num_items_to_fetch,
                self.serialized_data_size,
                self.max_response_size,
            );

            metrics::TRUNCATION_FOR_SIZE
        };
        metrics::increment_chunk_truncation_counter(truncation_reason, data_response_label);
    }
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

/// Serializes the given data and returns true iff the data will overflow
/// the maximum network frame size. Also returns the number of serialized
/// bytes for logging purposes.
fn check_overflow_network_frame<T: ?Sized + Serialize>(
    data: &T,
    max_network_frame_bytes: u64,
) -> aptos_storage_service_types::Result<(bool, u64), Error> {
    let num_serialized_bytes = bcs::to_bytes(&data)
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?
        .len() as u64;
    let overflow_frame = num_serialized_bytes >= max_network_frame_bytes;
    Ok((overflow_frame, num_serialized_bytes))
}

/// Serializes the given data and returns the number of serialized bytes
fn get_num_serialized_bytes<T: ?Sized + Serialize>(
    data: &T,
) -> aptos_storage_service_types::Result<u64, Error> {
    let num_serialized_bytes = bcs::serialized_size(data)
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
    Ok(num_serialized_bytes as u64)
}
