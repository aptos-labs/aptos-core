// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{ensure, Result};
use aptos_executor_types::LedgerUpdateOutput;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{accumulator::InMemoryTransactionAccumulator, TransactionInfoListWithProof},
    transaction::{TransactionInfo, Version},
};
use itertools::zip_eq;
use std::{borrow::Cow, ops::Range, sync::Arc};

pub trait ChunkResultVerifier {
    /// Verifies `transaction_infos`, the computed results of `self.transaction_infos()[range]`.
    /// `parent_accumulator` is the local ledger right before `range`.
    fn verify_chunk_result(
        &self,
        range: Range<usize>,
        parent_accumulator: &InMemoryTransactionAccumulator,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()>;

    fn transaction_infos(&self) -> &[TransactionInfo];

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>>;
}

pub struct StateSyncChunkVerifier {
    pub txn_infos_with_proof: TransactionInfoListWithProof,
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
}

impl ChunkResultVerifier for StateSyncChunkVerifier {
    fn verify_chunk_result(
        &self,
        range: Range<usize>,
        parent_accumulator: &InMemoryTransactionAccumulator,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()> {
        // In consensus-only mode, we cannot verify the proof against the executed output,
        // because the proof returned by the remote peer is an empty one.
        if cfg!(feature = "consensus-only-perf-test") {
            return Ok(());
        }

        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            let first_version = parent_accumulator.num_leaves();

            // Verify the chunk extends the parent accumulator. The proof is relative to the
            // beginning of the chunk, and later ranges continue from a range verified here.
            if range.start == 0 {
                let parent_root_hash = parent_accumulator.root_hash();
                let num_overlap = self.txn_infos_with_proof.verify_extends_ledger(
                    first_version,
                    parent_root_hash,
                    Some(first_version),
                )?;
                assert_eq!(num_overlap, 0, "overlapped chunks");
            }

            // Verify transaction infos match
            ensure_transaction_infos_match(
                first_version,
                transaction_infos,
                &self.txn_infos_with_proof.transaction_infos[range],
            )
        })
    }

    fn transaction_infos(&self) -> &[TransactionInfo] {
        &self.txn_infos_with_proof.transaction_infos
    }

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        let li = self.verified_target_li.ledger_info();
        let txn_accumulator = &ledger_update_output.transaction_accumulator;

        if li.version() + 1 == txn_accumulator.num_leaves() {
            // If the chunk corresponds to the target LI, the target LI can be added to storage.
            ensure!(
                li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash in target ledger info does not match local computation. {:?} != {:?}",
                li,
                txn_accumulator,
            );
            Ok(Some(self.verified_target_li.clone()))
        } else if let Some(epoch_change_li) = &self.epoch_change_li {
            // If the epoch change LI is present, it must match the version of the chunk:
            let li = epoch_change_li.ledger_info();

            // Verify that the given ledger info corresponds to the new accumulator.
            ensure!(
                li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash of a given epoch LI does not match local computation. {:?} vs {:?}",
                li,
                txn_accumulator,
            );
            ensure!(
                li.version() + 1 == txn_accumulator.num_leaves(),
                "Version of a given epoch LI does not match local computation. {:?} vs {:?}",
                li,
                txn_accumulator,
            );
            ensure!(
                li.ends_epoch(),
                "Epoch change LI does not carry validator set. version:{}",
                li.version(),
            );
            ensure!(
                li.next_epoch_state() == next_epoch_state,
                "New validator set of a given epoch LI does not match local computation. {:?} vs {:?}",
                li.next_epoch_state(),
                next_epoch_state,
            );
            Ok(Some(epoch_change_li.clone()))
        } else {
            ensure!(
                next_epoch_state.is_none(),
                "End of epoch chunk based on local computation but no EoE LedgerInfo provided. version: {:?}",
                txn_accumulator.num_leaves().checked_sub(1),
            );
            Ok(None)
        }
    }
}

pub struct ReplayChunkVerifier {
    pub transaction_infos: Vec<TransactionInfo>,
}

impl ChunkResultVerifier for ReplayChunkVerifier {
    fn verify_chunk_result(
        &self,
        range: Range<usize>,
        parent_accumulator: &InMemoryTransactionAccumulator,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()> {
        ensure_transaction_infos_match(
            parent_accumulator.num_leaves(),
            transaction_infos,
            &self.transaction_infos[range],
        )
    }

    fn transaction_infos(&self) -> &[TransactionInfo] {
        &self.transaction_infos
    }

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        _ledger_update_output: &LedgerUpdateOutput,
        _next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        Ok(None)
    }
}

/// A contiguous range of transactions from one received chunk.
pub struct ChunkPiece {
    verifier: Arc<dyn ChunkResultVerifier + Send + Sync>,
    range: Range<usize>,
}

impl ChunkPiece {
    pub fn new(verifier: Arc<dyn ChunkResultVerifier + Send + Sync>, range: Range<usize>) -> Self {
        assert!(
            range.start <= range.end && range.end <= verifier.transaction_infos().len(),
            "Range {:?} is out of bounds for a chunk of {} transactions.",
            range,
            verifier.transaction_infos().len(),
        );
        Self { verifier, range }
    }

    pub fn whole_chunk(verifier: Arc<dyn ChunkResultVerifier + Send + Sync>) -> Self {
        let range = 0..verifier.transaction_infos().len();
        Self { verifier, range }
    }

    fn len(&self) -> usize {
        self.range.len()
    }

    fn transaction_infos(&self) -> &[TransactionInfo] {
        &self.verifier.transaction_infos()[self.range.clone()]
    }

    fn reaches_chunk_end(&self) -> bool {
        self.range.end == self.verifier.transaction_infos().len()
    }
}

/// Verifies the results of `ChunkPiece`s executed together, which can come from several chunks.
pub struct ChunkPiecesVerifier {
    pieces: Vec<ChunkPiece>,
}

impl ChunkPiecesVerifier {
    pub fn new(pieces: Vec<ChunkPiece>) -> Self {
        Self { pieces }
    }

    pub fn transaction_infos(&self) -> impl Iterator<Item = &TransactionInfo> {
        self.pieces.iter().flat_map(ChunkPiece::transaction_infos)
    }

    pub fn verify_chunk_result(
        &self,
        parent_accumulator: &InMemoryTransactionAccumulator,
        ledger_update_output: &LedgerUpdateOutput,
    ) -> Result<()> {
        let transaction_infos = &ledger_update_output.transaction_infos;
        let num_txns = self.pieces.iter().map(ChunkPiece::len).sum::<usize>();
        ensure!(
            transaction_infos.len() == num_txns,
            "Lengths don't match. {} vs {}",
            transaction_infos.len(),
            num_txns,
        );

        let mut accumulator = Cow::Borrowed(parent_accumulator);
        let mut start = 0;
        for (idx, piece) in self.pieces.iter().enumerate() {
            let end = start + piece.len();
            piece.verifier.verify_chunk_result(
                piece.range.clone(),
                &accumulator,
                &transaction_infos[start..end],
            )?;
            if idx + 1 < self.pieces.len() {
                accumulator = Cow::Owned(
                    accumulator.append(&ledger_update_output.transaction_info_hashes[start..end]),
                );
            }
            start = end;
        }
        Ok(())
    }

    pub fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        match self.pieces.last() {
            Some(piece) if piece.reaches_chunk_end() => piece
                .verifier
                .maybe_select_chunk_ending_ledger_info(ledger_update_output, next_epoch_state),
            // A chunk only carries ledger infos at its end.
            Some(_) | None => {
                ensure!(
                    next_epoch_state.is_none(),
                    "End of epoch based on local computation, but not at the end of a chunk. version: {:?}",
                    ledger_update_output
                        .transaction_accumulator
                        .num_leaves()
                        .checked_sub(1),
                );
                Ok(None)
            },
        }
    }
}

fn ensure_transaction_infos_match(
    first_version: Version,
    transaction_infos: &[TransactionInfo],
    expected_transaction_infos: &[TransactionInfo],
) -> Result<()> {
    ensure!(
        transaction_infos.len() == expected_transaction_infos.len(),
        "Lengths don't match. {} vs {}",
        transaction_infos.len(),
        expected_transaction_infos.len(),
    );

    for (version, (txn_info, expected_txn_info)) in (first_version..).zip(zip_eq(
        transaction_infos.iter(),
        expected_transaction_infos.iter(),
    )) {
        ensure!(
            txn_info == expected_txn_info,
            "Transaction infos don't match. version:{version}, txn_info:{txn_info}, expected_txn_info:{expected_txn_info}",
        );
    }
    Ok(())
}
