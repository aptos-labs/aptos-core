// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, Transaction, Version,
    },
};
use serde::{Deserialize, Serialize};

/// A workload to benchmark. Contains signature verified transactions, and metadata specifying the
/// start and end versions of these transactions.
pub(crate) struct Workload {
    /// Stores a non-empty block of  signature verified transactions ready for execution.
    pub(crate) txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
    /// Stores metadata for the version range of a block, corresponding to [begin, end) versions.
    /// It is always set to [TransactionSliceMetadata::Chunk].
    pub(crate) transaction_slice_metadata: TransactionSliceMetadata,
}

/// On-disk representation of a workload, saved to the local filesystem.
#[derive(Serialize, Deserialize)]
pub(crate) struct TransactionBlock {
    /// The version of the first transaction in the block.
    pub(crate) begin_version: Version,
    /// Non-empty list of transactions in a block.
    pub(crate) transactions: Vec<Transaction>,
}

impl From<TransactionBlock> for Workload {
    fn from(txn_block: TransactionBlock) -> Self {
        assert!(!txn_block.transactions.is_empty());

        let end = txn_block.begin_version + txn_block.transactions.len() as Version;
        let transaction_slice_metadata =
            TransactionSliceMetadata::chunk(txn_block.begin_version, end);

        let signature_verified_txns = into_signature_verified_block(txn_block.transactions);
        let txn_provider = DefaultTxnProvider::new_without_info(signature_verified_txns);

        Self {
            txn_provider,
            transaction_slice_metadata,
        }
    }
}
