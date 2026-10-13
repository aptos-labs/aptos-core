// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunk_executor::chunk_result_verifier::{
        ChunkPiece, ChunkPiecesVerifier, ChunkResultVerifier, StateSyncChunkVerifier,
    },
    tests::{create_blocks_and_chunks, TestExecutor},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::LedgerUpdateOutput;
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    proof::accumulator::InMemoryTransactionAccumulator, transaction::TransactionInfo,
};
use itertools::Itertools;
use std::sync::Arc;

struct TestChunks {
    genesis_accumulator: Arc<InMemoryTransactionAccumulator>,
    /// Versions [1, 6] and [7, 10] of a single block.
    chunks: Vec<Arc<StateSyncChunkVerifier>>,
    ledger_info: LedgerInfoWithSignatures,
}

impl TestChunks {
    fn new(epoch_change_li_for_first_chunk: bool) -> Self {
        let block = 1..=10;
        let (blocks, chunks) = create_blocks_and_chunks(vec![block], vec![1..=6, 7..=10]);
        let ledger_info = blocks.last().unwrap().2.clone();
        let chunks = chunks
            .into_iter()
            .enumerate()
            .map(|(idx, chunk)| {
                let (txn_list_with_proof, _) = chunk.into_parts();
                Arc::new(StateSyncChunkVerifier {
                    txn_infos_with_proof: txn_list_with_proof.proof,
                    verified_target_li: ledger_info.clone(),
                    epoch_change_li: (idx == 0 && epoch_change_li_for_first_chunk)
                        .then(|| ledger_info.clone()),
                })
            })
            .collect();
        let genesis_accumulator = TestExecutor::new()
            .db
            .reader
            .get_pre_committed_ledger_summary()
            .unwrap()
            .transaction_accumulator;

        Self {
            genesis_accumulator,
            chunks,
            ledger_info,
        }
    }

    fn piece(&self, chunk_idx: usize, range: std::ops::Range<usize>) -> ChunkPiece {
        ChunkPiece::new(Arc::clone(&self.chunks[chunk_idx]) as _, range)
    }

    fn txn_infos(&self, chunk_idx: usize, range: std::ops::Range<usize>) -> Vec<TransactionInfo> {
        self.chunks[chunk_idx].transaction_infos()[range].to_vec()
    }
}

/// Mimics executing transactions that result in `txn_infos`.
fn ledger_update_output(
    parent_accumulator: &Arc<InMemoryTransactionAccumulator>,
    txn_infos: Vec<TransactionInfo>,
) -> LedgerUpdateOutput {
    let hashes = txn_infos.iter().map(CryptoHash::hash).collect_vec();
    LedgerUpdateOutput::builder()
        .transaction_accumulator(Arc::new(parent_accumulator.append(&hashes)))
        .parent_accumulator(Arc::clone(parent_accumulator))
        .transaction_infos(txn_infos)
        .transaction_info_hashes(hashes)
        .build()
}

#[test]
fn test_verify_pieces_across_chunks() {
    let t = TestChunks::new(false);

    // Versions [1, 3], the head of the first chunk.
    let verifier = ChunkPiecesVerifier::new(vec![t.piece(0, 0..3)]);
    let output = ledger_update_output(&t.genesis_accumulator, t.txn_infos(0, 0..3));
    verifier
        .verify_chunk_result(&t.genesis_accumulator, &output)
        .unwrap();
    assert!(verifier
        .maybe_select_chunk_ending_ledger_info(&output, None)
        .unwrap()
        .is_none());

    // Versions [4, 10], the tail of the first chunk and the whole second chunk.
    let parent_accumulator = Arc::clone(output.txn_accumulator());
    let verifier = ChunkPiecesVerifier::new(vec![t.piece(0, 3..6), t.piece(1, 0..4)]);
    let txn_infos = [t.txn_infos(0, 3..6), t.txn_infos(1, 0..4)].concat();
    assert_eq!(
        verifier.transaction_infos().cloned().collect_vec(),
        txn_infos
    );
    let output = ledger_update_output(&parent_accumulator, txn_infos);
    verifier
        .verify_chunk_result(&parent_accumulator, &output)
        .unwrap();
    assert_eq!(
        verifier
            .maybe_select_chunk_ending_ledger_info(&output, None)
            .unwrap(),
        Some(t.ledger_info),
    );
}

#[test]
fn test_verify_pieces_rejects_mismatched_txn_info() {
    let t = TestChunks::new(false);

    let parent_accumulator = Arc::new(
        t.genesis_accumulator.append(
            &t.txn_infos(0, 0..3)
                .iter()
                .map(CryptoHash::hash)
                .collect_vec(),
        ),
    );
    let verifier = ChunkPiecesVerifier::new(vec![t.piece(0, 3..6), t.piece(1, 0..4)]);
    // The last transaction info is computed differently.
    let txn_infos = [
        t.txn_infos(0, 3..6),
        t.txn_infos(1, 0..3),
        t.txn_infos(0, 0..1),
    ]
    .concat();
    let output = ledger_update_output(&parent_accumulator, txn_infos);
    let err = verifier
        .verify_chunk_result(&parent_accumulator, &output)
        .unwrap_err();
    assert!(
        err.to_string().contains("Transaction infos don't match"),
        "{err}"
    );
}

#[test]
fn test_verify_pieces_rejects_forked_parent() {
    let t = TestChunks::new(false);

    // Same number of transactions as the real parent, but a different history.
    let parent_accumulator = Arc::new(t.genesis_accumulator.append(&[HashValue::random(); 3]));
    // Only the second chunk is verified against the ledger, since the first piece doesn't start
    // its chunk.
    let verifier = ChunkPiecesVerifier::new(vec![t.piece(0, 3..6), t.piece(1, 0..4)]);
    let txn_infos = [t.txn_infos(0, 3..6), t.txn_infos(1, 0..4)].concat();
    let output = ledger_update_output(&parent_accumulator, txn_infos);
    let err = verifier
        .verify_chunk_result(&parent_accumulator, &output)
        .unwrap_err();
    assert!(err.to_string().contains("Fork happens"), "{err}");
}

#[test]
fn test_no_ledger_info_in_the_middle_of_chunk() {
    // Pretend the first chunk ends an epoch, which would have its ledger info checked against
    // the chunk's last version.
    let t = TestChunks::new(true);

    let verifier = ChunkPiecesVerifier::new(vec![t.piece(0, 0..3)]);
    let output = ledger_update_output(&t.genesis_accumulator, t.txn_infos(0, 0..3));
    assert!(verifier
        .maybe_select_chunk_ending_ledger_info(&output, None)
        .unwrap()
        .is_none());
    // An epoch can't end in the middle of a chunk.
    assert!(verifier
        .maybe_select_chunk_ending_ledger_info(&output, Some(&EpochState::empty()))
        .is_err());
}
