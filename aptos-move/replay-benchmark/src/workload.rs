// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, Transaction, TransactionBlock, Version,
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

/// Returns whether `txn` is a signed user transaction, the only kind of
/// transaction MonoMove executes. Written as an exhaustive match so that adding
/// a new [`Transaction`] variant is a compile error here, forcing an explicit
/// decision on whether to keep or drop it.
fn is_user_transaction(txn: &Transaction) -> bool {
    match txn {
        Transaction::UserTransaction(_) => true,
        Transaction::GenesisTransaction(_)
        | Transaction::BlockMetadata(_)
        | Transaction::BlockMetadataExt(_)
        | Transaction::StateCheckpoint(_)
        | Transaction::ValidatorTransaction(_)
        | Transaction::BlockEpilogue(_) => false,
    }
}

/// Drops every non-user transaction (block metadata, state checkpoint, block
/// epilogue, validator, and genesis transactions) from each block, keeping only
/// signed user transactions. The aligned `persisted_auxiliary_infos` are
/// filtered in lockstep, and blocks left empty are removed.
///
/// The captured read-set and the replayed workload must be filtered the same
/// way, so `--user-txns-only` has to be passed consistently to `initialize`,
/// `benchmark`, and `diff`. `begin_version` is left unchanged: the user
/// transactions replay on top of the pre-block state, so the block prologue's
/// timestamp is not advanced. This is acceptable for throughput measurement, but
/// means outputs for timestamp-dependent transactions can differ from on-chain.
pub(crate) fn retain_user_transactions_only(
    txn_blocks: Vec<TransactionBlock>,
) -> Vec<TransactionBlock> {
    txn_blocks
        .into_iter()
        .filter_map(|txn_block| {
            let TransactionBlock {
                begin_version,
                transactions,
                persisted_auxiliary_infos,
            } = txn_block;

            let keep = transactions
                .iter()
                .map(is_user_transaction)
                .collect::<Vec<_>>();
            let has_aux = !persisted_auxiliary_infos.is_empty();

            let transactions = transactions
                .into_iter()
                .zip(&keep)
                .filter_map(|(txn, &keep)| keep.then_some(txn))
                .collect::<Vec<_>>();
            let persisted_auxiliary_infos = if has_aux {
                persisted_auxiliary_infos
                    .into_iter()
                    .zip(&keep)
                    .filter_map(|(info, &keep)| keep.then_some(info))
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };

            (!transactions.is_empty()).then_some(TransactionBlock {
                begin_version,
                transactions,
                persisted_auxiliary_infos,
            })
        })
        .collect()
}
