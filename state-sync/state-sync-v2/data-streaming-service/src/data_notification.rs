// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::streaming_client::Epoch;
use diem_crypto::HashValue;
use diem_types::{
    account_state_blob::AccountStatesChunkWithProof,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
        Version,
    },
};
use std::fmt::Debug;

/// A unique ID used to identify each notification.
pub type NotificationId = u64;

/// A single data notification with an ID and data payload.
#[derive(Debug)]
pub struct DataNotification {
    pub notification_id: NotificationId,
    pub data_payload: DataPayload,
}

/// A single payload (e.g. chunk) of requested data.
#[derive(Debug)]
pub enum DataPayload {
    AccountStatesWithProof(AccountStatesChunkWithProof),
    EpochEndingLedgerInfos(Vec<LedgerInfoWithSignatures>),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
}

/// A request that has been sent to the Diem data client.
#[derive(Clone, Debug)]
pub enum DataClientRequest {
    AccountsWithProof(AccountsWithProofRequest),
    EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest),
    TransactionsWithProof(TransactionsWithProofRequest),
    TransactionOutputsWithProof(TransactionOutputsWithProofRequest),
}

/// A request for fetching account states.
#[derive(Clone, Debug)]
pub struct AccountsWithProofRequest {
    pub version: Version,
    pub start_index: HashValue,
    pub end_index: HashValue,
}

/// A client request for fetching epoch ending ledger infos.
#[derive(Clone, Debug)]
pub struct EpochEndingLedgerInfosRequest {
    pub start_epoch: Epoch,
    pub end_epoch: Epoch,
}

/// A client request for fetching transactions with proofs.
#[derive(Clone, Debug)]
pub struct TransactionsWithProofRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub max_proof_version: Version,
    pub include_events: bool,
}

/// A client request for fetching transaction outputs with proofs.
#[derive(Clone, Debug)]
pub struct TransactionOutputsWithProofRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub max_proof_version: Version,
}
