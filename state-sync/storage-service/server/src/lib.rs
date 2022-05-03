// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{increment_counter, start_timer, LRU_CACHE_HIT, LRU_CACHE_PROBE},
    network::StorageServiceNetworkEvents,
};
use ::network::ProtocolId;
use aptos_config::config::StorageServiceConfig;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    epoch_change::EpochChangeProof,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use bounded_executor::BoundedExecutor;
use futures::stream::StreamExt;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use storage_interface::DbReader;
use storage_service_types::{
    AccountStatesChunkWithProofRequest, CompleteDataRange, DataSummary,
    EpochEndingLedgerInfoRequest, ProtocolMetadata, Result, ServerProtocolVersion,
    StorageServerSummary, StorageServiceError, StorageServiceRequest, StorageServiceResponse,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};
use thiserror::Error;
use tokio::runtime::Handle;

mod logging;
mod metrics;
pub mod network;

#[cfg(test)]
mod tests;

/// Storage server constants.
pub const STORAGE_SERVER_VERSION: u64 = 1;
const SUMMARY_LOG_FREQUENCY_SECS: u64 = 5;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Serialize)]
pub enum Error {
    #[error("Invalid request received: {0}")]
    InvalidRequest(String),
    #[error("Storage error encountered: {0}")]
    StorageErrorEncountered(String),
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

impl Error {
    /// Returns a summary label for the error type
    fn get_label(&self) -> &'static str {
        match self {
            Error::InvalidRequest(_) => "invalid_request",
            Error::StorageErrorEncountered(_) => "storage_error",
            Error::UnexpectedErrorEncountered(_) => "unexpected_error",
        }
    }
}

/// The server-side actor for the storage service. Handles inbound storage
/// service requests from clients.
pub struct StorageServiceServer<T> {
    bounded_executor: BoundedExecutor,
    config: StorageServiceConfig,
    network_requests: StorageServiceNetworkEvents,
    storage: T,
    time_service: TimeService,

    // We maintain a cached storage server summary to avoid hitting the DB for
    // every request. This is refreshed periodically.
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,

    // We maintain a LRU cache for commonly requested data items. This is
    // separate from the cached storage summary because these responses
    // should never change (the storage summary changes over time).
    lru_storage_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
}

impl<T: StorageReaderInterface> StorageServiceServer<T> {
    pub fn new(
        config: StorageServiceConfig,
        executor: Handle,
        storage: T,
        time_service: TimeService,
        network_requests: StorageServiceNetworkEvents,
    ) -> Self {
        let bounded_executor =
            BoundedExecutor::new(config.max_concurrent_requests as usize, executor);
        let cached_storage_server_summary = Arc::new(RwLock::new(StorageServerSummary::default()));
        let lru_storage_cache = Arc::new(Mutex::new(LruCache::new(
            config.max_lru_cache_size as usize,
        )));

        Self {
            config,
            bounded_executor,
            storage,
            network_requests,
            time_service,
            cached_storage_server_summary,
            lru_storage_cache,
        }
    }

    /// Spawns a non-terminating task that refreshes the cached storage server summary
    async fn spawn_storage_summary_refresher(&mut self) {
        let config = self.config;
        let storage = self.storage.clone();
        let time_service = self.time_service.clone();
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();

        // Spawn the task
        self.bounded_executor
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Periodically refresh the cache
                loop {
                    ticker.next().await;

                    if let Err(error) = refresh_cached_storage_summary(
                        config,
                        storage.clone(),
                        cached_storage_server_summary.clone(),
                    ) {
                        let error = format!(
                            "Failed to refresh the cached storage summary! Error: {:?}",
                            error
                        );
                        error!(LogSchema::new(LogEntry::StorageServiceError).message(&error));
                    }
                }
            })
            .await;
    }

    /// Starts the storage service server thread
    pub async fn start(mut self) {
        // Spawn the refresher for the cache
        self.spawn_storage_summary_refresher().await;

        // Handle the storage requests
        while let Some(request) = self.network_requests.next().await {
            // Log the request
            let (peer, protocol, request, response_sender) = request;
            debug!(LogSchema::new(LogEntry::ReceivedStorageRequest)
                .request(&request)
                .message(&format!(
                    "Received storage request. Peer: {:?}, protocol: {:?}.",
                    peer, protocol,
                )));

            // All handler methods are currently CPU-bound and synchronous
            // I/O-bound, so we want to spawn on the blocking thread pool to
            // avoid starving other async tasks on the same runtime.
            let storage = self.storage.clone();
            let cached_storage_server_summary = self.cached_storage_server_summary.clone();
            let lru_storage_cache = self.lru_storage_cache.clone();
            self.bounded_executor
                .spawn_blocking(move || {
                    let response =
                        Handler::new(storage, cached_storage_server_summary, lru_storage_cache)
                            .call(protocol, request);
                    log_storage_response(&response);
                    response_sender.send(response);
                })
                .await;
        }
    }
}

/// Refreshes the cached storage server summary
fn refresh_cached_storage_summary<T: StorageReaderInterface>(
    storage_config: StorageServiceConfig,
    storage: T,
    cached_storage_summary: Arc<RwLock<StorageServerSummary>>,
) -> Result<()> {
    // Fetch the data summary from storage
    let data_summary = storage
        .get_data_summary()
        .map_err(|error| StorageServiceError::InternalError(error.to_string()))?;

    // Initialize the protocol metadata
    let protocol_metadata = ProtocolMetadata {
        max_epoch_chunk_size: storage_config.max_epoch_chunk_size,
        max_transaction_chunk_size: storage_config.max_transaction_chunk_size,
        max_transaction_output_chunk_size: storage_config.max_transaction_output_chunk_size,
        max_account_states_chunk_size: storage_config.max_account_states_chunk_sizes,
    };

    // Save the storage server summary
    let storage_server_summary = StorageServerSummary {
        protocol_metadata,
        data_summary,
    };
    *cached_storage_summary.write() = storage_server_summary;

    Ok(())
}

/// The `Handler` is the "pure" inbound request handler. It contains all the
/// necessary context and state needed to construct a response to an inbound
/// request. We usually clone/create a new handler for every request.
#[derive(Clone)]
pub struct Handler<T> {
    storage: T,
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    lru_storage_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
}

impl<T: StorageReaderInterface> Handler<T> {
    pub fn new(
        storage: T,
        cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
        lru_storage_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    ) -> Self {
        Self {
            storage,
            cached_storage_server_summary,
            lru_storage_cache,
        }
    }

    pub fn call(
        &self,
        protocol: ProtocolId,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse> {
        // Update the request count
        increment_counter(
            &metrics::STORAGE_REQUESTS_RECEIVED,
            protocol,
            request.get_label().into(),
        );

        // Time the request processing (the timer will stop when it's dropped)
        let _timer = start_timer(
            &metrics::STORAGE_REQUEST_PROCESSING_LATENCY,
            protocol,
            request.get_label().into(),
        );

        // Process the request
        let response = match &request {
            StorageServiceRequest::GetServerProtocolVersion => self.get_server_protocol_version(),
            StorageServiceRequest::GetStorageServerSummary => self.get_storage_server_summary(),
            _ => self.process_cachable_request(protocol, &request),
        };

        // Process the response and handle any errors
        match response {
            Err(error) => {
                // Log the error and update the counters
                increment_counter(
                    &metrics::STORAGE_ERRORS_ENCOUNTERED,
                    protocol,
                    error.get_label().into(),
                );
                error!(LogSchema::new(LogEntry::StorageServiceError)
                    .error(&error)
                    .request(&request));

                // Return an appropriate response to the client
                match error {
                    Error::InvalidRequest(error) => Err(StorageServiceError::InvalidRequest(error)),
                    error => Err(StorageServiceError::InternalError(error.to_string())),
                }
            }
            Ok(response) => {
                // The request was successful
                increment_counter(
                    &metrics::STORAGE_RESPONSES_SENT,
                    protocol,
                    response.get_label().into(),
                );
                Ok(response)
            }
        }
    }

    fn process_cachable_request(
        &self,
        protocol: ProtocolId,
        request: &StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        increment_counter(&metrics::LRU_CACHE_EVENT, protocol, LRU_CACHE_PROBE.into());

        // Check if the response is already in the cache
        if let Some(response) = self.lru_storage_cache.lock().get(request) {
            increment_counter(&metrics::LRU_CACHE_EVENT, protocol, LRU_CACHE_HIT.into());
            return Ok(response.clone());
        }

        // Fetch the response from storage
        let response = match request {
            StorageServiceRequest::GetAccountStatesChunkWithProof(request) => {
                self.get_account_states_chunk_with_proof(request)
            }
            StorageServiceRequest::GetEpochEndingLedgerInfos(request) => {
                self.get_epoch_ending_ledger_infos(request)
            }
            StorageServiceRequest::GetNumberOfAccountsAtVersion(version) => {
                self.get_number_of_accounts_at_version(*version)
            }
            StorageServiceRequest::GetTransactionOutputsWithProof(request) => {
                self.get_transaction_outputs_with_proof(request)
            }
            StorageServiceRequest::GetTransactionsWithProof(request) => {
                self.get_transactions_with_proof(request)
            }
            _ => unreachable!("Received an unexpected request: {:?}", request),
        }?;

        // Cache the response before returning
        let _ = self
            .lru_storage_cache
            .lock()
            .put(request.clone(), response.clone());

        Ok(response)
    }

    fn get_account_states_chunk_with_proof(
        &self,
        request: &AccountStatesChunkWithProofRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let account_states_chunk_with_proof = self.storage.get_account_states_chunk_with_proof(
            request.version,
            request.start_account_index,
            request.end_account_index,
        )?;

        Ok(StorageServiceResponse::AccountStatesChunkWithProof(
            account_states_chunk_with_proof,
        ))
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        request: &EpochEndingLedgerInfoRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let epoch_change_proof = self
            .storage
            .get_epoch_ending_ledger_infos(request.start_epoch, request.expected_end_epoch)?;

        Ok(StorageServiceResponse::EpochEndingLedgerInfos(
            epoch_change_proof,
        ))
    }

    fn get_number_of_accounts_at_version(
        &self,
        version: Version,
    ) -> Result<StorageServiceResponse, Error> {
        let number_of_accounts = self.storage.get_number_of_accounts(version)?;

        Ok(StorageServiceResponse::NumberOfAccountsAtVersion(
            number_of_accounts,
        ))
    }

    fn get_server_protocol_version(&self) -> Result<StorageServiceResponse, Error> {
        let server_protocol_version = ServerProtocolVersion {
            protocol_version: STORAGE_SERVER_VERSION,
        };
        Ok(StorageServiceResponse::ServerProtocolVersion(
            server_protocol_version,
        ))
    }

    fn get_storage_server_summary(&self) -> Result<StorageServiceResponse, Error> {
        let storage_server_summary = self.cached_storage_server_summary.read().clone();
        Ok(StorageServiceResponse::StorageServerSummary(
            storage_server_summary,
        ))
    }

    fn get_transaction_outputs_with_proof(
        &self,
        request: &TransactionOutputsWithProofRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let transaction_output_list_with_proof = self.storage.get_transaction_outputs_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
        )?;

        Ok(StorageServiceResponse::TransactionOutputsWithProof(
            transaction_output_list_with_proof,
        ))
    }

    fn get_transactions_with_proof(
        &self,
        request: &TransactionsWithProofRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let transactions_with_proof = self.storage.get_transactions_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
            request.include_events,
        )?;

        Ok(StorageServiceResponse::TransactionsWithProof(
            transactions_with_proof,
        ))
    }
}

/// The interface into local storage (e.g., the Aptos DB) used by the storage
/// server to handle client requests.
pub trait StorageReaderInterface: Clone + Send + 'static {
    /// Returns a data summary of the underlying storage state.
    fn get_data_summary(&self) -> Result<DataSummary, Error>;

    /// Returns a list of transactions with a proof relative to the
    /// `proof_version`. The transaction list is expected to start at
    /// `start_version` and end at `end_version` (inclusive).
    /// If `include_events` is true, events are also returned.
    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
    ) -> Result<TransactionListWithProof, Error>;

    /// Returns a list of epoch ending ledger infos, starting at `start_epoch`
    /// and ending at the `expected_end_epoch` (inclusive). For example, if
    /// `start_epoch` is 0 and `end_epoch` is 1, this will return 2 epoch ending
    /// ledger infos (ending epoch 0 and 1, respectively).
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> Result<EpochChangeProof, Error>;

    /// Returns a list of transaction outputs with a proof relative to the
    /// `proof_version`. The transaction output list is expected to start at
    /// `start_version` and end at `end_version` (inclusive).
    fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> Result<TransactionOutputListWithProof, Error>;

    /// Returns the number of accounts in the account state tree at the
    /// specified version.
    fn get_number_of_accounts(&self, version: u64) -> Result<u64, Error>;

    /// Returns a chunk holding a list of account states starting at the
    /// specified `start_account_index` and ending at
    /// `end_account_index` (inclusive).
    fn get_account_states_chunk_with_proof(
        &self,
        version: u64,
        start_account_index: u64,
        end_account_index: u64,
    ) -> Result<StateValueChunkWithProof, Error>;
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
        Self { config, storage }
    }

    /// Returns the account states range held in the database (lowest to highest).
    /// Note: it is currently assumed that if a node contains a transaction at a
    /// version, V, the node also contains all account states at V.
    fn fetch_account_states_range(
        &self,
        latest_version: Version,
        transactions_range: &Option<CompleteDataRange<Version>>,
    ) -> Result<Option<CompleteDataRange<Version>>, Error> {
        let pruning_window = self
            .storage
            .get_state_prune_window()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?
            .map(|window| window as u64);
        if let Some(pruning_window) = pruning_window {
            if latest_version > pruning_window {
                // lowest_account_version = latest_version - pruning_window + 1;
                let mut lowest_account_version =
                    latest_version.checked_sub(pruning_window).ok_or_else(|| {
                        Error::UnexpectedErrorEncountered(
                            "Lowest account states version has overflown!".into(),
                        )
                    })?;
                lowest_account_version =
                    lowest_account_version.checked_add(1).ok_or_else(|| {
                        Error::UnexpectedErrorEncountered(
                            "Lowest account states version has overflown!".into(),
                        )
                    })?;

                // Create the account range
                let account_range = CompleteDataRange::new(lowest_account_version, latest_version)
                    .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                return Ok(Some(account_range));
            }
        }

        // No pruning has occurred. Return the transactions range.
        Ok(*transactions_range)
    }

    /// Returns the transaction range held in the database (lowest to highest).
    fn fetch_transaction_range(
        &self,
        latest_version: Version,
    ) -> Result<Option<CompleteDataRange<Version>>, Error> {
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
    ) -> Result<Option<CompleteDataRange<Version>>, Error> {
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
    fn get_data_summary(&self) -> Result<DataSummary, Error> {
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

        // Fetch the account states range
        let account_states = self.fetch_account_states_range(latest_version, &transactions)?;

        // Return the relevant data summary
        let data_summary = DataSummary {
            synced_ledger_info: Some(latest_ledger_info_with_sigs),
            epoch_ending_ledger_infos,
            transactions,
            transaction_outputs,
            account_states,
        };

        Ok(data_summary)
    }

    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
    ) -> Result<TransactionListWithProof, Error> {
        let expected_num_transactions = inclusive_range_len(start_version, end_version)?;
        let max_transaction_chunk_size = self.config.max_transaction_chunk_size;
        if expected_num_transactions > max_transaction_chunk_size {
            return Err(Error::InvalidRequest(format!(
                "Requested number of transactions is larger than the maximum! \
             Requested: {:?}, maximum: {:?}.",
                expected_num_transactions, max_transaction_chunk_size
            )));
        }

        let transaction_list_with_proof = self
            .storage
            .get_transactions(
                start_version,
                expected_num_transactions,
                proof_version,
                include_events,
            )
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(transaction_list_with_proof)
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> Result<EpochChangeProof, Error> {
        let expected_num_epochs = inclusive_range_len(start_epoch, expected_end_epoch)?;
        let max_epoch_chunk_size = self.config.max_epoch_chunk_size;
        if expected_num_epochs > max_epoch_chunk_size {
            return Err(Error::InvalidRequest(format!(
                "Requested number of ledger infos is larger than the maximum! \
             Requested: {:?}, maximum: {:?}.",
                expected_num_epochs, max_epoch_chunk_size
            )));
        }

        // The DbReader interface returns the epochs up to: `expected_end_epoch - 1`.
        // However, we wish to fetch epoch endings up to expected_end_epoch (inclusive).
        let expected_end_epoch = expected_end_epoch.checked_add(1).ok_or_else(|| {
            Error::UnexpectedErrorEncountered("Expected end epoch has overflown!".into())
        })?;
        let epoch_change_proof = self
            .storage
            .get_epoch_ending_ledger_infos(start_epoch, expected_end_epoch)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(epoch_change_proof)
    }

    fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> Result<TransactionOutputListWithProof, Error> {
        let expected_num_outputs = inclusive_range_len(start_version, end_version)?;
        let max_output_chunk_size = self.config.max_transaction_output_chunk_size;
        if expected_num_outputs > max_output_chunk_size {
            return Err(Error::InvalidRequest(format!(
                "Requested number of outputs is larger than the maximum! \
             Requested: {:?}, maximum: {:?}.",
                expected_num_outputs, max_output_chunk_size
            )));
        }

        let output_list_with_proof = self
            .storage
            .get_transaction_outputs(start_version, expected_num_outputs, proof_version)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(output_list_with_proof)
    }

    fn get_number_of_accounts(&self, version: u64) -> Result<u64, Error> {
        let number_of_accounts = self
            .storage
            .get_state_leaf_count(version)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(number_of_accounts as u64)
    }

    fn get_account_states_chunk_with_proof(
        &self,
        version: u64,
        start_account_index: u64,
        end_account_index: u64,
    ) -> Result<StateValueChunkWithProof, Error> {
        let expected_num_accounts = inclusive_range_len(start_account_index, end_account_index)?;
        let max_account_chunk_size = self.config.max_account_states_chunk_sizes;
        if expected_num_accounts > max_account_chunk_size {
            return Err(Error::InvalidRequest(format!(
                "Requested number of accounts is larger than the maximum! \
             Requested: {:?}, maximum: {:?}.",
                expected_num_accounts, max_account_chunk_size
            )));
        }

        let account_states_chunk_with_proof = self
            .storage
            .get_state_value_chunk_with_proof(
                version,
                start_account_index as usize,
                expected_num_accounts as usize,
            )
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(account_states_chunk_with_proof)
    }
}

/// Calculate `(start..=end).len()`. Returns an error if `end < start` or
/// `end == u64::MAX`.
fn inclusive_range_len(start: u64, end: u64) -> Result<u64, Error> {
    // len = end - start + 1
    let len = end.checked_sub(start).ok_or_else(|| {
        Error::InvalidRequest(format!("end ({}) must be >= start ({})", end, start))
    })?;
    let len = len
        .checked_add(1)
        .ok_or_else(|| Error::InvalidRequest(format!("end ({}) must not be u64::MAX", end)))?;
    Ok(len)
}

/// Logs the response sent by storage for a peer request
fn log_storage_response(storage_response: &Result<StorageServiceResponse, StorageServiceError>) {
    match storage_response {
        Ok(storage_response) => {
            if matches!(
                storage_response,
                StorageServiceResponse::StorageServerSummary(_)
            ) {
                // We expect peers to be polling our storage server summary frequently,
                // so only log this response periodically.
                sample!(
                    SampleRate::Duration(Duration::from_secs(SUMMARY_LOG_FREQUENCY_SECS)),
                    {
                        let response = format!("{}", storage_response);
                        debug!(LogSchema::new(LogEntry::SentStorageResponse).response(&response));
                    }
                );
            } else {
                let response = format!("{}", storage_response);
                debug!(LogSchema::new(LogEntry::SentStorageResponse).response(&response));
            }
        }
        Err(storage_error) => {
            let storage_error = format!("{:?}", storage_error);
            debug!(LogSchema::new(LogEntry::SentStorageResponse).response(&storage_error));
        }
    };
}
