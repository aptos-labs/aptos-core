// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        Transaction, Version,
    },
};

/// A workload to benchmark. Contains signature verified transactions, and metadata specifying the
/// start and end versions of these transactions.
pub(crate) struct Workload {
    txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction>,
    transaction_slice_metadata: TransactionSliceMetadata,
}

impl Workload {
    /// Returns a new workload to execute transactions at specified version.
    pub(crate) fn new(begin: Version, txns: Vec<Transaction>) -> Self {
        assert!(!txns.is_empty());

        let end = begin + txns.len() as Version;
        let transaction_slice_metadata = TransactionSliceMetadata::chunk(begin, end);

        let signature_verified_txns = into_signature_verified_block(txns);
        let txn_provider = DefaultTxnProvider::new(signature_verified_txns);

        Workload {
            txn_provider,
            transaction_slice_metadata,
        }
    }

    /// Returns the signature verified transactions in the workload.
    pub(crate) fn txn_provider(&self) -> &DefaultTxnProvider<SignatureVerifiedTransaction> {
        &self.txn_provider
    }

    /// Returns transaction metadata corresponding to [begin, end) versions of the workload.
    pub(crate) fn transaction_slice_metadata(&self) -> TransactionSliceMetadata {
        self.transaction_slice_metadata
    }

    /// Returns the first transaction version in the workload.
    pub(crate) fn first_version(&self) -> Version {
        match &self.transaction_slice_metadata {
            TransactionSliceMetadata::Chunk { begin, .. } => *begin,
            _ => unreachable!("Transaction slice metadata is always a chunk"),
        }
    }

    /// Returns the last transaction version in the workload.
    #[allow(dead_code)]
    pub(crate) fn last_version(&self) -> Version {
        match &self.transaction_slice_metadata {
            TransactionSliceMetadata::Chunk { end, .. } => *end - 1,
            _ => unreachable!("Transaction slice metadata is always a chunk"),
        }
    }
}
