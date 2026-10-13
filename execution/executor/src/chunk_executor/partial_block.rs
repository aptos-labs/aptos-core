// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::chunk_executor::{
    chunk_result_verifier::{ChunkPiece, ChunkPiecesVerifier, ChunkResultVerifier},
    transaction_chunk::{ChunkToExecuteOrApply, TransactionChunk},
};
use anyhow::{ensure, Result};
use aptos_types::transaction::TransactionInfo;
use std::sync::Arc;

/// Received transactions after the last state checkpoint, i.e. the beginning of a block whose
/// remaining transactions are yet to be received.
#[derive(Default)]
pub struct PartialBlock {
    /// `None` if there's no such transaction.
    transactions: Option<ChunkToExecuteOrApply>,
    pieces: Vec<ChunkPiece>,
}

impl PartialBlock {
    pub fn num_transactions(&self) -> usize {
        self.transactions.as_ref().map_or(0, TransactionChunk::len)
    }

    /// Adds a received chunk, and takes the transactions up to the last state checkpoint, if any,
    /// together with the verifier of their results.
    ///
    /// The taken transactions are applied only if all of them came with outputs, otherwise they
    /// are executed.
    pub fn add_chunk(
        &mut self,
        mut chunk: ChunkToExecuteOrApply,
        verifier: Arc<dyn ChunkResultVerifier + Send + Sync>,
    ) -> Result<Option<(ChunkToExecuteOrApply, ChunkPiecesVerifier)>> {
        let num_txns = chunk.len();
        ensure!(num_txns > 0, "Empty chunk.");
        ensure!(
            verifier.transaction_infos().len() == num_txns,
            "Chunk has {} transactions but {} transaction infos.",
            num_txns,
            verifier.transaction_infos().len(),
        );
        if let Some(pending) = &self.transactions {
            ensure!(
                chunk.first_version() == pending.next_version(),
                "Chunk carries unexpected first version. Expected: {}, got: {}",
                pending.next_version(),
                chunk.first_version(),
            );
        }

        // The proven transaction infos tell where blocks end before execution.
        let Some(last_checkpoint_idx) = verifier
            .transaction_infos()
            .iter()
            .rposition(TransactionInfo::has_state_checkpoint_hash)
        else {
            self.push(chunk, ChunkPiece::whole_chunk(verifier));
            return Ok(None);
        };

        let num_to_take = last_checkpoint_idx + 1;
        // Split before merging with the pending transactions, so that the held-back rest keeps the
        // mode of its own chunk.
        let rest = chunk.split_off(num_to_take);
        let to_take = match self.transactions.take() {
            Some(pending) => pending.append(chunk),
            None => chunk,
        };
        let mut pieces = std::mem::take(&mut self.pieces);
        pieces.push(ChunkPiece::new(Arc::clone(&verifier), 0..num_to_take));

        if num_to_take < num_txns {
            self.push(rest, ChunkPiece::new(verifier, num_to_take..num_txns));
        }
        Ok(Some((to_take, ChunkPiecesVerifier::new(pieces))))
    }

    fn push(&mut self, transactions: ChunkToExecuteOrApply, piece: ChunkPiece) {
        self.transactions = Some(match self.transactions.take() {
            Some(pending) => pending.append(transactions),
            None => transactions,
        });
        self.pieces.push(piece);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_executor::{
        chunk_result_verifier::ReplayChunkVerifier,
        transaction_chunk::{ChunkToApply, ChunkToExecute},
    };
    use aptos_crypto::{hash::CryptoHash, HashValue};
    use aptos_types::transaction::{
        ExecutionStatus, PersistedAuxiliaryInfo, Transaction, TransactionOutput, Version,
    };
    use itertools::Itertools;

    fn txn(version: Version) -> Transaction {
        Transaction::StateCheckpoint(HashValue::from_u64(version))
    }

    fn txn_info(version: Version, is_checkpoint: bool) -> TransactionInfo {
        TransactionInfo::builder_v0()
            .transaction_hash(txn(version).hash())
            .state_change_hash(HashValue::zero())
            .event_root_hash(HashValue::zero())
            .maybe_state_checkpoint_hash(is_checkpoint.then(HashValue::zero))
            .gas_used(0)
            .status(ExecutionStatus::Success)
            .build()
    }

    /// A received chunk, where `checkpoints` marks the transactions ending a block.
    fn received_chunk(
        first_version: Version,
        checkpoints: &[bool],
        with_outputs: bool,
    ) -> (
        ChunkToExecuteOrApply,
        Arc<dyn ChunkResultVerifier + Send + Sync>,
    ) {
        let versions = first_version..first_version + checkpoints.len() as Version;
        let transactions = versions.clone().map(txn).collect_vec();
        let persisted_aux_info = vec![PersistedAuxiliaryInfo::None; checkpoints.len()];
        let chunk = if with_outputs {
            ChunkToExecuteOrApply::Apply(ChunkToApply {
                transactions,
                transaction_outputs: vec![
                    TransactionOutput::new_empty_success();
                    checkpoints.len()
                ],
                persisted_aux_info,
                first_version,
            })
        } else {
            ChunkToExecuteOrApply::Execute(ChunkToExecute {
                transactions,
                persisted_aux_info,
                first_version,
            })
        };
        let transaction_infos = versions
            .zip(checkpoints)
            .map(|(version, is_checkpoint)| txn_info(version, *is_checkpoint))
            .collect();
        (chunk, Arc::new(ReplayChunkVerifier { transaction_infos }))
    }

    fn add(
        partial_block: &mut PartialBlock,
        first_version: Version,
        checkpoints: &[bool],
        with_outputs: bool,
    ) -> Option<(ChunkToExecuteOrApply, ChunkPiecesVerifier)> {
        let (chunk, verifier) = received_chunk(first_version, checkpoints, with_outputs);
        partial_block.add_chunk(chunk, verifier).unwrap()
    }

    /// Asserts the taken transactions are at `versions`, and their transaction infos line up.
    fn assert_taken(
        taken: Option<(ChunkToExecuteOrApply, ChunkPiecesVerifier)>,
        versions: std::ops::Range<Version>,
        expect_outputs: bool,
    ) {
        let (chunk, verifier) = taken.expect("Expected complete blocks.");
        let transactions = match &chunk {
            ChunkToExecuteOrApply::Execute(chunk) => {
                assert!(!expect_outputs);
                &chunk.transactions
            },
            ChunkToExecuteOrApply::Apply(chunk) => {
                assert!(expect_outputs);
                assert_eq!(chunk.transaction_outputs.len(), chunk.transactions.len());
                &chunk.transactions
            },
        };
        assert_eq!(chunk.first_version(), versions.start);
        assert_eq!(transactions, &versions.clone().map(txn).collect_vec());
        let txn_hashes = verifier
            .transaction_infos()
            .map(TransactionInfo::transaction_hash)
            .collect_vec();
        assert_eq!(
            txn_hashes,
            versions.clone().map(|v| txn(v).hash()).collect_vec()
        );
        // The taken transactions end a block.
        assert!(verifier
            .transaction_infos()
            .last()
            .unwrap()
            .has_state_checkpoint_hash());
    }

    #[test]
    fn test_chunk_ending_at_checkpoint() {
        let mut partial_block = PartialBlock::default();
        let taken = add(&mut partial_block, 1, &[false, true, false, true], true);
        assert_taken(taken, 1..5, true);
        assert_eq!(partial_block.num_transactions(), 0);
    }

    #[test]
    fn test_block_across_chunks() {
        let mut partial_block = PartialBlock::default();

        // Versions 3 and 4 are held back.
        let taken = add(&mut partial_block, 1, &[false, true, false, false], true);
        assert_taken(taken, 1..3, true);
        assert_eq!(partial_block.num_transactions(), 2);

        // No block ends in the chunk.
        assert!(add(&mut partial_block, 5, &[false, false], true).is_none());
        assert_eq!(partial_block.num_transactions(), 4);

        // The block ends at version 7, and version 8 is held back.
        let taken = add(&mut partial_block, 7, &[true, false], true);
        assert_taken(taken, 3..8, true);
        assert_eq!(partial_block.num_transactions(), 1);

        let taken = add(&mut partial_block, 9, &[true], true);
        assert_taken(taken, 8..10, true);
        assert_eq!(partial_block.num_transactions(), 0);
    }

    #[test]
    fn test_mixed_modes() {
        let mut partial_block = PartialBlock::default();

        // A block received as transactions, then outputs, is executed.
        assert!(add(&mut partial_block, 1, &[false, false], false).is_none());
        let taken = add(&mut partial_block, 3, &[true, false], true);
        assert_taken(taken, 1..4, false);

        // The held-back version 4 still has its output, so it's applied with the next block.
        let taken = add(&mut partial_block, 5, &[true], true);
        assert_taken(taken, 4..6, true);

        // A block received as outputs, then transactions, is executed.
        assert!(add(&mut partial_block, 6, &[false], true).is_none());
        let taken = add(&mut partial_block, 7, &[true], false);
        assert_taken(taken, 6..8, false);
    }

    #[test]
    fn test_reject_bad_chunk() {
        let mut partial_block = PartialBlock::default();
        assert!(add(&mut partial_block, 1, &[true, false], true).is_some());

        // Gap after the held-back version 2.
        let (chunk, verifier) = received_chunk(4, &[true], true);
        assert!(partial_block.add_chunk(chunk, verifier).is_err());

        // Mismatched number of transaction infos.
        let (chunk, _) = received_chunk(3, &[true], true);
        let (_, verifier) = received_chunk(3, &[false, true], true);
        assert!(partial_block.add_chunk(chunk, verifier).is_err());
    }
}
