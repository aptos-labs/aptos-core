// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use async_trait::async_trait;
use diem_types::{
    account_state_blob::AccountStatesChunkWithProof,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
        Version,
    },
};
use serde::{Deserialize, Serialize};
use storage_service::UnexpectedResponseError;
use storage_service_types::{self as storage_service, CompleteDataRange, Epoch};
use thiserror::Error;

pub type ResponseId = u64;

pub mod diemnet;

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

// TODO(philiphayes): a Error { kind: ErrorKind, inner: BoxError } would be more convenient
/// An error returned by the Diem Data Client for failed API calls.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Serialize)]
pub enum Error {
    #[error("The requested data is unavailable and cannot be found! Error: {0}")]
    DataIsUnavailable(String),

    #[error("The requested data is too large: {0}")]
    DataIsTooLarge(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timed out waiting for a response: {0}")]
    TimeoutWaitingForResponse(String),

    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

// TODO(philiphayes): better error wrapping
impl From<UnexpectedResponseError> for Error {
    fn from(err: UnexpectedResponseError) -> Self {
        Self::InvalidResponse(err.0)
    }
}

/// A response error that users of the Diem Data Client can use to notify
/// the Data Client about invalid or malformed responses.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ResponseError {
    InvalidData,
    InvalidPayloadDataType,
    ProofVerificationError,
}

/// The API offered by the Diem Data Client.
#[async_trait]
pub trait DiemDataClient {
    /// Returns a global summary of the data currently available in the network.
    ///
    /// This API is intended to be relatively cheap to call, usually returning a
    /// cached view of this data client's available data.
    fn get_global_data_summary(&self) -> GlobalDataSummary;

    /// Notifies the Diem Data Client about a previously received response that
    /// was bad (e.g., invalid or malformed).
    ///
    /// This API is intended to be relatively cheap to call.
    ///
    /// Note: this is required because the Diem Data Client can only fetch
    /// data from peers in the network, but it is not able to fully verify that
    /// the given data responses are valid (e.g., it is unable to verify proofs).
    /// This API call provides a simple feedback mechanism for users of the Diem
    /// Data Client to alert it to bad responses so that the peers responsible
    /// for providing this data can be penalized. The `response_id` is the handle
    /// used by clients to notify the Diem Data Client of invalid responses.
    fn notify_bad_response(&self, response_id: u64, response_error: ResponseError);

    /// Returns a single account states chunk with proof, containing the accounts
    /// from start to end index (inclusive) at the specified version. The proof
    /// version is the same as the specified version.
    async fn get_account_states_with_proof(
        &self,
        version: u64,
        start_account_index: u64,
        end_account_index: u64,
    ) -> Result<Response<AccountStatesChunkWithProof>>;

    /// Returns all epoch ending ledger infos between start and end (inclusive).
    /// If the data cannot be fetched (e.g., the number of epochs is too large),
    /// an error is returned.
    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        expected_end_epoch: Epoch,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>>;

    /// Returns the number of account states at the specified version.
    async fn get_number_of_account_states(&self, version: Version) -> Result<Response<u64>>;

    /// Returns a transaction output list with proof object, with transaction
    /// outputs from start to end versions (inclusive). The proof is relative to
    /// the specified `proof_version`. If the data cannot be fetched (e.g., the
    /// number of transaction outputs is too large), an error is returned.
    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
    ) -> Result<Response<TransactionOutputListWithProof>>;

    /// Returns a transaction list with proof object, with transactions from
    /// start to end versions (inclusive). The proof is relative to the specified
    /// `proof_version`. If `include_events` is true, events are included in the
    /// proof. If the data cannot be fetched (e.g., the number of transactions is
    /// too large), an error is returned.
    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
    ) -> Result<Response<TransactionListWithProof>>;
}

/// A response from the Data Client for a single API call.
///
/// Note: the `response_id` is a simple handle returned by the Diem Data Client
/// that allows API callers to notify the Diem Data Client that the given
/// response payload is bad (e.g., it contains invalid or malformed data, or
/// the proof failed verification). This can be done using the
/// `notify_bad_response()` API call above.
#[derive(Clone, Debug)]
pub struct Response<T> {
    pub id: ResponseId,
    pub payload: T,
}

impl<T> Response<T> {
    pub fn new(id: u64, payload: T) -> Self {
        Self { id, payload }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }

    pub fn into_parts(self) -> (ResponseId, T) {
        (self.id, self.payload)
    }

    pub fn map<U, F>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        let (id, payload) = self.into_parts();
        Response::new(id, f(payload))
    }

    pub fn and_then<U, E, F>(self, f: F) -> Result<Response<U>, E>
    where
        F: FnOnce(T) -> Result<U, E>,
    {
        let (id, payload) = self.into_parts();
        match f(payload) {
            Ok(new_payload) => Ok(Response::new(id, new_payload)),
            Err(err) => Err(err),
        }
    }
}

/// The different data client response payloads as an enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponsePayload {
    AccountStatesWithProof(AccountStatesChunkWithProof),
    EpochEndingLedgerInfos(Vec<LedgerInfoWithSignatures>),
    NumberOfAccountStates(u64),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

// Conversions from the inner enum variants to the outer enum

impl From<AccountStatesChunkWithProof> for ResponsePayload {
    fn from(inner: AccountStatesChunkWithProof) -> Self {
        Self::AccountStatesWithProof(inner)
    }
}
impl From<Vec<LedgerInfoWithSignatures>> for ResponsePayload {
    fn from(inner: Vec<LedgerInfoWithSignatures>) -> Self {
        Self::EpochEndingLedgerInfos(inner)
    }
}
impl From<u64> for ResponsePayload {
    fn from(inner: u64) -> Self {
        Self::NumberOfAccountStates(inner)
    }
}
impl From<TransactionOutputListWithProof> for ResponsePayload {
    fn from(inner: TransactionOutputListWithProof) -> Self {
        Self::TransactionOutputsWithProof(inner)
    }
}
impl From<TransactionListWithProof> for ResponsePayload {
    fn from(inner: TransactionListWithProof) -> Self {
        Self::TransactionsWithProof(inner)
    }
}

/// A snapshot of the global state of data available in the Diem network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalDataSummary {
    pub advertised_data: AdvertisedData,
    pub optimal_chunk_sizes: OptimalChunkSizes,
}

impl GlobalDataSummary {
    /// Returns an empty global data summary. This can be used on startup
    /// before the global state is known, or for testing.
    pub fn empty() -> Self {
        GlobalDataSummary {
            advertised_data: AdvertisedData::empty(),
            optimal_chunk_sizes: OptimalChunkSizes::empty(),
        }
    }
}

/// Holds the optimal chunk sizes that clients should use when
/// requesting data. This makes the request *more likely* to succeed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimalChunkSizes {
    pub account_states_chunk_size: u64,
    pub epoch_chunk_size: u64,
    pub transaction_chunk_size: u64,
    pub transaction_output_chunk_size: u64,
}

impl OptimalChunkSizes {
    pub fn empty() -> Self {
        OptimalChunkSizes {
            account_states_chunk_size: 0,
            epoch_chunk_size: 0,
            transaction_chunk_size: 0,
            transaction_output_chunk_size: 0,
        }
    }
}

/// A summary of all data that is currently advertised in the network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvertisedData {
    /// The ranges of account states advertised, e.g., if a range is
    /// (X,Y), it means all account states are held for every version X->Y
    /// (inclusive).
    pub account_states: Vec<CompleteDataRange<Version>>,

    /// The ranges of epoch ending ledger infos advertised, e.g., if a range
    /// is (X,Y), it means all epoch ending ledger infos for epochs X->Y
    /// (inclusive) are available.
    pub epoch_ending_ledger_infos: Vec<CompleteDataRange<Epoch>>,

    /// The ledger infos corresponding to the highest synced versions
    /// currently advertised.
    pub synced_ledger_infos: Vec<LedgerInfoWithSignatures>,

    /// The ranges of transactions advertised, e.g., if a range is
    /// (X,Y), it means all transactions for versions X->Y (inclusive)
    /// are available.
    pub transactions: Vec<CompleteDataRange<Version>>,

    /// The ranges of transaction outputs advertised, e.g., if a range
    /// is (X,Y), it means all transaction outputs for versions X->Y
    /// (inclusive) are available.
    pub transaction_outputs: Vec<CompleteDataRange<Version>>,
}

impl AdvertisedData {
    pub fn empty() -> Self {
        AdvertisedData {
            account_states: vec![],
            epoch_ending_ledger_infos: vec![],
            synced_ledger_infos: vec![],
            transactions: vec![],
            transaction_outputs: vec![],
        }
    }

    /// Returns true iff all data items (`lowest` to `highest`, inclusive) can
    /// be found in the given `advertised_ranges`.
    pub fn contains_range(
        lowest: u64,
        highest: u64,
        advertised_ranges: &[CompleteDataRange<u64>],
    ) -> bool {
        for item in lowest..=highest {
            let mut item_exists = false;

            for advertised_range in advertised_ranges {
                if advertised_range.contains(item) {
                    item_exists = true;
                    break;
                }
            }

            if !item_exists {
                return false;
            }
        }
        true
    }
}
