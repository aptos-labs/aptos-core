// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::requests::{
    EpochEndingLedgerInfoRequest, StateValuesWithProofRequest, TransactionOutputsWithProofRequest,
    TransactionsWithProofRequest,
};
use crate::responses::{CompleteDataRange, DataSummary, ProtocolMetadata};
use crate::{Epoch, StorageServiceRequest};
use aptos_crypto::hash::HashValue;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use aptos_types::transaction::Version;
use aptos_types::{block_info::BlockInfo, ledger_info::LedgerInfo};
use claim::{assert_err, assert_ok};
use proptest::arbitrary::any;
use proptest::prelude::*;
use std::collections::BTreeMap;

#[test]
fn test_complete_data_range() {
    // good ranges
    assert_ok!(CompleteDataRange::new(0, 0));
    assert_ok!(CompleteDataRange::new(10, 10));
    assert_ok!(CompleteDataRange::new(10, 20));
    assert_ok!(CompleteDataRange::new(u64::MAX, u64::MAX));

    // degenerate ranges
    assert_err!(CompleteDataRange::new(1, 0));
    assert_err!(CompleteDataRange::new(20, 10));
    assert_err!(CompleteDataRange::new(u64::MAX, 0));
    assert_err!(CompleteDataRange::new(u64::MAX, 1));

    // range length overflow edge case
    assert_ok!(CompleteDataRange::new(1, u64::MAX));
    assert_ok!(CompleteDataRange::new(0, u64::MAX - 1));
    assert_err!(CompleteDataRange::new(0, u64::MAX));
}

#[test]
fn test_data_summary_can_service_epochs_request() {
    let summary = DataSummary {
        epoch_ending_ledger_infos: Some(create_range(100, 200)),
        ..Default::default()
    };

    // in range, can service

    assert!(summary.can_service(&create_get_epochs_request(100, 200)));
    assert!(summary.can_service(&create_get_epochs_request(125, 175)));
    assert!(summary.can_service(&create_get_epochs_request(100, 100)));
    assert!(summary.can_service(&create_get_epochs_request(150, 150)));
    assert!(summary.can_service(&create_get_epochs_request(200, 200)));

    // out of range, can't service

    assert!(!summary.can_service(&create_get_epochs_request(99, 200)));
    assert!(!summary.can_service(&create_get_epochs_request(100, 201)));
    assert!(!summary.can_service(&create_get_epochs_request(50, 250)));
    assert!(!summary.can_service(&create_get_epochs_request(50, 150)));
    assert!(!summary.can_service(&create_get_epochs_request(150, 250)));

    // degenerate range, can't service

    assert!(!summary.can_service(&create_get_epochs_request(150, 149)));
}

#[test]
fn test_data_summary_can_service_txns_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        transactions: Some(create_range(100, 200)),
        ..Default::default()
    };

    // in range, can service

    assert!(summary.can_service(&create_get_txns_request(225, 100, 200)));
    assert!(summary.can_service(&create_get_txns_request(225, 125, 175)));
    assert!(summary.can_service(&create_get_txns_request(225, 100, 100)));
    assert!(summary.can_service(&create_get_txns_request(225, 150, 150)));
    assert!(summary.can_service(&create_get_txns_request(225, 200, 200)));
    assert!(summary.can_service(&create_get_txns_request(250, 200, 200)));

    // out of range, can't service

    assert!(!summary.can_service(&create_get_txns_request(225, 99, 200)));
    assert!(!summary.can_service(&create_get_txns_request(225, 100, 201)));
    assert!(!summary.can_service(&create_get_txns_request(225, 50, 250)));
    assert!(!summary.can_service(&create_get_txns_request(225, 50, 150)));
    assert!(!summary.can_service(&create_get_txns_request(225, 150, 250)));

    assert!(!summary.can_service(&create_get_txns_request(300, 100, 200)));
    assert!(!summary.can_service(&create_get_txns_request(300, 125, 175)));
    assert!(!summary.can_service(&create_get_txns_request(300, 100, 100)));
    assert!(!summary.can_service(&create_get_txns_request(300, 150, 150)));
    assert!(!summary.can_service(&create_get_txns_request(300, 200, 200)));
    assert!(!summary.can_service(&create_get_txns_request(251, 200, 200)));
}

#[test]
fn test_data_summary_can_service_txn_outputs_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        transaction_outputs: Some(create_range(100, 200)),
        ..Default::default()
    };

    // in range and can provide proof => can service
    assert!(summary.can_service(&create_get_txn_outputs_request(225, 100, 200)));
    assert!(summary.can_service(&create_get_txn_outputs_request(225, 125, 175)));
    assert!(summary.can_service(&create_get_txn_outputs_request(225, 100, 100)));
    assert!(summary.can_service(&create_get_txn_outputs_request(225, 150, 150)));
    assert!(summary.can_service(&create_get_txn_outputs_request(225, 200, 200)));
    assert!(summary.can_service(&create_get_txn_outputs_request(250, 200, 200)));

    // can provide proof, but out of range => cannot service
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 99, 200)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 100, 201)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 50, 250)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 50, 150)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 150, 250)));

    // in range, but cannot provide proof => cannot service
    assert!(!summary.can_service(&create_get_txn_outputs_request(300, 100, 200)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(300, 125, 175)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(300, 100, 100)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(300, 150, 150)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(300, 200, 200)));
    assert!(!summary.can_service(&create_get_txn_outputs_request(251, 200, 200)));

    // invalid range
    assert!(!summary.can_service(&create_get_txn_outputs_request(225, 175, 125)));
}

#[test]
fn test_data_summary_can_service_state_chunk_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        states: Some(create_range(100, 300)),
        ..Default::default()
    };

    // in range and can provide proof => can service
    assert!(summary.can_service(&create_get_states_request(100)));
    assert!(summary.can_service(&create_get_states_request(200)));
    assert!(summary.can_service(&create_get_states_request(250)));

    // in range, but cannot provide proof => cannot service
    assert!(!summary.can_service(&create_get_states_request(251)));
    assert!(!summary.can_service(&create_get_states_request(300)));

    // can provide proof, but out of range ==> cannot service
    assert!(!summary.can_service(&create_get_states_request(50)));
    assert!(!summary.can_service(&create_get_states_request(99)));
}

#[test]
fn test_protocol_metadata_can_service() {
    let metadata = ProtocolMetadata {
        max_transaction_chunk_size: 100,
        max_epoch_chunk_size: 100,
        max_transaction_output_chunk_size: 100,
        max_state_chunk_size: 100,
    };

    assert!(metadata.can_service(&create_get_txns_request(200, 100, 199)));
    assert!(!metadata.can_service(&create_get_txns_request(200, 100, 200)));

    assert!(metadata.can_service(&create_get_epochs_request(100, 199)));
    assert!(!metadata.can_service(&create_get_epochs_request(100, 200)));

    assert!(metadata.can_service(&create_get_txn_outputs_request(200, 100, 199)));
    assert!(!metadata.can_service(&create_get_txn_outputs_request(200, 100, 200)));

    assert!(metadata.can_service(&create_get_state_values_request(200, 100, 199)));
    assert!(!metadata.can_service(&create_get_state_values_request(200, 100, 200)));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_data_summary_length_invariant(range in any::<CompleteDataRange<u64>>()) {
        // should not panic
        let _ = range.len();
    }
}

fn create_mock_ledger_info(version: Version) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None),
            HashValue::zero(),
        ),
        BTreeMap::new(),
    )
}

fn create_range(lowest: u64, highest: u64) -> CompleteDataRange<u64> {
    CompleteDataRange::new(lowest, highest).unwrap()
}

fn create_get_epochs_request(start: Epoch, end: Epoch) -> StorageServiceRequest {
    StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch: start,
        expected_end_epoch: end,
    })
}

fn create_get_txns_request(proof: Version, start: Version, end: Version) -> StorageServiceRequest {
    StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version: proof,
        start_version: start,
        end_version: end,
        include_events: true,
    })
}

fn create_get_txn_outputs_request(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
) -> StorageServiceRequest {
    StorageServiceRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
        proof_version,
        start_version,
        end_version,
    })
}

fn create_get_state_values_request(
    version: Version,
    start_index: u64,
    end_index: u64,
) -> StorageServiceRequest {
    StorageServiceRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    })
}

fn create_get_states_request(version: Version) -> StorageServiceRequest {
    create_get_state_values_request(version, 0, 1000)
}
