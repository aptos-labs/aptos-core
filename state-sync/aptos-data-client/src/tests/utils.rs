// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_storage_service_types::responses::{
    CompleteDataRange, DataSummary, ProtocolMetadata, StorageServerSummary,
};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::Version,
};

/// Creates a test ledger info at the given version
fn create_ledger_info(version: Version) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None),
            HashValue::zero(),
        ),
        AggregateSignature::empty(),
    )
}

/// Creates a test storage server summary at the given version
pub fn create_storage_summary(version: Version) -> StorageServerSummary {
    StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 1000,
            max_state_chunk_size: 1000,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(create_ledger_info(version)),
            epoch_ending_ledger_infos: None,
            transactions: Some(CompleteDataRange::new(0, version).unwrap()),
            transaction_outputs: Some(CompleteDataRange::new(0, version).unwrap()),
            states: None,
        },
    }
}
