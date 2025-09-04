// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::COMPRESSION_SUFFIX_LABEL;
use velor_types::transaction::Version;
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
    GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest), // Optimistically fetches new transaction outputs
    GetNewTransactionsWithProof(NewTransactionsWithProofRequest), // Optimistically fetches new transactions
    GetNumberOfStatesAtVersion(Version), // Fetches the number of states at the specified version
    GetServerProtocolVersion,            // Fetches the protocol version run by the server
    GetStateValuesWithProof(StateValuesWithProofRequest), // Fetches a list of states with a proof
    GetStorageServerSummary,             // Fetches a summary of the storage server state
    GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest), // Fetches a list of transaction outputs with a proof
    GetTransactionsWithProof(TransactionsWithProofRequest), // Fetches a list of transactions with a proof
    GetNewTransactionsOrOutputsWithProof(NewTransactionsOrOutputsWithProofRequest), // Optimistically fetches new transactions or outputs
    GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest), // Fetches a list of transactions or outputs with a proof
    SubscribeTransactionOutputsWithProof(SubscribeTransactionOutputsWithProofRequest), // Subscribes to transaction outputs with a proof
    SubscribeTransactionsOrOutputsWithProof(SubscribeTransactionsOrOutputsWithProofRequest), // Subscribes to transactions or outputs with a proof
    SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest), // Subscribes to transactions with a proof

    // All the requests listed below are for transaction data v2 (i.e., transactions with auxiliary information).
    // TODO: eventually we should deprecate all the old request types.
    GetTransactionDataWithProof(GetTransactionDataWithProofRequest), // Fetches transaction data with a proof
    GetNewTransactionDataWithProof(GetNewTransactionDataWithProofRequest), // Optimistically fetches new transaction data with a proof
    SubscribeTransactionDataWithProof(SubscribeTransactionDataWithProofRequest), // Subscribes to transaction data with a proof
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
            Self::GetNewTransactionsOrOutputsWithProof(_) => {
                "get_new_transactions_or_outputs_with_proof"
            },
            Self::GetTransactionsOrOutputsWithProof(_) => "get_transactions_or_outputs_with_proof",
            Self::SubscribeTransactionOutputsWithProof(_) => {
                "subscribe_transaction_outputs_with_proof"
            },
            Self::SubscribeTransactionsOrOutputsWithProof(_) => {
                "subscribe_transactions_or_outputs_with_proof"
            },
            Self::SubscribeTransactionsWithProof(_) => "subscribe_transactions_with_proof",

            // Transaction data v2 requests (transactions with auxiliary data)
            Self::GetTransactionDataWithProof(request) => match request
                .transaction_data_request_type
            {
                TransactionDataRequestType::TransactionData(_) => "get_transactions_with_proof_v2",
                TransactionDataRequestType::TransactionOutputData => {
                    "get_transaction_outputs_with_proof_v2"
                },
                TransactionDataRequestType::TransactionOrOutputData(_) => {
                    "get_transactions_or_outputs_with_proof_v2"
                },
            },
            Self::GetNewTransactionDataWithProof(request) => {
                match request.transaction_data_request_type {
                    TransactionDataRequestType::TransactionData(_) => {
                        "get_new_transactions_with_proof_v2"
                    },
                    TransactionDataRequestType::TransactionOutputData => {
                        "get_new_transaction_outputs_with_proof_v2"
                    },
                    TransactionDataRequestType::TransactionOrOutputData(_) => {
                        "get_new_transactions_or_outputs_with_proof_v2"
                    },
                }
            },
            Self::SubscribeTransactionDataWithProof(request) => {
                match request.transaction_data_request_type {
                    TransactionDataRequestType::TransactionData(_) => {
                        "subscribe_transactions_with_proof_v2"
                    },
                    TransactionDataRequestType::TransactionOutputData => {
                        "subscribe_transaction_outputs_with_proof_v2"
                    },
                    TransactionDataRequestType::TransactionOrOutputData(_) => {
                        "subscribe_transactions_or_outputs_with_proof_v2"
                    },
                }
            },
        }
    }

    /// Returns true iff the request is an optimistic fetch request
    pub fn is_optimistic_fetch(&self) -> bool {
        matches!(self, &Self::GetNewTransactionOutputsWithProof(_))
            || matches!(self, &Self::GetNewTransactionsWithProof(_))
            || matches!(self, Self::GetNewTransactionsOrOutputsWithProof(_))
            || matches!(self, &Self::GetNewTransactionDataWithProof(_))
    }

    /// Returns true iff the request is a protocol version request
    pub fn is_protocol_version_request(&self) -> bool {
        matches!(self, &Self::GetServerProtocolVersion)
    }

    /// Returns true iff the request is a storage summary request
    pub fn is_storage_summary_request(&self) -> bool {
        matches!(self, &Self::GetStorageServerSummary)
    }

    /// Returns true iff the request is a subscription request
    pub fn is_subscription_request(&self) -> bool {
        matches!(self, &Self::SubscribeTransactionOutputsWithProof(_))
            || matches!(self, &Self::SubscribeTransactionsWithProof(_))
            || matches!(self, Self::SubscribeTransactionsOrOutputsWithProof(_))
            || matches!(self, Self::SubscribeTransactionDataWithProof(_))
    }

    /// Returns true iff the request is a transaction data v2 request
    pub fn is_transaction_data_v2_request(&self) -> bool {
        matches!(self, &Self::GetTransactionDataWithProof(_))
            || matches!(self, &Self::GetNewTransactionDataWithProof(_))
            || matches!(self, &Self::SubscribeTransactionDataWithProof(_))
    }

    /// Creates and returns a request to get transaction data with a proof
    pub fn get_transaction_data_with_proof(
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionData(TransactionData { include_events });
        Self::GetTransactionDataWithProof(GetTransactionDataWithProofRequest {
            transaction_data_request_type,
            proof_version,
            start_version,
            end_version,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to get new transaction output data with a proof
    pub fn get_transaction_output_data_with_proof(
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type = TransactionDataRequestType::TransactionOutputData;
        Self::GetTransactionDataWithProof(GetTransactionDataWithProofRequest {
            transaction_data_request_type,
            proof_version,
            start_version,
            end_version,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to get new transaction or output data with a proof
    pub fn get_transaction_or_output_data_with_proof(
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionOrOutputData(TransactionOrOutputData {
                include_events,
            });
        Self::GetTransactionDataWithProof(GetTransactionDataWithProofRequest {
            transaction_data_request_type,
            proof_version,
            start_version,
            end_version,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to get new transaction data with a proof
    pub fn get_new_transaction_data_with_proof(
        known_version: u64,
        known_epoch: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionData(TransactionData { include_events });
        Self::GetNewTransactionDataWithProof(GetNewTransactionDataWithProofRequest {
            transaction_data_request_type,
            known_version,
            known_epoch,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to get new transaction output data with a proof
    pub fn get_new_transaction_output_data_with_proof(
        known_version: u64,
        known_epoch: u64,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type = TransactionDataRequestType::TransactionOutputData;
        Self::GetNewTransactionDataWithProof(GetNewTransactionDataWithProofRequest {
            transaction_data_request_type,
            known_version,
            known_epoch,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to get new transaction or output data with a proof
    pub fn get_new_transaction_or_output_data_with_proof(
        known_version: u64,
        known_epoch: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionOrOutputData(TransactionOrOutputData {
                include_events,
            });
        Self::GetNewTransactionDataWithProof(GetNewTransactionDataWithProofRequest {
            transaction_data_request_type,
            known_version,
            known_epoch,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to subscribe to transaction with a proof
    pub fn subscribe_transaction_data_with_proof(
        subscription_stream_metadata: SubscriptionStreamMetadata,
        subscription_stream_index: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionData(TransactionData { include_events });
        Self::SubscribeTransactionDataWithProof(SubscribeTransactionDataWithProofRequest {
            transaction_data_request_type,
            subscription_stream_metadata,
            subscription_stream_index,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to subscribe to transaction output with a proof
    pub fn subscribe_transaction_output_data_with_proof(
        subscription_stream_metadata: SubscriptionStreamMetadata,
        subscription_stream_index: u64,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type = TransactionDataRequestType::TransactionOutputData;
        Self::SubscribeTransactionDataWithProof(SubscribeTransactionDataWithProofRequest {
            transaction_data_request_type,
            subscription_stream_metadata,
            subscription_stream_index,
            max_response_bytes,
        })
    }

    /// Creates and returns a request to subscribe to transaction or output with a proof
    pub fn subscribe_transaction_or_output_data_with_proof(
        subscription_stream_metadata: SubscriptionStreamMetadata,
        subscription_stream_index: u64,
        include_events: bool,
        max_response_bytes: u64,
    ) -> Self {
        let transaction_data_request_type =
            TransactionDataRequestType::TransactionOrOutputData(TransactionOrOutputData {
                include_events,
            });
        Self::SubscribeTransactionDataWithProof(SubscribeTransactionDataWithProofRequest {
            transaction_data_request_type,
            subscription_stream_metadata,
            subscription_stream_index,
            max_response_bytes,
        })
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

/// A storage service request for fetching a new transaction or output list
/// beyond the already known version and epoch.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NewTransactionsOrOutputsWithProofRequest {
    pub known_version: u64,             // The highest known version
    pub known_epoch: u64,               // The highest known epoch
    pub include_events: bool,           // Whether or not to include events in the response
    pub max_num_output_reductions: u64, // The max num of output reductions before transactions are returned
}

/// A storage service request for fetching a transaction list with a
/// corresponding proof or an output list with a corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TransactionsOrOutputsWithProofRequest {
    pub proof_version: u64,   // The version the proof should be relative to
    pub start_version: u64,   // The starting version of the transaction/output list
    pub end_version: u64,     // The ending version of the transaction/output list (inclusive)
    pub include_events: bool, // Whether or not to include events (if transactions are returned)
    pub max_num_output_reductions: u64, // The max num of output reductions before transactions are returned
}

/// A storage service request for subscribing to transaction
/// outputs with a corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscribeTransactionOutputsWithProofRequest {
    pub subscription_stream_metadata: SubscriptionStreamMetadata, // The metadata for the subscription stream request
    pub subscription_stream_index: u64, // The request index of the subscription stream
}

/// A storage service request for subscribing to transactions
/// or outputs with a corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscribeTransactionsOrOutputsWithProofRequest {
    pub subscription_stream_metadata: SubscriptionStreamMetadata, // The metadata for the subscription stream request
    pub subscription_stream_index: u64, // The request index of the subscription stream
    pub include_events: bool,           // Whether or not to include events in the response
    pub max_num_output_reductions: u64, // The max num of output reductions before transactions are returned
}

/// A storage service request for subscribing to transactions
/// with a corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscribeTransactionsWithProofRequest {
    pub subscription_stream_metadata: SubscriptionStreamMetadata, // The metadata for the subscription stream request
    pub subscription_stream_index: u64, // The request index of the subscription stream
    pub include_events: bool,           // Whether or not to include events in the response
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscriptionStreamMetadata {
    pub known_version_at_stream_start: u64, // The highest known transaction version at stream start
    pub known_epoch_at_stream_start: u64,   // The highest known epoch at stream start
    pub subscription_stream_id: u64,        // The unique id of the subscription stream
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct GetTransactionDataWithProofRequest {
    pub transaction_data_request_type: TransactionDataRequestType, // The type of transaction data to request
    pub proof_version: u64,      // The version the proof should be relative to
    pub start_version: u64,      // The starting version of the data
    pub end_version: u64,        // The ending version of the data (inclusive)
    pub max_response_bytes: u64, // The max number of bytes to return in the response
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct GetNewTransactionDataWithProofRequest {
    pub transaction_data_request_type: TransactionDataRequestType, // The type of transaction data to request
    pub known_version: u64,                                        // The highest known version
    pub known_epoch: u64,                                          // The highest known epoch
    pub max_response_bytes: u64, // The max number of bytes to return in the response
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscribeTransactionDataWithProofRequest {
    pub transaction_data_request_type: TransactionDataRequestType, // The type of transaction data to request
    pub subscription_stream_metadata: SubscriptionStreamMetadata, // The metadata for the subscription stream request
    pub subscription_stream_index: u64, // The request index of the subscription stream
    pub max_response_bytes: u64,        // The max number of bytes to return in the response
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TransactionDataRequestType {
    TransactionData(TransactionData),
    TransactionOutputData,
    TransactionOrOutputData(TransactionOrOutputData),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TransactionData {
    pub include_events: bool, // Whether to include events with the transactions
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TransactionOrOutputData {
    pub include_events: bool, // Whether to include events with the transactions
}
