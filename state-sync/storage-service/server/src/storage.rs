// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, metrics::increment_network_frame_overflow};
use aptos_config::config::StorageServiceConfig;
use aptos_logger::debug;
use aptos_storage_interface::{AptosDbError, DbReader, Result as StorageResult};
use aptos_storage_service_types::{
    requests::{GetTransactionDataWithProofRequest, TransactionDataRequestType},
    responses::{
        CompleteDataRange, DataResponse, DataSummary, TransactionDataResponseType,
        TransactionDataWithProofResponse, TransactionOrOutputListWithProof,
    },
};
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        PersistedAuxiliaryInfo, TransactionListWithAuxiliaryInfos, TransactionListWithProof,
        TransactionListWithProofV2, TransactionOutputListWithAuxiliaryInfos,
        TransactionOutputListWithProof, TransactionOutputListWithProofV2, Version,
    },
};
use serde::Serialize;
use std::{cmp::min, sync::Arc};

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
}

impl StorageReader {
    pub fn new(config: StorageServiceConfig, storage: Arc<dyn DbReader>) -> Self {
        // Create a timed storage reader
        let storage = Arc::new(TimedStorageReader::new(storage));

        Self { config, storage }
    }

    /// Constructs the transaction list with proof v2 (which includes auxiliary
    /// information for each transaction in the given list with proof v1).
    fn construct_transaction_list_with_proof_v2(
        &self,
        transaction_list_with_proof: TransactionListWithProof,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        // Verify that the first transaction version exists
        let first_transaction_version = transaction_list_with_proof
            .get_first_transaction_version()
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered(
                    "First transaction version is missing in the response!".into(),
                )
            })?;

        // Get the persisted auxiliary infos for the transactions
        let num_infos_to_fetch = transaction_list_with_proof.get_num_transactions();
        let persisted_auxiliary_infos =
            self.fetch_persisted_auxiliary_infos(first_transaction_version, num_infos_to_fetch)?;

        // Create the transaction list with proof v2
        let transaction_list_with_auxiliary_info = TransactionListWithAuxiliaryInfos {
            transaction_list_with_proof,
            persisted_auxiliary_infos,
        };
        let transaction_list_with_proof =
            TransactionListWithProofV2::new(transaction_list_with_auxiliary_info);

        // Return the transaction data response
        Ok(TransactionDataWithProofResponse {
            transaction_data_response_type: TransactionDataResponseType::TransactionData,
            transaction_list_with_proof: Some(transaction_list_with_proof),
            transaction_output_list_with_proof: None,
        })
    }

    /// Constructs the transaction output list with proof v2 (which includes
    /// auxiliary information for each item in the given list with proof v1).
    fn construct_output_list_with_proof_v2(
        &self,
        transaction_output_list_with_proof: TransactionOutputListWithProof,
    ) -> Result<TransactionDataWithProofResponse, Error> {
        // Verify that the first transaction output version exists
        let first_transaction_output_version = transaction_output_list_with_proof
            .get_first_output_version()
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered(
                    "First transaction output version is missing in the response!".into(),
                )
            })?;

        // Get the persisted auxiliary infos
        let num_infos_to_fetch = transaction_output_list_with_proof.get_num_outputs();
        let persisted_auxiliary_infos = self.fetch_persisted_auxiliary_infos(
            first_transaction_output_version,
            num_infos_to_fetch,
        )?;

        // Create the transaction output list with proof v2
        let transaction_output_list_with_auxiliary_info = TransactionOutputListWithAuxiliaryInfos {
            transaction_output_list_with_proof,
            persisted_auxiliary_infos,
        };
        let transaction_output_list_with_proof_v2 =
            TransactionOutputListWithProofV2::new(transaction_output_list_with_auxiliary_info);

        // Return the transaction data response
        Ok(TransactionDataWithProofResponse {
            transaction_data_response_type: TransactionDataResponseType::TransactionOutputData,
            transaction_list_with_proof: None,
            transaction_output_list_with_proof: Some(transaction_output_list_with_proof_v2),
        })
    }

    /// Fetches the persisted auxiliary infos starting at the specified
    /// version. Note: it is possible for some auxiliary infos to be
    /// missing, in which case None is returned for each missing version.
    fn fetch_persisted_auxiliary_infos(
        &self,
        first_version: Version,
        num_infos_to_fetch: usize,
    ) -> aptos_storage_service_types::Result<Vec<PersistedAuxiliaryInfo>, Error> {
        // Get an iterator for the persisted auxiliary infos
        let persisted_auxiliary_info_iter = self
            .storage
            .get_persisted_auxiliary_info_iterator(first_version, num_infos_to_fetch)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;

        // Collect the persisted auxiliary infos into a vector
        let mut persisted_auxiliary_infos = vec![];
        for result in persisted_auxiliary_info_iter {
            match result {
                Ok(auxiliary_info) => persisted_auxiliary_infos.push(auxiliary_info),
                Err(error) => return Err(Error::StorageErrorEncountered(error.to_string())),
            }
        }
        Ok(persisted_auxiliary_infos)
    }

    /// Returns the state values range held in the database (lowest to highest).
    /// Note: it is currently assumed that if a node contains a transaction at a
    /// version, V, the node also contains all state values at V.
    fn fetch_state_values_range(
        &self,
        latest_version: Version,
        transactions_range: &Option<CompleteDataRange<Version>>,
    ) -> aptos_storage_service_types::Result<Option<CompleteDataRange<Version>>, Error> {
        let pruner_enabled = self
            .storage
            .is_state_merkle_pruner_enabled()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        if !pruner_enabled {
            return Ok(*transactions_range);
        }
        let pruning_window = self
            .storage
            .get_epoch_snapshot_prune_window()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;

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
        let first_transaction_version = self
            .storage
            .get_first_txn_version()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
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
        let first_output_version = self
            .storage
            .get_first_write_set_version()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        if let Some(first_output_version) = first_output_version {
            let output_range = CompleteDataRange::new(first_output_version, latest_version)
                .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
            Ok(Some(output_range))
        } else {
            Ok(None)
        }
    }
}

impl StorageReaderInterface for StorageReader {
    fn get_data_summary(&self) -> aptos_storage_service_types::Result<DataSummary, Error> {
        // Fetch the latest ledger info
        let latest_ledger_info_with_sigs = self
            .storage
            .get_latest_ledger_info()
            .map_err(|err| Error::StorageErrorEncountered(err.to_string()))?;

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
        let mut num_transactions_to_fetch = min(expected_num_transactions, max_num_transactions);

        // Attempt to serve the request
        while num_transactions_to_fetch >= 1 {
            let transaction_list_with_proof = self
                .storage
                .get_transactions(
                    start_version,
                    num_transactions_to_fetch,
                    proof_version,
                    include_events,
                )
                .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
            if num_transactions_to_fetch == 1 {
                return Ok(transaction_list_with_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) = check_overflow_network_frame(
                &transaction_list_with_proof,
                self.config.max_network_chunk_bytes,
            )?;
            if !overflow_frame {
                return Ok(transaction_list_with_proof);
            } else {
                increment_network_frame_overflow(
                    DataResponse::TransactionsWithProof(transaction_list_with_proof).get_label(),
                );
                let new_num_transactions_to_fetch = num_transactions_to_fetch / 2;
                debug!("The request for {:?} transactions was too large (num bytes: {:?}). Retrying with {:?}.",
                    num_transactions_to_fetch, num_bytes, new_num_transactions_to_fetch);
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

    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> aptos_storage_service_types::Result<EpochChangeProof, Error> {
        // Calculate the number of ledger infos to fetch
        let expected_num_ledger_infos = inclusive_range_len(start_epoch, expected_end_epoch)?;
        let max_num_ledger_infos = self.config.max_epoch_chunk_size;
        let mut num_ledger_infos_to_fetch = min(expected_num_ledger_infos, max_num_ledger_infos);

        // Attempt to serve the request
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
                .get_epoch_ending_ledger_infos(start_epoch, end_epoch)
                .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
            if num_ledger_infos_to_fetch == 1 {
                return Ok(epoch_change_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) = check_overflow_network_frame(
                &epoch_change_proof,
                self.config.max_network_chunk_bytes,
            )?;
            if !overflow_frame {
                return Ok(epoch_change_proof);
            } else {
                increment_network_frame_overflow(
                    DataResponse::EpochEndingLedgerInfos(epoch_change_proof).get_label(),
                );
                let new_num_ledger_infos_to_fetch = num_ledger_infos_to_fetch / 2;
                debug!("The request for {:?} ledger infos was too large (num bytes: {:?}). Retrying with {:?}.",
                    num_ledger_infos_to_fetch, num_bytes, new_num_ledger_infos_to_fetch);
                num_ledger_infos_to_fetch = new_num_ledger_infos_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_epoch_ending_ledger_infos request! Start epoch: {:?}, \
            expected end epoch: {:?}. The data cannot fit into a single network frame!",
            start_epoch, expected_end_epoch
        )))
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
        let mut num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // Attempt to serve the request
        while num_outputs_to_fetch >= 1 {
            let output_list_with_proof = self
                .storage
                .get_transaction_outputs(start_version, num_outputs_to_fetch, proof_version)
                .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
            if num_outputs_to_fetch == 1 {
                return Ok(output_list_with_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) = check_overflow_network_frame(
                &output_list_with_proof,
                self.config.max_network_chunk_bytes,
            )?;
            if !overflow_frame {
                return Ok(output_list_with_proof);
            } else {
                increment_network_frame_overflow(
                    DataResponse::TransactionOutputsWithProof(output_list_with_proof).get_label(),
                );
                let new_num_outputs_to_fetch = num_outputs_to_fetch / 2;
                debug!("The request for {:?} outputs was too large (num bytes: {:?}). Retrying with {:?}.",
                    num_outputs_to_fetch, num_bytes, new_num_outputs_to_fetch);
                num_outputs_to_fetch = new_num_outputs_to_fetch; // Try again with half the amount of data
            }
        }

        Err(Error::UnexpectedErrorEncountered(format!(
            "Unable to serve the get_transaction_outputs_with_proof request! Proof version: {:?}, \
            start version: {:?}, end version: {:?}. The data cannot fit into a single network frame!",
            proof_version, start_version, end_version
        )))
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
        let mut num_outputs_to_fetch = min(expected_num_outputs, max_num_outputs);

        // Attempt to serve the outputs. Halve the data only as many
        // times as the fallback count allows. If the data still
        // doesn't fit, return a transaction chunk instead.
        let mut num_output_reductions = 0;
        while num_output_reductions <= max_num_output_reductions {
            let output_list_with_proof = self
                .storage
                .get_transaction_outputs(start_version, num_outputs_to_fetch, proof_version)
                .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
            let (overflow_frame, num_bytes) = check_overflow_network_frame(
                &output_list_with_proof,
                self.config.max_network_chunk_bytes,
            )?;

            if !overflow_frame {
                return Ok((None, Some(output_list_with_proof)));
            } else if num_outputs_to_fetch == 1 {
                break; // We cannot return less than a single item. Fallback to transactions
            } else {
                increment_network_frame_overflow(
                    DataResponse::TransactionsOrOutputsWithProof((
                        None,
                        Some(output_list_with_proof),
                    ))
                    .get_label(),
                );
                let new_num_outputs_to_fetch = num_outputs_to_fetch / 2;
                debug!("The request for {:?} outputs was too large (num bytes: {:?}). Current number of data reductions: {:?}",
                    num_outputs_to_fetch, num_bytes, num_output_reductions);
                num_outputs_to_fetch = new_num_outputs_to_fetch; // Try again with half the amount of data
                num_output_reductions += 1;
            }
        }

        // Return transactions only
        let transactions_with_proof = self.get_transactions_with_proof(
            proof_version,
            start_version,
            end_version,
            include_events,
        )?;
        Ok((Some(transactions_with_proof), None))
    }

    // TODOs:
    // 1. Make this function respect the `max_network_chunk_bytes` limit. It's
    //    currently not respected because the auxiliary information is not
    //    size checked. However, this is okay for now because the limit is not
    //    a hard limit, and auxiliary information is still quite small (e.g., u32
    //    per transaction). However, we should address this.
    // 2. Make this function respect the `max_response_bytes` limit in each request.
    //    It's currently not respected because we only consider the
    //    `max_network_chunk_bytes` limit as defined by the storage service config.
    //    We should also address this.
    fn get_transaction_data_with_proof(
        &self,
        transaction_data_with_proof_request: &GetTransactionDataWithProofRequest,
    ) -> aptos_storage_service_types::Result<TransactionDataWithProofResponse, Error> {
        // Extract the data versions from the request
        let proof_version = transaction_data_with_proof_request.proof_version;
        let start_version = transaction_data_with_proof_request.start_version;
        let end_version = transaction_data_with_proof_request.end_version;

        // Fetch the transaction data based on the request type
        match transaction_data_with_proof_request.transaction_data_request_type {
            TransactionDataRequestType::TransactionData(request) => {
                // Get the transaction list with proof
                let transaction_list_with_proof = self.get_transactions_with_proof(
                    proof_version,
                    start_version,
                    end_version,
                    request.include_events,
                )?;

                // Fetch the persisted auxiliary infos and combine the data
                self.construct_transaction_list_with_proof_v2(transaction_list_with_proof)
            },
            TransactionDataRequestType::TransactionOutputData => {
                // Get the transaction output list with proof
                let output_list_with_proof = self.get_transaction_outputs_with_proof(
                    proof_version,
                    start_version,
                    end_version,
                )?;

                // Fetch the persisted auxiliary infos and combine the data
                self.construct_output_list_with_proof_v2(output_list_with_proof)
            },
            TransactionDataRequestType::TransactionOrOutputData(request) => {
                // Get the transaction or output list with proof
                let (transaction_list_with_proof, output_list_with_proof) = self
                    .get_transactions_or_outputs_with_proof(
                        proof_version,
                        start_version,
                        end_version,
                        request.include_events,
                        0, // Fetch all outputs, or return transactions
                    )?;

                // Fetch the persisted auxiliary infos and combine the data
                match (transaction_list_with_proof, output_list_with_proof) {
                    (Some(transaction_list_with_proof), None) => {
                        self.construct_transaction_list_with_proof_v2(transaction_list_with_proof)
                    },
                    (None, Some(output_list_with_proof)) => {
                        self.construct_output_list_with_proof_v2(output_list_with_proof)
                    },
                    _ => Err(Error::UnexpectedErrorEncountered(
                        "Unexpected transactions and outputs returned! None or both found!".into(),
                    )),
                }
            },
        }
    }

    fn get_number_of_states(
        &self,
        version: u64,
    ) -> aptos_storage_service_types::Result<u64, Error> {
        let number_of_states = self
            .storage
            .get_state_item_count(version)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
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
        let mut num_state_values_to_fetch = min(expected_num_state_values, max_num_state_values);

        // Attempt to serve the request
        while num_state_values_to_fetch >= 1 {
            let state_value_chunk_with_proof = self
                .storage
                .get_state_value_chunk_with_proof(
                    version,
                    start_index as usize,
                    num_state_values_to_fetch as usize,
                )
                .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
            if num_state_values_to_fetch == 1 {
                return Ok(state_value_chunk_with_proof); // We cannot return less than a single item
            }

            // Attempt to divide up the request if it overflows the message size
            let (overflow_frame, num_bytes) = check_overflow_network_frame(
                &state_value_chunk_with_proof,
                self.config.max_network_chunk_bytes,
            )?;
            if !overflow_frame {
                return Ok(state_value_chunk_with_proof);
            } else {
                increment_network_frame_overflow(
                    DataResponse::StateValueChunkWithProof(state_value_chunk_with_proof)
                        .get_label(),
                );
                let new_num_state_values_to_fetch = num_state_values_to_fetch / 2;
                debug!("The request for {:?} state values was too large (num bytes: {:?}). Retrying with {:?}.",
                    num_state_values_to_fetch, num_bytes, new_num_state_values_to_fetch);
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

        fn get_state_item_count(&self, version: Version) -> StorageResult<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> StorageResult<StateValueChunkWithProof>;

        fn get_persisted_auxiliary_info_iterator(
            &self,
            start_version: Version,
            num_persisted_auxiliary_info: usize,
        ) -> StorageResult<Box<dyn Iterator<Item = StorageResult<PersistedAuxiliaryInfo>> + '_>>;
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

/// Serializes the given data and returns true iff the data will overflow
/// the maximum network frame size. Also returns the number of serialized
/// bytes for logging purposes.
pub(crate) fn check_overflow_network_frame<T: ?Sized + Serialize>(
    data: &T,
    max_network_frame_bytes: u64,
) -> aptos_storage_service_types::Result<(bool, u64), Error> {
    let num_serialized_bytes = bcs::to_bytes(&data)
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?
        .len() as u64;
    let overflow_frame = num_serialized_bytes >= max_network_frame_bytes;
    Ok((overflow_frame, num_serialized_bytes))
}
