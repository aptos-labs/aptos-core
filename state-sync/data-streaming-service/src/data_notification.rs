// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::streaming_client::Epoch;
use velor_data_client::interface::{Response, ResponsePayload};
use velor_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProofV2, TransactionOutputListWithProofV2, Version},
};
use std::{
    fmt::{Debug, Formatter},
    time::Instant,
};

/// A unique ID used to identify each notification.
pub type NotificationId = u64;

/// A single data notification with an ID and data payload.
#[derive(Clone, Debug)]
pub struct DataNotification {
    pub creation_time: Instant,
    pub notification_id: NotificationId,
    pub data_payload: DataPayload,
}

impl DataNotification {
    pub fn new(notification_id: NotificationId, data_payload: DataPayload) -> Self {
        Self {
            creation_time: Instant::now(),
            notification_id,
            data_payload,
        }
    }
}

/// A single payload (e.g. chunk) of data delivered to a data listener.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataPayload {
    ContinuousTransactionOutputsWithProof(
        LedgerInfoWithSignatures,
        TransactionOutputListWithProofV2,
    ),
    ContinuousTransactionsWithProof(LedgerInfoWithSignatures, TransactionListWithProofV2),
    EpochEndingLedgerInfos(Vec<LedgerInfoWithSignatures>),
    EndOfStream,
    StateValuesWithProof(StateValueChunkWithProof),
    TransactionOutputsWithProof(TransactionOutputListWithProofV2),
    TransactionsWithProof(TransactionListWithProofV2),
}

/// A request that has been sent to the Velor data client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataClientRequest {
    EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest),
    NewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest),
    NewTransactionsWithProof(NewTransactionsWithProofRequest),
    NumberOfStates(NumberOfStatesRequest),
    StateValuesWithProof(StateValuesWithProofRequest),
    TransactionsWithProof(TransactionsWithProofRequest),
    TransactionOutputsWithProof(TransactionOutputsWithProofRequest),
    NewTransactionsOrOutputsWithProof(NewTransactionsOrOutputsWithProofRequest),
    TransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest),
    SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest),
    SubscribeTransactionOutputsWithProof(SubscribeTransactionOutputsWithProofRequest),
    SubscribeTransactionsOrOutputsWithProof(SubscribeTransactionsOrOutputsWithProofRequest),
}

impl DataClientRequest {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::EpochEndingLedgerInfos(_) => "epoch_ending_ledger_infos",
            Self::NewTransactionOutputsWithProof(_) => "new_transaction_outputs_with_proof",
            Self::NewTransactionsWithProof(_) => "new_transactions_with_proof",
            Self::NumberOfStates(_) => "number_of_states",
            Self::StateValuesWithProof(_) => "state_values_with_proof",
            Self::TransactionsWithProof(_) => "transactions_with_proof",
            Self::TransactionOutputsWithProof(_) => "transaction_outputs_with_proof",
            Self::NewTransactionsOrOutputsWithProof(_) => "new_transactions_or_outputs_with_proof",
            Self::TransactionsOrOutputsWithProof(_) => "transactions_or_outputs_with_proof",
            Self::SubscribeTransactionsWithProof(_) => "subscribe_transactions_with_proof",
            Self::SubscribeTransactionOutputsWithProof(_) => {
                "subscribe_transaction_outputs_with_proof"
            },
            Self::SubscribeTransactionsOrOutputsWithProof(_) => {
                "subscribe_transactions_or_outputs_with_proof"
            },
        }
    }

    /// Returns true iff the request is a new data request
    pub fn is_new_data_request(&self) -> bool {
        self.is_optimistic_fetch_request() || self.is_subscription_request()
    }

    /// Returns true iff the request is an optimistic fetch request
    pub fn is_optimistic_fetch_request(&self) -> bool {
        matches!(self, DataClientRequest::NewTransactionsWithProof(_))
            || matches!(self, DataClientRequest::NewTransactionOutputsWithProof(_))
            || matches!(
                self,
                DataClientRequest::NewTransactionsOrOutputsWithProof(_)
            )
    }

    /// Returns true iff the request is a subscription request
    pub fn is_subscription_request(&self) -> bool {
        matches!(self, DataClientRequest::SubscribeTransactionsWithProof(_))
            || matches!(
                self,
                DataClientRequest::SubscribeTransactionOutputsWithProof(_)
            )
            || matches!(
                self,
                DataClientRequest::SubscribeTransactionsOrOutputsWithProof(_)
            )
    }
}

/// A request for fetching states values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateValuesWithProofRequest {
    pub version: Version,
    pub start_index: u64,
    pub end_index: u64,
}

/// A client request for fetching epoch ending ledger infos.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpochEndingLedgerInfosRequest {
    pub start_epoch: Epoch,
    pub end_epoch: Epoch,
}

/// A client request for fetching new transactions with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewTransactionsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub include_events: bool,
}

/// A client request for fetching new transactions or outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewTransactionsOrOutputsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub include_events: bool,
}

/// A client request for fetching new transaction outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewTransactionOutputsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
}

/// A client request for fetching the number of states at a version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberOfStatesRequest {
    pub version: Version,
}

/// A client request for subscribing to transactions with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeTransactionsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub include_events: bool,
    pub subscription_stream_id: u64,
    pub subscription_stream_index: u64,
}

/// A client request for subscribing to transaction outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeTransactionOutputsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub subscription_stream_id: u64,
    pub subscription_stream_index: u64,
}

/// A client request for subscribing to transactions or outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeTransactionsOrOutputsWithProofRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub include_events: bool,
    pub subscription_stream_id: u64,
    pub subscription_stream_index: u64,
}

/// A client request for fetching transactions with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionsWithProofRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub proof_version: Version,
    pub include_events: bool,
}

/// A client request for fetching transaction outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionOutputsWithProofRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub proof_version: Version,
}

/// A client request for fetching transaction or outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionsOrOutputsWithProofRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub proof_version: Version,
    pub include_events: bool,
}

/// A pending client response where data has been requested from the
/// network and will be available in `client_response` when received.
pub struct PendingClientResponse {
    pub client_request: DataClientRequest,
    pub client_response: Option<Result<Response<ResponsePayload>, velor_data_client::error::Error>>,
}

impl PendingClientResponse {
    pub fn new(client_request: DataClientRequest) -> Self {
        Self {
            client_request,
            client_response: None,
        }
    }

    #[cfg(test)]
    /// Creates a new pending client response with a response already available
    pub fn new_with_response(
        client_request: DataClientRequest,
        client_response: Result<Response<ResponsePayload>, velor_data_client::error::Error>,
    ) -> Self {
        Self {
            client_request,
            client_response: Some(client_response),
        }
    }
}

impl Debug for PendingClientResponse {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Client request: {:?}, client response: {:?}",
            self.client_request, self.client_response
        )
    }
}
