// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, StateValuesWithProofRequest,
        TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{CompleteDataRange, DataSummary, ProtocolMetadata},
    Epoch, StorageServiceRequest,
};
use aptos_crypto::hash::HashValue;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::Version,
};
use claims::{assert_err, assert_ok};
use proptest::{arbitrary::any, prelude::*};

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

    for compression in [true, false] {
        // in range, can service
        assert!(summary.can_service(&epochs_request(100, 200, compression)));
        assert!(summary.can_service(&epochs_request(125, 175, compression)));
        assert!(summary.can_service(&epochs_request(100, 100, compression)));
        assert!(summary.can_service(&epochs_request(150, 150, compression)));
        assert!(summary.can_service(&epochs_request(200, 200, compression)));

        // out of range, can't service
        assert!(!summary.can_service(&epochs_request(99, 200, compression)));
        assert!(!summary.can_service(&epochs_request(100, 201, compression)));
        assert!(!summary.can_service(&epochs_request(50, 250, compression)));
        assert!(!summary.can_service(&epochs_request(50, 150, compression)));
        assert!(!summary.can_service(&epochs_request(150, 250, compression)));

        // degenerate range, can't service
        assert!(!summary.can_service(&epochs_request(150, 149, compression)));
    }
}

#[test]
fn test_data_summary_can_service_txns_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        transactions: Some(create_range(100, 200)),
        ..Default::default()
    };

    for compression in [true, false] {
        // in range, can service
        assert!(summary.can_service(&txns_request(225, 100, 200, compression)));
        assert!(summary.can_service(&txns_request(225, 125, 175, compression)));
        assert!(summary.can_service(&txns_request(225, 100, 100, compression)));
        assert!(summary.can_service(&txns_request(225, 150, 150, compression)));
        assert!(summary.can_service(&txns_request(225, 200, 200, compression)));
        assert!(summary.can_service(&txns_request(250, 200, 200, compression)));

        // out of range, can't service
        assert!(!summary.can_service(&txns_request(225, 99, 200, compression)));
        assert!(!summary.can_service(&txns_request(225, 100, 201, compression)));
        assert!(!summary.can_service(&txns_request(225, 50, 250, compression)));
        assert!(!summary.can_service(&txns_request(225, 50, 150, compression)));
        assert!(!summary.can_service(&txns_request(225, 150, 250, compression)));

        assert!(!summary.can_service(&txns_request(300, 100, 200, compression)));
        assert!(!summary.can_service(&txns_request(300, 125, 175, compression)));
        assert!(!summary.can_service(&txns_request(300, 100, 100, compression)));
        assert!(!summary.can_service(&txns_request(300, 150, 150, compression)));
        assert!(!summary.can_service(&txns_request(300, 200, 200, compression)));
        assert!(!summary.can_service(&txns_request(251, 200, 200, compression)));
    }
}

#[test]
fn test_data_summary_can_service_txn_outputs_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        transaction_outputs: Some(create_range(100, 200)),
        ..Default::default()
    };

    for compression in [true, false] {
        // in range and can provide proof => can service
        assert!(summary.can_service(&outputs_request(225, 100, 200, compression)));
        assert!(summary.can_service(&outputs_request(225, 125, 175, compression)));
        assert!(summary.can_service(&outputs_request(225, 100, 100, compression)));
        assert!(summary.can_service(&outputs_request(225, 150, 150, compression)));
        assert!(summary.can_service(&outputs_request(225, 200, 200, compression)));
        assert!(summary.can_service(&outputs_request(250, 200, 200, compression)));

        // can provide proof, but out of range => cannot service
        assert!(!summary.can_service(&outputs_request(225, 99, 200, compression)));
        assert!(!summary.can_service(&outputs_request(225, 100, 201, compression)));
        assert!(!summary.can_service(&outputs_request(225, 50, 250, compression)));
        assert!(!summary.can_service(&outputs_request(225, 50, 150, compression)));
        assert!(!summary.can_service(&outputs_request(225, 150, 250, compression)));

        // in range, but cannot provide proof => cannot service
        assert!(!summary.can_service(&outputs_request(300, 100, 200, compression)));
        assert!(!summary.can_service(&outputs_request(300, 125, 175, compression)));
        assert!(!summary.can_service(&outputs_request(300, 100, 100, compression)));
        assert!(!summary.can_service(&outputs_request(300, 150, 150, compression)));
        assert!(!summary.can_service(&outputs_request(300, 200, 200, compression)));
        assert!(!summary.can_service(&outputs_request(251, 200, 200, compression)));

        // invalid range
        assert!(!summary.can_service(&outputs_request(225, 175, 125, compression)));
    }
}

#[test]
fn test_data_summary_can_service_state_chunk_request() {
    let summary = DataSummary {
        synced_ledger_info: Some(create_mock_ledger_info(250)),
        states: Some(create_range(100, 300)),
        ..Default::default()
    };

    for compression in [true, false] {
        // in range and can provide proof => can service
        assert!(summary.can_service(&states_request(100, compression)));
        assert!(summary.can_service(&states_request(200, compression)));
        assert!(summary.can_service(&states_request(250, compression)));

        // in range, but cannot provide proof => cannot service
        assert!(!summary.can_service(&states_request(251, compression)));
        assert!(!summary.can_service(&states_request(300, compression)));

        // can provide proof, but out of range ==> cannot service
        assert!(!summary.can_service(&states_request(50, compression)));
        assert!(!summary.can_service(&states_request(99, compression)));
    }
}

#[test]
fn test_protocol_metadata_can_service() {
    let metadata = ProtocolMetadata {
        max_transaction_chunk_size: 100,
        max_epoch_chunk_size: 100,
        max_transaction_output_chunk_size: 100,
        max_state_chunk_size: 100,
    };

    for compression in [true, false] {
        assert!(metadata.can_service(&txns_request(200, 100, 199, compression)));
        assert!(!metadata.can_service(&txns_request(200, 100, 200, compression)));

        assert!(metadata.can_service(&epochs_request(100, 199, compression)));
        assert!(!metadata.can_service(&epochs_request(100, 200, compression)));

        assert!(metadata.can_service(&outputs_request(200, 100, 199, compression)));
        assert!(!metadata.can_service(&outputs_request(200, 100, 200, compression)));

        assert!(metadata.can_service(&state_values_request(200, 100, 199, compression)));
        assert!(!metadata.can_service(&state_values_request(200, 100, 200, compression)));
    }
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
        AggregateSignature::empty(),
    )
}

fn create_range(lowest: u64, highest: u64) -> CompleteDataRange<u64> {
    CompleteDataRange::new(lowest, highest).unwrap()
}

fn epochs_request(start: Epoch, end: Epoch, use_compression: bool) -> StorageServiceRequest {
    let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch: start,
        expected_end_epoch: end,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

fn txns_request(
    proof: Version,
    start: Version,
    end: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version: proof,
        start_version: start,
        end_version: end,
        include_events: true,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

fn outputs_request(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version,
            start_version,
            end_version,
        });
    StorageServiceRequest::new(data_request, use_compression)
}

fn state_values_request(
    version: Version,
    start_index: u64,
    end_index: u64,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

fn states_request(version: Version, use_compression: bool) -> StorageServiceRequest {
    state_values_request(version, 0, 1000, use_compression)
}
