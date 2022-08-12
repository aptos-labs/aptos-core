// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::COMPRESSION_SUFFIX_LABEL;
use aptos_types::transaction::Version;
use serde::{Deserialize, Serialize};

/// A storage service request.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct StorageServiceRequest {
    pub data_request: DataRequest, // The data to fetch from the storage service
    pub use_compression: bool,     // Whether or not the client wishes data to be compressed
}

impl StorageServiceRequest {
    pub fn new(data_request: DataRequest, use_compression: bool) -> Self {
        Self {
            data_request,
            use_compression,
        }
    }

    /// Returns a summary label for the request
    pub fn get_label(&self) -> String {
        let mut label = self.data_request.get_label().to_string();
        if self.use_compression {
            label += COMPRESSION_SUFFIX_LABEL;
        }
        label
    }
}

/// A single data request.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum DataRequest {
    GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest), // Fetches a list of epoch ending ledger infos
    GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest), // Subscribes to new transaction outputs
    GetNewTransactionsWithProof(NewTransactionsWithProofRequest), // Subscribes to new transactions with a proof
    GetNumberOfStatesAtVersion(Version), // Fetches the number of states at the specified version
    GetServerProtocolVersion,            // Fetches the protocol version run by the server
    GetStateValuesWithProof(StateValuesWithProofRequest), // Fetches a list of states with a proof
    GetStorageServerSummary,             // Fetches a summary of the storage server state
    GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest), // Fetches a list of transaction outputs with a proof
    GetTransactionsWithProof(TransactionsWithProofRequest), // Fetches a list of transactions with a proof
}

impl DataRequest {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::GetEpochEndingLedgerInfos(_) => "get_epoch_ending_ledger_infos",
            Self::GetNewTransactionOutputsWithProof(_) => "get_new_transaction_outputs_with_proof",
            Self::GetNewTransactionsWithProof(_) => "get_new_transactions_with_proof",
            Self::GetNumberOfStatesAtVersion(_) => "get_number_of_states_at_version",
            Self::GetServerProtocolVersion => "get_server_protocol_version",
            Self::GetStateValuesWithProof(_) => "get_state_values_with_proof",
            Self::GetStorageServerSummary => "get_storage_server_summary",
            Self::GetTransactionOutputsWithProof(_) => "get_transaction_outputs_with_proof",
            Self::GetTransactionsWithProof(_) => "get_transactions_with_proof",
        }
    }

    pub fn is_storage_summary_request(&self) -> bool {
        matches!(self, &Self::GetStorageServerSummary)
    }

    pub fn is_data_subscription_request(&self) -> bool {
        matches!(self, &Self::GetNewTransactionOutputsWithProof(_))
            || matches!(self, &Self::GetNewTransactionsWithProof(_))
    }

    pub fn is_protocol_version_request(&self) -> bool {
        matches!(self, &Self::GetServerProtocolVersion)
    }
}

/// A storage service request for fetching a list of epoch ending ledger infos.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct EpochEndingLedgerInfoRequest {
    pub start_epoch: u64,        // The epoch to start at
    pub expected_end_epoch: u64, // The epoch to finish at
}

/// A storage service request for fetching a new transaction output list
/// beyond the already known version and epoch.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NewTransactionOutputsWithProofRequest {
    pub known_version: u64, // The highest known output version
    pub known_epoch: u64,   // The highest known epoch
}

/// A storage service request for fetching a new transaction list
/// beyond the already known version and epoch.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NewTransactionsWithProofRequest {
    pub known_version: u64,   // The highest known transaction version
    pub known_epoch: u64,     // The highest known epoch
    pub include_events: bool, // Whether or not to include events in the response
}

/// A storage service request for fetching a list of state
/// values at a specified version.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct StateValuesWithProofRequest {
    pub version: u64,     // The version to fetch the state values at
    pub start_index: u64, // The index to start fetching state values (inclusive)
    pub end_index: u64,   // The index to stop fetching state values (inclusive)
}

/// A storage service request for fetching a transaction output list with a
/// corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TransactionOutputsWithProofRequest {
    pub proof_version: u64, // The version the proof should be relative to
    pub start_version: u64, // The starting version of the transaction output list
    pub end_version: u64,   // The ending version of the transaction output list (inclusive)
}

/// A storage service request for fetching a transaction list with a
/// corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TransactionsWithProofRequest {
    pub proof_version: u64,   // The version the proof should be relative to
    pub start_version: u64,   // The starting version of the transaction list
    pub end_version: u64,     // The ending version of the transaction list (inclusive)
    pub include_events: bool, // Whether or not to include events in the response
}
