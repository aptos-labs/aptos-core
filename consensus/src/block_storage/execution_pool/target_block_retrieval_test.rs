// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block_storage::{
    block_store::sync_manager::TargetBlockRetrieval,
    execution_pool::common::create_block_tree_with_forks_unordered_parents, BlockReader,
    BlockStore,
};

#[tokio::test]
async fn test_no_window_quorum_round_greater_than_commit_round() {
    let window_size: Option<u64> = None;
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, _a1_r1, _a2_r3, _a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    let commit_root = block_store.commit_root().id();
    assert_eq!(commit_root, genesis_block.id());

    // Use a4_r9 as an example of a quorum cert
    let qc = a4_r9.quorum_cert().clone();
    let qc_round = qc.certified_block().round();
    let commit = qc.into_wrapped_ledger_info();
    let commit_round = commit.commit_info().round();

    assert_eq!(qc_round, 6);
    assert_eq!(commit_round, 0);

    let (payload, num_blocks) = BlockStore::generate_target_block_retrieval_payload_and_num_blocks(
        &qc,
        &commit,
        window_size,
    );

    match payload {
        TargetBlockRetrieval::TargetBlockId(id) => {
            assert_eq!(id, genesis_block.id());
            assert_eq!(num_blocks, 7);
        },
        TargetBlockRetrieval::TargetRound(_) => {
            panic!("Should not be TargetRound variant")
        },
    }
}
