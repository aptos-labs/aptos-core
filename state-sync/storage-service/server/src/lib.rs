// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use diem_infallible::RwLock;
use diem_types::{epoch_change::EpochChangeProof, transaction::TransactionListWithProof};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage_interface::DbReaderWriter;
use storage_service_types::{
    DataSummary, EpochEndingLedgerInfoRequest, ProtocolMetadata, ServerProtocolVersion,
    StorageServerSummary, StorageServiceError, StorageServiceRequest, StorageServiceResponse,
    TransactionsWithProofRequest,
};
use thiserror::Error;

#[cfg(test)]
mod tests;

// TODO(joshlind): make these configurable.
/// Storage server constants.
pub const MAX_TRANSACTION_CHUNK_SIZE: u64 = 1000;
pub const STORAGE_SERVER_VERSION: u64 = 1;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Serialize)]
pub enum Error {
    #[error("Storage error encountered: {0}")]
    StorageErrorEncountered(String),
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

/// The server-side implementation of the storage service. This provides all the
/// functionality required to handle storage service requests (i.e., from clients).
pub struct StorageServiceServer<T> {
    storage: T,
}

impl<T: StorageReaderInterface> StorageServiceServer<T> {
    pub fn new(storage: T) -> Self {
        Self { storage }
    }

    pub fn handle_request(
        &self,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let response = match request {
            StorageServiceRequest::GetEpochEndingLedgerInfos(request) => {
                self.get_epoch_ending_ledger_infos(request)
            }
            StorageServiceRequest::GetServerProtocolVersion => self.get_server_protocol_version(),
            StorageServiceRequest::GetStorageServerSummary => self.get_storage_server_summary(),
            StorageServiceRequest::GetTransactionsWithProof(request) => {
                self.get_transactions_with_proof(request)
            }
        };

        // If any requests resulted in an unexpected error, return an InternalStorageError to the
        // client and log the actual error.
        if let Err(_error) = response {
            // TODO(joshlind): add logging support to this library so we can log _error
            Ok(StorageServiceResponse::StorageServiceError(
                StorageServiceError::InternalError,
            ))
        } else {
            response
        }
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        request: EpochEndingLedgerInfoRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let epoch_change_proof = self
            .storage
            .get_epoch_ending_ledger_infos(request.start_epoch, request.expected_end_epoch)?;

        Ok(StorageServiceResponse::EpochEndingLedgerInfos(
            epoch_change_proof,
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
        let storage_server_summary = StorageServerSummary {
            protocol_metadata: ProtocolMetadata {
                max_transaction_chunk_size: MAX_TRANSACTION_CHUNK_SIZE,
            },
            data_summary: self.storage.get_data_summary()?,
        };

        Ok(StorageServiceResponse::StorageServerSummary(
            storage_server_summary,
        ))
    }

    fn get_transactions_with_proof(
        &self,
        request: TransactionsWithProofRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let transactions_with_proof = self.storage.get_transactions_with_proof(
            request.proof_version,
            request.start_version,
            request.expected_num_transactions,
            request.include_events,
        )?;

        Ok(StorageServiceResponse::TransactionsWithProof(
            transactions_with_proof,
        ))
    }
}

/// The interface into local storage (e.g., the Diem DB) used by the storage
/// server to handle client requests.
pub trait StorageReaderInterface {
    /// Returns a data summary of the underlying storage state.
    fn get_data_summary(&self) -> Result<DataSummary, Error>;

    /// Returns a list of transactions with a proof relative to the
    /// `proof_version`. The transaction list is expected to contain *at most*
    /// `expected_num_transactions` and start at `start_version`.
    /// If `include_events` is true, events are also returned.
    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        expected_num_transactions: u64,
        include_events: bool,
    ) -> Result<TransactionListWithProof, Error>;

    /// Returns a list of epoch ending ledger infos, starting at `start_epoch`
    /// and ending *at most* at the `expected_end_epoch`.
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> Result<EpochChangeProof, Error>;

    // TODO(joshlind): support me!
    //
    // Returns a list of transaction outputs with a proof relative to the
    // `proof_version`. The transaction output list is expected to contain
    // *at most* `expected_num_transaction_outputs` and start at `start_version`.
    //fn get_transaction_outputs_with_proof(
    //    &self,
    //    proof_version: u64,
    //    start_version: u64,
    //    expected_num_transaction_outputs: u64,
    //) -> Result<TransactionOutputListWithProof, Error>;

    // TODO(joshlind): support me!
    //
    // Returns an AccountStateChunk holding a list of account states
    // starting at the specified account key with *at most*
    // `expected_num_account_states`.
    //fn get_account_states_chunk(
    //    version,
    //    start_account_key,
    //    expected_num_account_states: u64,
    //) -> Result<AccountStateChunk, Error>
}

/// The underlying implementation of the StorageReaderInterface, used by the
/// storage server.
pub struct StorageReader {
    storage: Arc<RwLock<DbReaderWriter>>,
}

impl StorageReader {
    pub fn new(storage: Arc<RwLock<DbReaderWriter>>) -> Self {
        Self { storage }
    }
}

impl StorageReaderInterface for StorageReader {
    fn get_data_summary(&self) -> Result<DataSummary, Error> {
        // Fetch the latest ledger info
        let latest_ledger_info_with_sigs = self
            .storage
            .read()
            .reader
            .get_latest_ledger_info()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        let latest_ledger_info = latest_ledger_info_with_sigs.ledger_info();

        // Return the relevant data summary
        // TODO(joshlind): Update the DiemDB to support fetching the lowest txn version and epoch!
        let data_summary = DataSummary {
            highest_transaction_version: latest_ledger_info.version(),
            lowest_transaction_version: 0,
            highest_epoch: latest_ledger_info.epoch(),
            lowest_epoch: 0,
        };
        Ok(data_summary)
    }

    fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        expected_num_transactions: u64,
        include_events: bool,
    ) -> Result<TransactionListWithProof, Error> {
        let transaction_list_with_proof = self
            .storage
            .read()
            .reader
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
        let epoch_change_proof = self
            .storage
            .read()
            .reader
            .get_epoch_ending_ledger_infos(start_epoch, expected_end_epoch)
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        Ok(epoch_change_proof)
    }
}
