// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;
use serde::{Deserialize, Serialize};

/// A storage service request.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum StorageServiceRequest {
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

impl StorageServiceRequest {
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

    pub fn is_get_storage_server_summary(&self) -> bool {
        matches!(self, &Self::GetStorageServerSummary)
    }

    pub fn is_data_subscription_request(&self) -> bool {
        matches!(self, &Self::GetNewTransactionOutputsWithProof(_))
            || matches!(self, &Self::GetNewTransactionsWithProof(_))
    }
}

/// A storage service request for fetching a list of epoch ending ledger infos.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct EpochEndingLedgerInfoRequest {
    pub start_epoch: u64,
    pub expected_end_epoch: u64,
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
