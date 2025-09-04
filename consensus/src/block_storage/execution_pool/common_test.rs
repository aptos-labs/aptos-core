// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{BlockReader, BlockStore},
    test_utils::TreeInserter,
};
use velor_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
    vote_data::VoteData,
};
use velor_crypto::{hash::CryptoHash, HashValue};
use velor_types::{
    aggregate_signature::PartialSignatures,
    block_info::{BlockInfo, Round},
    ledger_info::{LedgerInfo, LedgerInfoWithVerifiedSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
};
use std::sync::Arc;

pub const DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM: usize = 10;

/// Helper function to create a `QuorumCert` which can provide a `highest_commit_cert` via
/// `highest_quorum_cert.into_wrapped_ledger_info()`
#[allow(dead_code)]
pub fn generate_qc(round: Round, parent_round: Round) -> QuorumCert {
    let num_nodes = 4;
    let (signers, validators) = random_validator_verifier(num_nodes, None, false);
    let quorum_size = validators.quorum_voting_power() as usize;

    let generate_quorum_inner = |round, parent_round| {
        let vote_data = VoteData::new(BlockInfo::random(round), BlockInfo::random(parent_round));
        let mut ledger_info = LedgerInfoWithVerifiedSignatures::new(
            LedgerInfo::new(BlockInfo::empty(), vote_data.hash()),
            PartialSignatures::empty(),
        );
        for signer in &signers[0..quorum_size] {
            let signature = signer.sign(ledger_info.ledger_info()).unwrap();
            ledger_info.add_signature(signer.author(), signature);
        }
        QuorumCert::new(
            vote_data,
            ledger_info.aggregate_signatures(&validators).unwrap(),
        )
    };

    generate_quorum_inner(round, parent_round)
}

/// Helper function to get the [`OrderedBlockWindow`](velor_consensus_types::pipelined_block::OrderedBlockWindow)
/// from the `block_store`
pub fn get_blocks_from_block_store_and_window(
    block_store: Arc<BlockStore>,
    block: &Block,
    window_size: Option<u64>,
) -> Vec<Block> {
    let windowed_blocks = block_store.get_ordered_block_window(block, window_size);
    let ordered_block_window = windowed_blocks.expect("Expected valid OrderedBlockWindow");
    ordered_block_window.blocks().to_owned()
}

/// Helper function to create a block tree of size `N` with no forks
/// ```text
/// +--------------+       +---------+       +---------+       +---------+       +---------+
/// | Genesis Block| ----> | Block 1 | ----> | Block 2 | ----> | Block 3 | ----> | Block 4 | --> ...
/// +--------------+       +---------+       +---------+       +---------+       +---------+
/// ```
///
/// NOTE: `num_blocks` includes the genesis block
pub async fn create_block_tree_no_forks_inner<const N: usize>(
    num_blocks: u64,
    window_size: Option<u64>,
    max_pruned_blocks_in_mem: usize,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; N]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        max_pruned_blocks_in_mem,
        None,
    );
    let block_store = inserter.block_store();

    // Block Store is initialized with a genesis block
    let genesis_pipelined_block = block_store
        .get_block(block_store.ordered_root().id())
        .expect("No genesis block found in BlockStore");
    assert!(genesis_pipelined_block.block().is_genesis_block());
    assert_eq!(genesis_pipelined_block.parent_id(), HashValue::zero());
    let mut cur_node = genesis_pipelined_block.clone();

    // num_blocks + 1
    let mut pipelined_blocks = Vec::with_capacity(num_blocks as usize);
    pipelined_blocks.push(genesis_pipelined_block.clone());

    // Adds `num_blocks` blocks to the block_store
    for round in 1..num_blocks {
        if round == 1 {
            cur_node = inserter
                .insert_block_with_qc(certificate_for_genesis(), &genesis_pipelined_block, round)
                .await;
        } else {
            cur_node = inserter.insert_block(&cur_node, round, None).await;
        }
        pipelined_blocks.push(cur_node.clone());
    }
    let pipelined_blocks: [Arc<PipelinedBlock>; N] = pipelined_blocks
        .try_into()
        .expect("Unexpected error converting fixed size vector into fixed size array. Ensure the generic `N` is equal to `num_blocks`");

    (inserter, block_store, pipelined_blocks)
}

/// Same as [`create_block_tree_no_forks_inner`](create_block_tree_no_forks_inner) defined above
/// however ths includes a default for `max_pruned_blocks_in_mem` of 10
pub async fn create_block_tree_no_forks<const N: usize>(
    num_blocks: u64,
    window_size: Option<u64>,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; N]) {
    create_block_tree_no_forks_inner(num_blocks, window_size, DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM)
        .await
}

/// Create a block tree with forks. Similar to [`build_simple_tree`](crate::test_utils::build_simple_tree)
/// Returns the following tree.
///
/// ```text
///       ╭--> A1--> A2--> A3
/// Genesis--> B1--> B2
///             ╰--> C1
/// ```
///
/// WARNING: Be wary of changing this function, it will affect consumers downstream
#[cfg(test)]
pub async fn create_block_tree_with_forks(
    window_size: Option<u64>,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; 7]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
        None,
    );
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let genesis_block_id = genesis.id();
    let genesis_block = block_store
        .get_block(genesis_block_id)
        .expect("genesis block must exist");
    assert_eq!(genesis_block.parent_id(), HashValue::zero());

    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert!(block_store.block_exists(genesis_block.id()));

    // a1 -> round 1
    let a1_r1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 1)
        .await;
    let a2_r2 = inserter.insert_block(&a1_r1, 2, None).await;
    let a3_r3 = inserter.insert_block(&a2_r2, 3, None).await;
    let b1_r4 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 4)
        .await;
    let b2_r5 = inserter.insert_block(&b1_r4, 5, None).await;
    let c1_r6 = inserter.insert_block(&b1_r4, 6, None).await;

    let pipelined_blocks: [Arc<PipelinedBlock>; 7] =
        [genesis_block, a1_r1, a2_r2, a3_r3, b1_r4, b2_r5, c1_r6];

    (inserter, block_store, pipelined_blocks)
}

/// Create a block tree with forks. Similar to [`create_block_tree_with_forks`](create_block_tree_with_forks)
/// but blocks within a fork are not strictly sequentially increasing in round.
///
/// e.g., A1 = round 1, A2 = round 3, A3 = round 6
///
/// ```text
///       ╭--> A1--> A2--> A3--> A4
/// Genesis--> B1--> B2--> B3
///             ╰--> C1
///             ╰--> D1
/// ```
///
/// WARNING: Be wary of changing this function, it will affect consumers downstream
pub async fn create_block_tree_with_forks_unordered_parents(
    window_size: Option<u64>,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; 10]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
        None,
    );
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let genesis_block_id = genesis.id();
    let genesis_block = block_store
        .get_block(genesis_block_id)
        .expect("genesis block must exist");
    assert_eq!(genesis_block.parent_id(), HashValue::zero());

    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert!(block_store.block_exists(genesis_block.id()));

    // a1 -> round 1
    let a1_r1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 1)
        .await;
    let b1_r2 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 2)
        .await;

    // NOTE: we are adding a1_r1 as the committed block here in insert_block
    // This is important for setting the right QuorumCert for commit_callback in tests
    let a2_r3 = inserter
        .insert_block(&a1_r1, 3, Some(a1_r1.block_info()))
        .await;

    let b2_r4 = inserter.insert_block(&b1_r2, 4, None).await;
    let b3_r5 = inserter.insert_block(&b2_r4, 5, None).await;

    // NOTE: we are adding a2_r3 as the committed block here in insert_block
    // This is important for setting the right QuorumCert for commit_callback in tests
    let a3_r6 = inserter
        .insert_block(&a2_r3, 6, Some(a2_r3.block_info()))
        .await;
    let c1_r7 = inserter.insert_block(&b1_r2, 7, None).await;
    let d1_r8 = inserter.insert_block(&b1_r2, 8, None).await;
    let a4_r9 = inserter
        .insert_block(&a3_r6, 9, Some(genesis.block_info()))
        .await;

    let pipelined_blocks: [Arc<PipelinedBlock>; 10] = [
        genesis_block,
        a1_r1,
        a2_r3,
        a3_r6,
        a4_r9,
        b1_r2,
        b2_r4,
        b3_r5,
        c1_r7,
        d1_r8,
    ];

    (inserter, block_store, pipelined_blocks)
}

/// Same as [`create_block_tree_with_forks_unordered_parents`](create_block_tree_with_forks_unordered_parents)
/// but makes `a4_r9` a nil block and `b2_r4` a nil block. This is to test that nil blocks have
/// windows. See the test case below.
pub async fn create_block_tree_with_forks_unordered_parents_and_nil_blocks(
    window_size: Option<u64>,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; 10]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
        None,
    );
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let genesis_block_id = genesis.id();
    let genesis_block = block_store
        .get_block(genesis_block_id)
        .expect("genesis block must exist");
    assert_eq!(genesis_block.parent_id(), HashValue::zero());

    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert!(block_store.block_exists(genesis_block.id()));

    // a1 -> round 1
    let a1_r1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 1)
        .await;
    let b1_r2 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 2)
        .await;
    let a2_r3 = inserter.insert_block(&a1_r1, 3, None).await;

    // Nil block
    let b2_r4 = inserter.insert_nil_block(&b1_r2, 4, None).await;
    let b3_r5 = inserter.insert_block(&b2_r4, 5, None).await;
    let a3_r6 = inserter.insert_block(&a2_r3, 6, None).await;
    let c1_r7 = inserter.insert_block(&b1_r2, 7, None).await;
    let d1_r8 = inserter.insert_block(&b1_r2, 8, None).await;

    // Nil block
    let a4_r9 = inserter
        .insert_nil_block(&a3_r6, 9, Some(genesis.block_info()))
        .await;

    let pipelined_blocks: [Arc<PipelinedBlock>; 10] = [
        genesis_block,
        a1_r1,
        a2_r3,
        a3_r6,
        a4_r9,
        b1_r2,
        b2_r4,
        b3_r5,
        c1_r7,
        d1_r8,
    ];

    (inserter, block_store, pipelined_blocks)
}
