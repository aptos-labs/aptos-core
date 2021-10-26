// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use diem_types::{
    account_state_blob::AccountStatesChunkWithProof,
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
        Version,
    },
};
use num_traits::identities::Zero;
use serde::{de, Deserialize, Serialize};
use thiserror::Error;

pub type Result<T, E = StorageServiceError> = ::std::result::Result<T, E>;

/// A storage service error that can be returned to the client on a failure
/// to process a service request.
#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub enum StorageServiceError {
    #[error("Internal service error")]
    InternalError,
}

/// A single storage service message sent or received over DiemNet.
#[derive(Clone, Debug, Deserialize, Serialize)]
// TODO(philiphayes): do something about this without making it ugly :(
#[allow(clippy::large_enum_variant)]
pub enum StorageServiceMessage {
    /// A request to the storage service.
    Request(StorageServiceRequest),
    /// A response from the storage service. If there was an error while handling
    /// the request, the service will return an [`StorageServiceError`] error.
    Response(Result<StorageServiceResponse>),
}

/// A storage service request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StorageServiceRequest {
    GetAccountStatesChunkWithProof(AccountStatesChunkWithProofRequest), // Fetches a list of account states with a proof
    GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest), // Fetches a list of epoch ending ledger infos
    GetNumberOfAccountsAtVersion(Version), // Fetches the number of accounts at the specified version
    GetServerProtocolVersion,              // Fetches the protocol version run by the server
    GetStorageServerSummary,               // Fetches a summary of the storage server state
    GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest), // Fetches a list of transaction outputs with a proof
    GetTransactionsWithProof(TransactionsWithProofRequest), // Fetches a list of transactions with a proof
}

impl StorageServiceRequest {
    pub fn is_get_storage_server_summary(&self) -> bool {
        matches!(self, &Self::GetStorageServerSummary)
    }
}

/// A storage service response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
// TODO(philiphayes): do something about this without making it ugly :(
#[allow(clippy::large_enum_variant)]
pub enum StorageServiceResponse {
    AccountStatesChunkWithProof(AccountStatesChunkWithProof),
    EpochEndingLedgerInfos(EpochChangeProof),
    NumberOfAccountsAtVersion(u64),
    ServerProtocolVersion(ServerProtocolVersion),
    StorageServerSummary(StorageServerSummary),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

/// A storage service request for fetching a list of account states at a
/// specified version.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountStatesChunkWithProofRequest {
    pub version: u64,                     // The version to fetch the account states at
    pub start_account_index: u64,         // The account index to start fetching account states
    pub expected_num_account_states: u64, // Expected number of account states to fetch
}

/// A storage service request for fetching a transaction output list with a
/// corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionOutputsWithProofRequest {
    pub proof_version: u64,        // The version the proof should be relative to
    pub start_version: u64,        // The starting version of the transaction output list
    pub expected_num_outputs: u64, // Expected number of transaction outputs in the list
}

/// A storage service request for fetching a transaction list with a
/// corresponding proof.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionsWithProofRequest {
    pub proof_version: u64, // The version the proof should be relative to
    pub start_version: u64, // The starting version of the transaction list
    pub expected_num_transactions: u64, // Expected number of transactions in the list
    pub include_events: bool, // Whether or not to include events in the response
}

/// A storage service request for fetching a list of epoch ending ledger infos.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EpochEndingLedgerInfoRequest {
    pub start_epoch: u64,
    pub expected_end_epoch: u64,
}

/// The protocol version run by this server. Clients request this first to
/// identify what API calls and data requests the server supports.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerProtocolVersion {
    pub protocol_version: u64, // The storage server version run by this instance.
}

/// A storage server summary, containing a summary of the information held
/// by the corresponding server instance. This is useful for identifying the
/// data that a server instance can provide, as well as relevant metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StorageServerSummary {
    pub protocol_metadata: ProtocolMetadata,
    pub data_summary: DataSummary,
}

impl StorageServerSummary {
    pub fn can_service(&self, request: &StorageServiceRequest) -> bool {
        self.protocol_metadata.can_service(request) && self.data_summary.can_service(request)
    }
}

/// A summary of the protocol metadata for the storage service instance, such as
/// the maximum chunk sizes supported for different requests.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolMetadata {
    pub max_epoch_chunk_size: u64, // The max number of epochs the server can return in a single chunk
    pub max_transaction_chunk_size: u64, // The max number of transactions the server can return in a single chunk
    pub max_transaction_output_chunk_size: u64, // The max number of transaction outputs the server can return in a single chunk
    pub max_account_states_chunk_size: u64, // The max number of account states the server can return in a single chunk
}

impl ProtocolMetadata {
    pub fn can_service(&self, _request: &StorageServiceRequest) -> bool {
        // TODO(philiphayes): fill out
        true
    }
}

/// A type alias for different epochs.
pub type Epoch = u64;

/// A summary of the data actually held by the storage service instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataSummary {
    /// The ledger info corresponding to the highest synced version in storage.
    /// This indicates the highest version and epoch that storage can prove.
    pub synced_ledger_info: Option<LedgerInfoWithSignatures>,
    /// The range of epoch ending ledger infos in storage, e.g., if the range
    /// is [(X,Y)], it means all epoch ending ledger infos for epochs X->Y
    /// (inclusive) are held.
    pub epoch_ending_ledger_infos: Option<CompleteDataRange<Epoch>>,
    /// The range of transactions held in storage, e.g., if the range is
    /// [(X,Y)], it means all transactions for versions X->Y (inclusive) are held.
    pub transactions: Option<CompleteDataRange<Version>>,
    /// The range of transaction outputs held in storage, e.g., if the range
    /// is [(X,Y)], it means all transaction outputs for versions X->Y
    /// (inclusive) are held.
    pub transaction_outputs: Option<CompleteDataRange<Version>>,
    /// The range of account states held in storage, e.g., if the range is
    /// [(X,Y)], it means all account states are held for every version X->Y
    /// (inclusive).
    pub account_states: Option<CompleteDataRange<Version>>,
}

impl DataSummary {
    pub fn can_service(&self, _request: &StorageServiceRequest) -> bool {
        // TODO(philiphayes): fill out
        true
    }
}

#[derive(Clone, Debug, Error)]
#[error("data range cannot be degenerate (lowest > highest)")]
pub struct DegenerateRangeError;

/// A struct representing a contiguous, non-empty data range (lowest to highest,
/// inclusive) where data is complete (i.e. there are no missing pieces of data).
///
/// This is used to provide a summary of the data currently held in storage, e.g.
/// a CompleteDataRange<Version> of (A,B) means all versions A->B (inclusive).
///
/// Note: CompleteDataRanges are never degenerate (lowest > highest). Constructing
/// a degenerate range via `new` will return an `Err`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompleteDataRange<T> {
    lowest: T,
    highest: T,
}

impl<T: Copy + Ord> CompleteDataRange<T> {
    pub fn new(lowest: T, highest: T) -> Result<Self, DegenerateRangeError> {
        if lowest <= highest {
            Ok(Self { lowest, highest })
        } else {
            Err(DegenerateRangeError)
        }
    }

    #[inline]
    pub fn lowest(&self) -> T {
        self.lowest
    }

    #[inline]
    pub fn highest(&self) -> T {
        self.highest
    }

    /// Returns true iff the given data item is within this range
    pub fn contains(&self, data_item: T) -> bool {
        data_item >= self.lowest && data_item <= self.highest
    }
}

impl<T: Zero> CompleteDataRange<T> {
    pub fn from_genesis(highest: T) -> Self {
        Self {
            lowest: T::zero(),
            highest,
        }
    }
}

impl<'de, T> de::Deserialize<'de> for CompleteDataRange<T>
where
    T: Copy + Ord + de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        #[serde(rename = "CompleteDataRange")]
        struct Value<U> {
            lowest: U,
            highest: U,
        }

        let value = Value::<T>::deserialize(deserializer)?;
        Self::new(value.lowest, value.highest).map_err(D::Error::custom)
    }
}
