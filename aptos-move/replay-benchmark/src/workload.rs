// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_move_testing_utils::TransactionBlock;
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, Version,
    },
};

/// A workload to benchmark. Contains signature verified transactions, and metadata specifying the
/// start and end versions of these transactions.
pub(crate) struct Workload {
    /// Stores a non-empty block of  signature verified transactions ready for execution.
    pub(crate) txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
    /// Stores metadata for the version range of a block, corresponding to [begin, end) versions.
    /// It is always set to [TransactionSliceMetadata::Chunk].
    pub(crate) transaction_slice_metadata: TransactionSliceMetadata,
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
