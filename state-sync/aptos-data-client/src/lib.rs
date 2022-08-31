// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};
use storage_service_types::{responses::CompleteDataRange, Epoch};
use thiserror::Error;

pub type ResponseId = u64;

pub mod aptosnet;

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

// TODO(philiphayes): a Error { kind: ErrorKind, inner: BoxError } would be more convenient
/// An error returned by the Aptos Data Client for failed API calls.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
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

impl Error {
    /// Returns a summary label for the error
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::DataIsUnavailable(_) => "data_is_unavailable",
            Self::DataIsTooLarge(_) => "data_is_too_large",
            Self::InvalidRequest(_) => "invalid_request",
            Self::InvalidResponse(_) => "invalid_response",
            Self::TimeoutWaitingForResponse(_) => "timeout_waiting_for_response",
            Self::UnexpectedErrorEncountered(_) => "unexpected_error_encountered",
        }
    }
}

impl From<storage_service_types::responses::Error> for Error {
    fn from(error: storage_service_types::responses::Error) -> Self {
        Self::InvalidResponse(error.to_string())
    }
}

/// The API offered by the Aptos Data Client.
#[async_trait]
pub trait AptosDataClient {
    /// Fetches a global summary of the data currently available in the network.
    ///
    /// This API is intended to be relatively cheap to call, usually returning a
    /// cached view of this data client's available data.
    fn get_global_data_summary(&self) -> GlobalDataSummary;

    /// Fetches the epoch ending ledger infos between start and end
    /// (inclusive). In some cases, fewer ledger infos may be returned (e.g.,
    /// to tolerate network or chunk limits). If the data cannot be fetched,
    /// an error is returned.
    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        expected_end_epoch: Epoch,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>>;

    /// Fetches a new transaction output list with proof. Versions start at
    /// `known_version + 1` and `known_epoch` (inclusive). The end version
    /// and proof version are specified by the server. If the data cannot be
    /// fetched, an error is returned.
    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
    ) -> Result<Response<(TransactionOutputListWithProof, LedgerInfoWithSignatures)>>;

    /// Fetches a new transaction list with proof. Versions start at
    /// `known_version + 1` and `known_epoch` (inclusive). The end version
    /// and proof version are specified by the server. If the data cannot be
    /// fetched, an error is returned.
    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
    ) -> Result<Response<(TransactionListWithProof, LedgerInfoWithSignatures)>>;

    /// Fetches the number of states at the specified version.
    async fn get_number_of_states(&self, version: Version) -> Result<Response<u64>>;

    /// Fetches a single state value chunk with proof, containing the values
    /// from start to end index (inclusive) at the specified version. The proof
    /// version is the same as the specified version. In some cases, fewer
    /// state values may be returned (e.g., to tolerate network or chunk
    /// limits). If the data cannot be fetched, an error is returned.
    async fn get_state_values_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> Result<Response<StateValueChunkWithProof>>;

    /// Fetches a transaction output list with proof, with transaction
    /// outputs from start to end versions (inclusive). The proof is relative
    /// to the specified `proof_version`. In some cases, fewer outputs may be
    /// returned (e.g., to tolerate network or chunk limits). If the data
    /// cannot be fetched, an error is returned.
    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
    ) -> Result<Response<TransactionOutputListWithProof>>;

    /// Fetches a transaction list with proof, with transactions from
    /// start to end versions (inclusive). The proof is relative to the
    /// specified `proof_version`. If `include_events` is true, events are
    /// included in the proof. In some cases, fewer transactions may be returned
    /// (e.g., to tolerate network or chunk limits). If the data cannot
    /// be fetched, an error is returned.
    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
    ) -> Result<Response<TransactionListWithProof>>;
}

/// A response error that users of the Aptos Data Client can use to notify
/// the Data Client about invalid or malformed responses.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ResponseError {
    InvalidData,
    InvalidPayloadDataType,
    ProofVerificationError,
}

/// A callback that lets the consumer provide error feedback about a response.
/// Typically, this will contain a reference to the underlying data client and
/// any additional request context needed to update internal scoring.
///
/// This feedback mechanism is required because a Data Client is not always able
/// to fully verify that a given data response is valid (e.g., it is unable
/// to verify all proofs).
///
/// This trait provides a simple feedback mechanism for users of the Data Client
/// to alert it to bad responses so that the peers responsible for providing this
/// data can be penalized.
pub trait ResponseCallback: fmt::Debug + Send + Sync + 'static {
    // TODO(philiphayes): ideally this would take a `self: Box<Self>`, i.e.,
    // consume the callback, which better communicates that you should only report
    // an error once. however, the current state-sync-v2 code makes this difficult...
    fn notify_bad_response(&self, error: ResponseError);
}

#[derive(Debug)]
pub struct ResponseContext {
    /// A unique identifier for this request/response pair. Intended mostly for
    /// debugging.
    pub id: ResponseId,
    /// A callback for notifying the data-client source about an error with this
    /// response.
    pub response_callback: Box<dyn ResponseCallback>,
}

/// A response from the Data Client for a single API call.
#[derive(Debug)]
pub struct Response<T> {
    /// Additional context.
    pub context: ResponseContext,
    /// The actual response payload.
    pub payload: T,
}

impl<T> Response<T> {
    pub fn new(context: ResponseContext, payload: T) -> Self {
        Self { context, payload }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }

    pub fn into_parts(self) -> (ResponseContext, T) {
        (self.context, self.payload)
    }

    pub fn map<U, F>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        let (context, payload) = self.into_parts();
        Response::new(context, f(payload))
    }
}

/// The different data client response payloads as an enum.
#[derive(Debug, Eq, PartialEq)]
pub enum ResponsePayload {
    EpochEndingLedgerInfos(Vec<LedgerInfoWithSignatures>),
    NewTransactionOutputsWithProof((TransactionOutputListWithProof, LedgerInfoWithSignatures)),
    NewTransactionsWithProof((TransactionListWithProof, LedgerInfoWithSignatures)),
    NumberOfStates(u64),
    StateValuesWithProof(StateValueChunkWithProof),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

impl ResponsePayload {
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::EpochEndingLedgerInfos(_) => "epoch_ending_ledger_infos",
            Self::NewTransactionOutputsWithProof(_) => "new_transaction_outputs_with_proof",
            Self::NewTransactionsWithProof(_) => "new_transactions_with_proof",
            Self::NumberOfStates(_) => "number_of_states",
            Self::StateValuesWithProof(_) => "state_values_with_proof",
            Self::TransactionOutputsWithProof(_) => "transaction_outputs_with_proof",
            Self::TransactionsWithProof(_) => "transactions_with_proof",
        }
    }
}

// Conversions from the inner enum variants to the outer enum

impl From<StateValueChunkWithProof> for ResponsePayload {
    fn from(inner: StateValueChunkWithProof) -> Self {
        Self::StateValuesWithProof(inner)
    }
}

impl From<Vec<LedgerInfoWithSignatures>> for ResponsePayload {
    fn from(inner: Vec<LedgerInfoWithSignatures>) -> Self {
        Self::EpochEndingLedgerInfos(inner)
    }
}

impl From<(TransactionOutputListWithProof, LedgerInfoWithSignatures)> for ResponsePayload {
    fn from(inner: (TransactionOutputListWithProof, LedgerInfoWithSignatures)) -> Self {
        Self::NewTransactionOutputsWithProof(inner)
    }
}

impl From<(TransactionListWithProof, LedgerInfoWithSignatures)> for ResponsePayload {
    fn from(inner: (TransactionListWithProof, LedgerInfoWithSignatures)) -> Self {
        Self::NewTransactionsWithProof(inner)
    }
}

impl From<u64> for ResponsePayload {
    fn from(inner: u64) -> Self {
        Self::NumberOfStates(inner)
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

/// A snapshot of the global state of data available in the Aptos network.
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

    /// Returns true iff the global data summary is empty
    pub fn is_empty(&self) -> bool {
        self == &Self::empty()
    }
}

impl Display for GlobalDataSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, {:?}",
            self.advertised_data, self.optimal_chunk_sizes
        )
    }
}

/// Holds the optimal chunk sizes that clients should use when
/// requesting data. This makes the request *more likely* to succeed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimalChunkSizes {
    pub epoch_chunk_size: u64,
    pub state_chunk_size: u64,
    pub transaction_chunk_size: u64,
    pub transaction_output_chunk_size: u64,
}

impl OptimalChunkSizes {
    pub fn empty() -> Self {
        OptimalChunkSizes {
            epoch_chunk_size: 0,
            state_chunk_size: 0,
            transaction_chunk_size: 0,
            transaction_output_chunk_size: 0,
        }
    }
}

/// A summary of all data that is currently advertised in the network.
#[derive(Clone, Eq, PartialEq)]
pub struct AdvertisedData {
    /// The ranges of epoch ending ledger infos advertised, e.g., if a range
    /// is (X,Y), it means all epoch ending ledger infos for epochs X->Y
    /// (inclusive) are available.
    pub epoch_ending_ledger_infos: Vec<CompleteDataRange<Epoch>>,

    /// The ranges of states advertised, e.g., if a range is
    /// (X,Y), it means all states are held for every version X->Y
    /// (inclusive).
    pub states: Vec<CompleteDataRange<Version>>,

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

impl fmt::Debug for AdvertisedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let synced_ledger_infos = (&self.synced_ledger_infos)
            .iter()
            .map(|LedgerInfoWithSignatures::V0(ledger)| {
                let version = ledger.commit_info().version();
                let epoch = ledger.commit_info().epoch();
                let ends_epoch = ledger.commit_info().next_epoch_state().is_some();
                format!(
                    "(Version: {:?}, Epoch: {:?}, Ends epoch: {:?})",
                    version, epoch, ends_epoch
                )
            })
            .join(", ");
        write!(
            f,
            "epoch_ending_ledger_infos: {:?}, states: {:?}, synced_ledger_infos: [{}], transactions: {:?}, transaction_outputs: {:?}",
            &self.epoch_ending_ledger_infos, &self.states, synced_ledger_infos, &self.transactions, &self.transaction_outputs
        )
    }
}

/// Provides an aggregated version of all advertised data (i.e, highest and lowest)
impl Display for AdvertisedData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Calculate the highest advertised data
        let highest_epoch_ending_ledger_info = self.highest_epoch_ending_ledger_info();
        let highest_synced_ledger_info = self.highest_synced_ledger_info();
        let highest_synced_version = highest_synced_ledger_info
            .as_ref()
            .map(|li| li.ledger_info().version());
        let highest_synced_epoch = highest_synced_ledger_info.map(|li| li.ledger_info().epoch());

        // Calculate the lowest advertised data
        let lowest_transaction_version = self.lowest_transaction_version();
        let lowest_output_version = self.lowest_transaction_output_version();
        let lowest_states_version = self.lowest_state_version();

        write!(
            f,
            "AdvertisedData {{ Highest epoch ending ledger info, epoch: {:?}. Highest synced ledger info, epoch: {:?}, version: {:?}. \
            Lowest transaction version: {:?}, Lowest transaction output version: {:?}, Lowest states version: {:?} }}",
            highest_epoch_ending_ledger_info, highest_synced_epoch, highest_synced_version,
            lowest_transaction_version, lowest_output_version, lowest_states_version
        )
    }
}

impl AdvertisedData {
    pub fn empty() -> Self {
        AdvertisedData {
            epoch_ending_ledger_infos: vec![],
            states: vec![],
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

    /// Returns the highest epoch ending ledger info advertised in the network
    pub fn highest_epoch_ending_ledger_info(&self) -> Option<Epoch> {
        self.epoch_ending_ledger_infos
            .iter()
            .map(|epoch_range| epoch_range.highest())
            .max()
    }

    /// Returns the highest synced ledger info advertised in the network
    pub fn highest_synced_ledger_info(&self) -> Option<LedgerInfoWithSignatures> {
        let highest_synced_position = self
            .synced_ledger_infos
            .iter()
            .map(|ledger_info_with_sigs| ledger_info_with_sigs.ledger_info().version())
            .position_max();

        if let Some(highest_synced_position) = highest_synced_position {
            self.synced_ledger_infos
                .get(highest_synced_position)
                .cloned()
        } else {
            None
        }
    }

    /// Returns the lowest advertised version containing all states
    pub fn lowest_state_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.states)
    }

    /// Returns the lowest advertised transaction output version
    pub fn lowest_transaction_output_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.transaction_outputs)
    }

    /// Returns the lowest advertised transaction version
    pub fn lowest_transaction_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.transactions)
    }
}

/// Returns the lowest version from the given set of data ranges
fn get_lowest_version_from_range_set(
    data_ranges: &[CompleteDataRange<Version>],
) -> Option<Version> {
    data_ranges
        .iter()
        .map(|data_range| data_range.lowest())
        .min()
}
