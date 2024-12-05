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
    /// Stores a block of transactions for execution. Always has at least one transaction.
    txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction>,
    /// Stores metadata for the version range of a block. It is always set to
    /// [TransactionSliceMetadata::Chunk].
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
}
