// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use diem_types::{
    epoch_change::EpochChangeProof,
    transaction::default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
};

/// A storage service request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageServiceRequest {
    GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest), // Fetches a list of epoch ending ledger infos
    GetServerProtocolVersion, // Fetches the protocol version run by the server
    GetStorageServerSummary,  // Fetches a summary of the storage server state
    GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest), // Fetches a list of transaction outputs with a proof
    GetTransactionsWithProof(TransactionsWithProofRequest), // Fetches a list of transactions with a proof
}

/// A storage service response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageServiceResponse {
    EpochEndingLedgerInfos(EpochChangeProof),
    ServerProtocolVersion(ServerProtocolVersion),
    StorageServiceError(StorageServiceError),
    StorageServerSummary(StorageServerSummary),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

/// A storage service request for fetching a transaction output list with a
/// corresponding proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionOutputsWithProofRequest {
    pub proof_version: u64,        // The version the proof should be relative to
    pub start_version: u64,        // The starting version of the transaction output list
    pub expected_num_outputs: u64, // Expected number of transaction outputs in the list
}

/// A storage service request for fetching a transaction list with a
/// corresponding proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionsWithProofRequest {
    pub proof_version: u64, // The version the proof should be relative to
    pub start_version: u64, // The starting version of the transaction list
    pub expected_num_transactions: u64, // Expected number of transactions in the list
    pub include_events: bool, // Whether or not to include events in the response
}

/// A storage service request for fetching a list of epoch ending ledger infos.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpochEndingLedgerInfoRequest {
    pub start_epoch: u64,
    pub expected_end_epoch: u64,
}

/// A storage service error that can be returned to the client on a failure
/// to process a service request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageServiceError {
    InternalError,
}

/// The protocol version run by this server. Clients request this first to
/// identify what API calls and data requests the server supports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerProtocolVersion {
    pub protocol_version: u64, // The storage server version run by this instance.
}

/// A storage server summary, containing a summary of the information held
/// by the corresponding server instance. This is useful for identifying the
/// data that a server instance can provide, as well as relevant metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageServerSummary {
    pub protocol_metadata: ProtocolMetadata,
    pub data_summary: DataSummary,
}

/// A summary of the protocol metadata for the storage service instance, such as
/// the maximum chunk sizes supported for different requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolMetadata {
    pub max_transaction_chunk_size: u64, // The max number of transaction the server can return in a single chunk
}

/// A summary of the data actually held by the storage service instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSummary {
    pub highest_transaction_version: u64, // The highest transaction version currently synced
    pub lowest_transaction_version: u64,  // The lowest transaction version currently stored

    pub highest_epoch: u64, // The highest epoch currently synced
    pub lowest_epoch: u64,  // The lowest epoch currently stored
}
