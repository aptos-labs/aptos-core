// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::streaming_client::Epoch;
use aptos_data_client::{Response, ResponsePayload};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use std::fmt::{Debug, Formatter};

/// A unique ID used to identify each notification.
pub type NotificationId = u64;

/// A single data notification with an ID and data payload.
#[derive(Clone, Debug)]
pub struct DataNotification {
    pub notification_id: NotificationId,
    pub data_payload: DataPayload,
}

/// A single payload (e.g. chunk) of data delivered to a data listener.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum DataPayload {
    ContinuousTransactionOutputsWithProof(LedgerInfoWithSignatures, TransactionOutputListWithProof),
    ContinuousTransactionsWithProof(LedgerInfoWithSignatures, TransactionListWithProof),
    EpochEndingLedgerInfos(Vec<LedgerInfoWithSignatures>),
    EndOfStream,
    StateValuesWithProof(StateValueChunkWithProof),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

/// A request that has been sent to the Aptos data client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataClientRequest {
    EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest),
    NewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest),
    NewTransactionsWithProof(NewTransactionsWithProofRequest),
    NumberOfStates(NumberOfStatesRequest),
    StateValuesWithProof(StateValuesWithProofRequest),
    TransactionsWithProof(TransactionsWithProofRequest),
    TransactionOutputsWithProof(TransactionOutputsWithProofRequest),
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
        }
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

/// A pending client response where data has been requested from the
/// network and will be available in `client_response` when received.
pub struct PendingClientResponse {
    pub client_request: DataClientRequest,
    pub client_response: Option<Result<Response<ResponsePayload>, aptos_data_client::Error>>,
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
