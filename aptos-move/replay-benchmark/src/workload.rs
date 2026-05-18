// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, PersistedAuxiliaryInfo, Transaction, Version,
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
    /// Persisted auxiliary info for each transaction, aligned with `transactions`.
    #[serde(default = "Vec::new")]
    pub(crate) persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

impl From<TransactionBlock> for Workload {
    fn from(txn_block: TransactionBlock) -> Self {
        assert!(!txn_block.transactions.is_empty());

        let end = txn_block.begin_version + txn_block.transactions.len() as Version;
        let transaction_slice_metadata =
            TransactionSliceMetadata::chunk(txn_block.begin_version, end);

        let signature_verified_txns = into_signature_verified_block(txn_block.transactions);
        let txn_provider = if txn_block.persisted_auxiliary_infos.is_empty() {
            DefaultTxnProvider::new_without_info(signature_verified_txns)
        } else {
            let auxiliary_infos = txn_block
                .persisted_auxiliary_infos
                .into_iter()
                .map(|persisted_info| AuxiliaryInfo::new(persisted_info, None))
                .collect::<Vec<_>>();
            DefaultTxnProvider::new(signature_verified_txns, auxiliary_infos)
        };

        Self {
            txn_provider,
            transaction_slice_metadata,
        }
    }
}
