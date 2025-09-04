// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block_storage::{
    block_store::sync_manager::TargetBlockRetrieval,
    execution_pool::common_test::create_block_tree_with_forks_unordered_parents, BlockReader,
    BlockStore,
};
use velor_consensus_types::{
    block_retrieval::{
        BlockRetrievalRequest, BlockRetrievalRequestV1, BlockRetrievalRequestV2,
        BlockRetrievalResponse, BlockRetrievalStatus,
    },
    quorum_cert::QuorumCert,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use claims::assert_ok;
use std::sync::Arc;

#[tokio::test]
async fn test_no_window_quorum_round_greater_than_commit_round() {
    let window_size: Option<u64> = None;
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, _a1_r1, _a2_r3, _a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    let commit_root = block_store.commit_root().id();
    let window_root = block_store.window_root().id();

    // Use a4_r9 as an example of a quorum cert and commit from a different node
    let highest_quorum_cert = a4_r9.quorum_cert().clone();
    let highest_quorum_cert_round = highest_quorum_cert.certified_block().round();
    let highest_quorum_cert_id = highest_quorum_cert.certified_block().id();
    let highest_commit_cert = highest_quorum_cert.into_wrapped_ledger_info();
    let highest_commit_cert_round = highest_commit_cert.commit_info().round();

    // commit_root, window_root (my validator)       highest_quorum_cert (different validator)
    //      ┌───────────────┘             ┌─────────────────────────────┘
    //      ↓                             ↓
    //  Genesis ──> A1_R1 ──> A2_R3 ──> A3_R6 ──> A4_R9
    //      ↑
    //      └────────────┐
    //    highest_commit_cert (different validator)
    assert_eq!(commit_root, genesis_block.id());
    assert_eq!(window_root, genesis_block.id());
    assert_eq!(highest_quorum_cert_round, 6);
    assert_eq!(highest_commit_cert_round, 0);

    let (payload, num_blocks) = BlockStore::generate_target_block_retrieval_payload_and_num_blocks(
        &highest_quorum_cert,
        &highest_commit_cert,
        window_size,
    );

    match payload {
        TargetBlockRetrieval::TargetBlockId(id) => {
            assert_eq!(id, genesis_block.id());
            assert_eq!(num_blocks, 7);

            let request =
                BlockRetrievalRequest::V1(BlockRetrievalRequestV1::new_with_target_block_id(
                    highest_quorum_cert_id,
                    num_blocks,
                    id,
                ));
            let response = block_store.process_block_retrieval_inner(&request).await;
            let blocks = response.blocks();

            assert_eq!(
                blocks.first().expect("No first block found").round(),
                _a3_r6.block().round()
            );
            assert_eq!(
                blocks.last().expect("No last block found").round(),
                genesis_block.block().round()
            );

            // Verifies BlockRetrievalStatus and num_blocks and target_block_id matches
            assert_ok!(response.verify_inner(&request));
        },
        TargetBlockRetrieval::TargetRound(_) => {
            panic!("Should not be TargetRound variant")
        },
    }
}

#[tokio::test]
async fn test_window_quorum_round_greater_than_commit_round() {
    let window_size: Option<u64> = Some(1u64);
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [_genesis_block, a1_r1, a2_r3, a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    // Prune tree, moves the commit root to a2_r3 and move the highest_commit_root too
    block_store.commit_callback(
        a2_r3.block().id(),
        a2_r3.block().round(),
        a3_r6.quorum_cert().into_wrapped_ledger_info(), // TODO this is correct right?
        window_size,
    );
    let commit_root = block_store.commit_root().id();
    let window_root = block_store.window_root().id();

    // Use a4_r9 as an example of a quorum cert and commit from a different node
    let highest_quorum_cert = a4_r9.quorum_cert().clone();
    let highest_quorum_cert_round = highest_quorum_cert.certified_block().round();
    let highest_quorum_cert_id = highest_quorum_cert.certified_block().id();
    let highest_commit_cert = block_store.highest_commit_cert();
    let highest_commit_cert_round = highest_commit_cert.commit_info().round();

    // commit_root, window_root (my validator)       highest_quorum_cert (different validator)
    //             └────────────┐         ┌─────────────────────────────┘
    //                          ↓         ↓
    //  Genesis ──> A1_R1 ──> A2_R3 ──> A3_R6 ──> A4_R9
    //                          ↑
    //                   ┌──────┘
    //    highest_commit_cert (different validator)
    assert_eq!(window_root, a2_r3.id());
    assert_eq!(commit_root, a2_r3.id());
    assert_eq!(highest_quorum_cert_round, 6);
    assert_eq!(highest_commit_cert_round, 3);

    let payload_generator = |block_store: Arc<BlockStore>,
                             highest_quorum_cert: QuorumCert,
                             highest_commit_cert: Arc<WrappedLedgerInfo>,
                             window_size: Option<u64>| async move {
        let (payload, num_blocks) =
            BlockStore::generate_target_block_retrieval_payload_and_num_blocks(
                &highest_quorum_cert,
                &highest_commit_cert,
                window_size,
            );

        match payload {
            TargetBlockRetrieval::TargetBlockId(_) => {
                panic!("Should not be TargetBlockId variant")
            },
            TargetBlockRetrieval::TargetRound(target_round) => {
                let request =
                    BlockRetrievalRequest::V2(BlockRetrievalRequestV2::new_with_target_round(
                        highest_quorum_cert_id,
                        num_blocks,
                        target_round,
                    ));
                let response = block_store.process_block_retrieval_inner(&request).await;
                let process_block_retrieval_response_blocks = response.blocks().clone();

                // Verifies BlockRetrievalStatus and num_blocks and target_round matches
                assert_ok!(response.verify_inner(&request));

                (
                    payload,
                    num_blocks,
                    target_round,
                    process_block_retrieval_response_blocks,
                )
            },
        }
    };

    // ----------------------------------- window_size = 1 ----------------------------------- //

    let window_size = Some(1u64);
    let (_, num_blocks, target_round, process_block_retrieval_response_blocks) = payload_generator(
        block_store.clone(),
        highest_quorum_cert.clone(),
        highest_commit_cert.clone(),
        window_size,
    )
    .await;
    assert_eq!(target_round, 3);
    assert_eq!(num_blocks, 4);
    assert_eq!(
        process_block_retrieval_response_blocks
            .first()
            .expect("No first block found")
            .round(),
        a3_r6.block().round()
    );
    assert_eq!(
        process_block_retrieval_response_blocks
            .last()
            .expect("No last block found")
            .round(),
        a2_r3.block().round()
    );

    // ----------------------------------- window_size = 3 ----------------------------------- //

    let window_size = Some(3u64);
    let (_, num_blocks, target_round, process_block_retrieval_response_blocks) = payload_generator(
        block_store.clone(),
        highest_quorum_cert.clone(),
        highest_commit_cert.clone(),
        window_size,
    )
    .await;
    assert_eq!(target_round, 1);
    assert_eq!(num_blocks, 6);
    assert_eq!(
        process_block_retrieval_response_blocks
            .first()
            .expect("No first block found")
            .round(),
        a3_r6.block().round()
    );
    assert_eq!(
        process_block_retrieval_response_blocks
            .last()
            .expect("No last block found")
            .round(),
        a1_r1.round()
    );

    // ----------------------------------- window_size = 5 ----------------------------------- //
    // This is the same as window_size = 3 because the target_round is 1

    let window_size = Some(5u64);
    let (_, num_blocks, target_round, process_block_retrieval_response_blocks) = payload_generator(
        block_store.clone(),
        highest_quorum_cert,
        highest_commit_cert,
        window_size,
    )
    .await;
    assert_eq!(target_round, 1);
    assert_eq!(num_blocks, 6);
    assert_eq!(
        process_block_retrieval_response_blocks
            .first()
            .expect("No first block found")
            .round(),
        a3_r6.block().round()
    );
    assert_eq!(
        process_block_retrieval_response_blocks
            .last()
            .expect("No last block found")
            .round(),
        a1_r1.round()
    );
}

#[tokio::test]
async fn test_verify_badly_formed_retrieval_responses() {
    let window_size: Option<u64> = Some(1u64);
    let (_, _block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [_genesis_block, a1_r1, a2_r3, a3_r6, _a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    //  Genesis ──> A1_R1 ──> A2_R3 ──> A3_R6 ──> A4_R9

    let request = BlockRetrievalRequest::new_with_target_round(a3_r6.id(), 6, 2);

    // Correct SucceededWithTarget: [ A2_R3 ──> A3_R6 ]
    let response = BlockRetrievalResponse::new(BlockRetrievalStatus::SucceededWithTarget, vec![
        a3_r6.block().clone(),
        a2_r3.block().clone(),
    ]);
    assert!(response.verify_inner(&request).is_ok());

    // Correct SucceededWithTarget, but not marked as SucceededWithTarget
    for status in [
        BlockRetrievalStatus::Succeeded,
        BlockRetrievalStatus::NotEnoughBlocks,
    ] {
        let response =
            BlockRetrievalResponse::new(status, vec![a3_r6.block().clone(), a2_r3.block().clone()]);
        assert!(response.verify_inner(&request).is_err());
    }

    // Insufficient SucceededWithTarget
    let response =
        BlockRetrievalResponse::new(BlockRetrievalStatus::SucceededWithTarget, vec![a3_r6
            .block()
            .clone()]);
    assert!(response.verify_inner(&request).is_err());

    // Block returned not within target round
    for status in [
        BlockRetrievalStatus::SucceededWithTarget,
        BlockRetrievalStatus::Succeeded,
        BlockRetrievalStatus::NotEnoughBlocks,
    ] {
        let response = BlockRetrievalResponse::new(status, vec![
            a3_r6.block().clone(),
            a2_r3.block().clone(),
            a1_r1.block().clone(),
        ]);
        assert!(response.verify_inner(&request).is_err());
    }
}
