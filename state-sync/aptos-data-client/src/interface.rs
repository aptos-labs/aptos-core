// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error, error::Error, global_summary::GlobalDataSummary};
use aptos_storage_service_types::{
    responses::{TransactionOrOutputListWithProof, TransactionOrOutputListWithProofV2},
    Epoch,
};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        TransactionListWithProof, TransactionListWithProofV2, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, Version,
    },
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Instant};

/// The API offered by the Aptos Data Client.
#[async_trait]
pub trait AptosDataClientInterface {
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
        request_timeout_ms: u64,
    ) -> error::Result<Response<Vec<LedgerInfoWithSignatures>>>;

    /// Fetches a new transaction output list with proof. Versions start at
    /// `known_version + 1` and `known_epoch` (inclusive). The end version
    /// and proof version are specified by the server. If the data cannot be
    /// fetched, an error is returned.
    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>;

    /// Fetches a new transaction list with proof. Versions start at
    /// `known_version + 1` and `known_epoch` (inclusive). The end version
    /// and proof version are specified by the server. If the data cannot be
    /// fetched, an error is returned.
    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>;

    /// Fetches a new transaction or output list with proof. Versions start at
    /// `known_version + 1` and `known_epoch` (inclusive). The end version
    /// and proof version are specified by the server. If the data cannot be
    /// fetched, an error is returned.
    async fn get_new_transactions_or_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>>;

    /// Fetches the number of states at the specified version.
    async fn get_number_of_states(
        &self,
        version: Version,
        request_timeout_ms: u64,
    ) -> error::Result<Response<u64>>;

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
        request_timeout_ms: u64,
    ) -> error::Result<Response<StateValueChunkWithProof>>;

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
        request_timeout_ms: u64,
    ) -> error::Result<Response<TransactionOutputListWithProofV2>>;

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
        request_timeout_ms: u64,
    ) -> error::Result<Response<TransactionListWithProofV2>>;

    /// Fetches a transaction or output list with proof, with data from
    /// start to end versions (inclusive). The proof is relative to the
    /// specified `proof_version`. If `include_events` is true, events are
    /// included in the proof. In some cases, fewer data items may be returned
    /// (e.g., to tolerate network or chunk limits). If the data cannot
    /// be fetched, an error is returned.
    async fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> error::Result<Response<TransactionOrOutputListWithProofV2>>;

    /// Subscribes to new transaction output lists with proofs. Subscriptions
    /// start at `known_version + 1` and `known_epoch` (inclusive), as
    /// specified by the stream metadata. The end version and proof version
    /// are specified by the server. If the data cannot be fetched, an
    /// error is returned.
    async fn subscribe_to_transaction_outputs_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>;

    /// Subscribes to new transaction lists with proofs. Subscriptions start
    /// at `known_version + 1` and `known_epoch` (inclusive), as specified
    /// by the subscription metadata. If `include_events` is true,
    /// events are included in the proof. The end version and proof version
    /// are specified by the server. If the data cannot be fetched, an error
    /// is returned.
    async fn subscribe_to_transactions_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>;

    /// Subscribes to new transaction or output lists with proofs. Subscriptions
    /// start at `known_version + 1` and `known_epoch` (inclusive), as
    /// specified by the subscription metadata. If `include_events` is true,
    /// events are included in the proof. The end version and proof version
    /// are specified by the server. If the data cannot be fetched, an error
    /// is returned.
    async fn subscribe_to_transactions_or_outputs_with_proof(
        &self,
        subscription_request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> error::Result<Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>>;
}

/// Subscription stream metadata associated with each subscription request
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SubscriptionRequestMetadata {
    pub known_version_at_stream_start: u64, // The highest known transaction version at stream start
    pub known_epoch_at_stream_start: u64,   // The highest known epoch at stream start
    pub subscription_stream_id: u64,        // The unique id of the subscription stream
    pub subscription_stream_index: u64,     // The index of the request in the subscription stream
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

/// A unique identifier for each response
pub type ResponseId = u64;

#[derive(Debug)]
pub struct ResponseContext {
    /// The time at which this response context was created
    pub creation_time: Instant,
    /// A unique identifier for this request/response pair. Intended mostly for
    /// debugging.
    pub id: ResponseId,
    /// A callback for notifying the data-client source about an error with this
    /// response.
    pub response_callback: Box<dyn ResponseCallback>,
}

impl ResponseContext {
    pub fn new(id: ResponseId, response_callback: Box<dyn ResponseCallback>) -> Self {
        Self {
            creation_time: Instant::now(),
            id,
            response_callback,
        }
    }
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// Returns a label for the response payload. This is useful
    /// for logging and metrics.
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

    /// Returns the chunk size of the response payload (i.e., the
    /// number of data items held in the response).
    pub fn get_data_chunk_size(&self) -> usize {
        match self {
            Self::EpochEndingLedgerInfos(epoch_ending_ledger_infos) => {
                epoch_ending_ledger_infos.len()
            },
            Self::NewTransactionOutputsWithProof((outputs_with_proof, _)) => {
                outputs_with_proof.get_num_outputs()
            },
            Self::NewTransactionsWithProof((transactions_with_proof, _)) => {
                transactions_with_proof.get_num_transactions()
            },
            Self::NumberOfStates(_) => {
                1 // The number of states is a single u64
            },
            Self::StateValuesWithProof(state_values_with_proof) => {
                state_values_with_proof.raw_values.len()
            },
            Self::TransactionOutputsWithProof(outputs_with_proof) => {
                outputs_with_proof.get_num_outputs()
            },
            Self::TransactionsWithProof(transactions_with_proof) => {
                transactions_with_proof.get_num_transactions()
            },
        }
    }
}

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

impl TryFrom<(TransactionOrOutputListWithProof, LedgerInfoWithSignatures)> for ResponsePayload {
    type Error = Error;

    fn try_from(
        inner: (TransactionOrOutputListWithProof, LedgerInfoWithSignatures),
    ) -> error::Result<Self, Error> {
        let ((transaction_list, output_list), ledger_info) = inner;
        if let Some(transaction_list) = transaction_list {
            Ok(Self::NewTransactionsWithProof((
                transaction_list,
                ledger_info,
            )))
        } else if let Some(output_list) = output_list {
            Ok(Self::NewTransactionOutputsWithProof((
                output_list,
                ledger_info,
            )))
        } else {
            Err(Error::InvalidResponse(
                "Invalid response! No transaction or output list was returned!".into(),
            ))
        }
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

impl TryFrom<TransactionOrOutputListWithProof> for ResponsePayload {
    type Error = Error;

    fn try_from(inner: TransactionOrOutputListWithProof) -> error::Result<Self, Error> {
        let (transaction_list, output_list) = inner;
        if let Some(transaction_list) = transaction_list {
            Ok(Self::TransactionsWithProof(transaction_list))
        } else if let Some(output_list) = output_list {
            Ok(Self::TransactionOutputsWithProof(output_list))
        } else {
            Err(Error::InvalidResponse(
                "Invalid response! No transaction or output list was returned!".into(),
            ))
        }
    }
}
