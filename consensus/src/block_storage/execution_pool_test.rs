// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for Execution Pool and their behaviors on the block_store
//!
//! Note: For the sake of testing, some functions use
//! [`prune_tree`](BlockStore::prune_tree) to mimic some of the functionality present in
//! [`commit_callback`](crate::block_storage::block_tree::BlockTree::commit_callback)
//! however they are still different and should be treated as such. But this is why you may
//! sometimes see the `window_root` behind the `commit_root`.
//! This should not happen in production.

use crate::{
    block_storage::{BlockReader, BlockStore},
    test_utils::TreeInserter,
};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    pipelined_block::PipelinedBlock,
};
use aptos_crypto::HashValue;
use aptos_types::{block_info::Round, validator_signer::ValidatorSigner};
use std::sync::Arc;

const DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM: usize = 10;

/// Helper function to get the [`OrderedBlockWindow`](aptos_consensus_types::pipelined_block::OrderedBlockWindow)
/// from the `block_store`
fn get_blocks_from_block_store_and_window(
    block_store: Arc<BlockStore>,
    block: &Block,
    window_size: usize,
) -> Vec<Block> {
    let windowed_blocks = block_store
        .inner
        .read()
        .get_ordered_block_window(block, window_size);
    let ordered_block_window = windowed_blocks.unwrap();
    ordered_block_window.blocks().to_owned()
}

/// Helper function for getting `commit_root`, `window_root`
/// NOTE: `block` is the reference block to base the window on
fn get_roots(
    block: &Block,
    block_store: Arc<BlockStore>,
    window_size: usize,
) -> (Arc<PipelinedBlock>, Option<HashValue>) {
    let block_store_inner_guard = block_store.inner.read();
    let commit_root = block_store_inner_guard.commit_root();

    let window_root = block_store_inner_guard.find_window_root(block.id(), window_size);

    (commit_root, Some(window_root))
}

/// Helper function to create a block tree of size `N` with no forks
/// ```text
/// +--------------+       +---------+       +---------+       +---------+       +---------+
/// | Genesis Block| ----> | Block 1 | ----> | Block 2 | ----> | Block 3 | ----> | Block 4 | --> ...
/// +--------------+       +---------+       +---------+       +---------+       +---------+
/// ```
///
/// NOTE: `num_blocks` includes the genesis block
async fn create_block_tree_no_forks_inner<const N: usize>(
    num_blocks: u64,
    window_size: usize,
    max_pruned_blocks_in_mem: usize,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; N]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter =
        TreeInserter::new_with_params(validator_signer, window_size, max_pruned_blocks_in_mem);
    let block_store = inserter.block_store();

    // Block Store is initialized with a genesis block
    let genesis_pipelined_block = block_store
        .get_block(block_store.ordered_root().id())
        .unwrap();
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
async fn create_block_tree_no_forks<const N: usize>(
    num_blocks: u64,
    window_size: usize,
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
async fn create_block_tree_with_forks(
    window_size: usize,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; 7]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
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
async fn create_block_tree_with_forks_unordered_parents(
    window_size: usize,
) -> (TreeInserter, Arc<BlockStore>, [Arc<PipelinedBlock>; 10]) {
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
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
    let b2_r4 = inserter.insert_block(&b1_r2, 4, None).await;
    let b3_r5 = inserter.insert_block(&b2_r4, 5, None).await;
    let a3_r6 = inserter.insert_block(&a2_r3, 6, None).await;
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

/// Check the following:
/// 1. [`OrderedBlockWindow`](aptos_consensus_types::pipelined_block::OrderedBlockWindow)
///    has a length of at most (window_size - 1) blocks
/// 2. [`OrderedBlockWindow`](aptos_consensus_types::pipelined_block::OrderedBlockWindow)
///    excludes the current block.
/// 3. Block rounds are in ascending order (oldest -> newest).
/// 4. Confirm that the genesis block is not included in the
///    [`OrderedBlockWindow`](aptos_consensus_types::pipelined_block::OrderedBlockWindow).
#[tokio::test]
async fn test_execution_pool_block_window_3_no_commit() {
    let window_size: usize = 3;
    let validator_signer = ValidatorSigner::random(None);
    let mut inserter = TreeInserter::new_with_params(
        validator_signer,
        window_size,
        DEFAULT_MAX_PRUNED_BLOCKS_IN_MEM,
    );
    let block_store = inserter.block_store();
    let mut round: Round = 0;

    // Block Store is initialized with a genesis block
    let genesis_pipelined_block = block_store
        .get_block(block_store.ordered_root().id())
        .unwrap();
    assert_eq!(genesis_pipelined_block.block().round(), 0);
    assert_eq!(genesis_pipelined_block.parent_id(), HashValue::zero());
    let mut curr_pipelined_block = genesis_pipelined_block.clone();

    // | blocks inserted | window_size | round | ordered_block_window block count |
    // |-----------------|-------------|-------|----------------------------------|
    // | 0               | 3           | 0     | 0                                |
    let block = curr_pipelined_block.block();
    let blocks = get_blocks_from_block_store_and_window(block_store.clone(), block, window_size);
    assert_eq!(blocks.len(), 0);

    // | blocks inserted | window_size | round | ordered_block_window block count |
    // |-----------------|-------------|-------|----------------------------------|
    // | 1               | 3           | 1     | 0                                |
    round += 1;
    curr_pipelined_block = inserter
        .insert_block(&curr_pipelined_block, round, None)
        .await;
    let block = curr_pipelined_block.block();
    let blocks = get_blocks_from_block_store_and_window(block_store.clone(), block, window_size);

    // Confirm that the genesis block is NOT included in the OrderedBlockWindow
    assert_eq!(blocks.len(), 0);
    assert_eq!(round, 1);

    // | blocks inserted | window_size | round | ordered_block_window block count |
    // |-----------------|-------------|-------|----------------------------------|
    // | 2               | 3           | 2     | 1                                |
    round += 1;
    curr_pipelined_block = inserter
        .insert_block(&curr_pipelined_block, round, None)
        .await;
    let blocks = get_blocks_from_block_store_and_window(
        block_store.clone(),
        curr_pipelined_block.block(),
        window_size,
    );
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks.first().unwrap().round(), 1);
    assert_eq!(round, 2);

    // | blocks inserted | window_size | round | ordered_block_window block count |
    // |-----------------|-------------|-------|----------------------------------|
    // | 3               | 3           | 3     | 2                                |
    round += 1;
    curr_pipelined_block = inserter
        .insert_block(&curr_pipelined_block, round, None)
        .await;
    let blocks = get_blocks_from_block_store_and_window(
        block_store.clone(),
        curr_pipelined_block.block(),
        window_size,
    );
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks.first().unwrap().round(), 1);
    assert_eq!(blocks.get(1).unwrap().round(), 2);
    assert_eq!(round, 3);

    // | blocks inserted | window_size | round | ordered_block_window block count |
    // |-----------------|-------------|-------|----------------------------------|
    // | 4               | 3           | 4     | 2                                |
    round += 1;
    curr_pipelined_block = inserter
        .insert_block(&curr_pipelined_block, round, None)
        .await;
    let blocks = get_blocks_from_block_store_and_window(
        block_store.clone(),
        curr_pipelined_block.block(),
        window_size,
    );

    // Max should be 2, even if more blocks are added since Max(len(OrderedBlockWindow)) = window_size - 1
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks.first().unwrap().round(), 2);
    assert_eq!(blocks.get(1).unwrap().round(), 3);
    assert_eq!(round, 4);
}

#[tokio::test]
async fn test_execution_pool_block_window_with_forks() {
    let window_size: usize = 3;

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (_, block_store, pipelined_blocks) = create_block_tree_with_forks(window_size).await;
    let [_, a1, a2, a3, b1, _, c1] = pipelined_blocks;

    let ordered_root_pipelined_block = block_store.ordered_root();
    assert_eq!(ordered_root_pipelined_block.round(), 0);

    //             ┌──────────┐
    // Genesis ──> │ A1 -> A2 │ ──> A3
    //             └──────────┘
    let ordered_blocks =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);
    assert_eq!(ordered_blocks.len(), 2);
    assert_eq!(ordered_blocks.first().unwrap().id(), a1.id());
    assert_eq!(ordered_blocks.get(1).unwrap().id(), a2.id());

    //             ┌────┐
    // Genesis ──> │ B1 │ ──> C1
    //             └────┘
    let ordered_blocks =
        get_blocks_from_block_store_and_window(block_store.clone(), c1.block(), window_size);
    assert_eq!(ordered_blocks.len(), 1);
    assert_eq!(ordered_blocks.first().unwrap().id(), b1.id());
}

#[tokio::test]
async fn test_execution_pool_window_size_greater_than_block_store() {
    // window size > block store size
    const NUM_BLOCKS: usize = 4;
    let window_size: usize = 10;

    // Genesis ──> A1 ──> A2 ──> A3
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;
    let [_, a1, a2, a3] = pipelined_blocks;

    //            ┌───────────┐
    // Genesis ─> │ A1 ──> A2 │ ──> A3
    //            └───────────┘
    let blocks =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks.first().unwrap().id(), a1.id());
    assert_eq!(blocks.get(1).unwrap().id(), a2.id());
}

#[tokio::test]
async fn test_execution_pool_block_window_with_pruning() {
    const NUM_BLOCKS: usize = 5;
    let window_size: usize = 3;

    // Genesis ──> A1 ──> ... ──> A4
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;
    let [_, _, a2, a3, a4] = pipelined_blocks;

    // A2 ──> ... ──> A4
    block_store.prune_tree(a2.id());
    let ordered_root = block_store.ordered_root();
    let commit_root = block_store.commit_root();
    assert_eq!(ordered_root.round(), 2);
    assert_eq!(commit_root.round(), 2);

    // ┌───────────┐
    // │ A2 ──> A3 │ ──> A4
    // └───────────┘
    let blocks =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks.first().unwrap().id(), a2.id());
    assert_eq!(blocks.get(1).unwrap().id(), a3.id())
}

/// `get_block_window` on a block that has been pruned. Should panic if the
/// `max_pruned_blocks_in_mem` is 0.
#[should_panic]
#[tokio::test]
async fn test_execution_pool_block_window_with_pruning_failure() {
    const NUM_BLOCKS: usize = 5;
    let window_size: usize = 3;

    // No pruned blocks are not kept in the block store if this is set to 0
    let max_pruned_blocks_in_mem: usize = 0;
    let (_, block_store, pipelined_blocks) = create_block_tree_no_forks_inner::<{ NUM_BLOCKS }>(
        NUM_BLOCKS as u64,
        window_size,
        max_pruned_blocks_in_mem,
    )
    .await;
    let [_, _, a2, a3, _] = pipelined_blocks;

    block_store.prune_tree(a3.id());

    // a2 was pruned, no longer exists in the block_store
    get_blocks_from_block_store_and_window(block_store.clone(), a2.block(), window_size);
}

#[should_panic]
#[tokio::test]
async fn test_window_root_window_size_0_failure() {
    const NUM_BLOCKS: usize = 5;
    let window_size: usize = 1;
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, _, _, _, _] = pipelined_blocks;

    // Window size must be greater than 0, should panic
    let window_size = 0;
    block_store
        .inner
        .read()
        .find_window_root(genesis_block.id(), window_size);
}

#[tokio::test]
async fn test_window_root_no_forks() {
    // window_size > NUM_BLOCKS
    const NUM_BLOCKS: usize = 5;
    let window_size: usize = 8;
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, a1, a2, _, a4] = pipelined_blocks;
    let (commit_root, window_root) = get_roots(a4.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    // commit_root
    //      ↓
    //              ┌──────────────────┐
    //  Genesis ──> │ A1 ──> A2 ──> A3 │ ──> A4
    //              └──────────────────┘
    //      ↑                ↑
    // window_root      block_window
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(block_window.first().unwrap().id(), a1.id());
    assert_eq!(block_window.len(), 3);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );

    // Prune A2
    block_store.prune_tree(a2.id());
    let (commit_root, window_root) = get_roots(a4.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    //                   commit_root
    //                        │
    //              ┌──────── ↓ ───────┐
    //  Genesis ──> │ A1 ──> A2 ──> A3 │ ──> A4
    //              └──────────────────┘
    //     ↑                  ↑
    // window_root       block_window
    assert_eq!(commit_root.id(), a2.id());
    assert_eq!(block_window.first().unwrap().id(), a1.id());
    assert_eq!(block_window.len(), 3);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );

    // ----------------------------------------------------------------------------------------- //

    // window_size < NUM_BLOCKS
    let window_size: usize = 2;
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, _, a2, a3, a4] = pipelined_blocks;
    let (commit_root, window_root) = get_roots(a4.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    // commit_root     window_root
    //      ↓              ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                              ↑
    //                         block_window
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(block_window.first().unwrap().id(), a3.id());
    assert_eq!(block_window.len(), 1);
    assert_eq!(window_root.expect("Window root not found"), a2.id());

    // Prune A2
    block_store.prune_tree(a2.id());
    let (commit_root, window_root) = get_roots(a4.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    //                 commit_root
    //                      ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                      ↑        ↑
    //              window_root   block_window
    assert_eq!(commit_root.id(), a2.id());
    assert_eq!(block_window.first().unwrap().id(), a3.id());
    assert_eq!(block_window.len(), 1);
    assert_eq!(window_root.expect("Window root not found"), a2.id());
}

#[tokio::test]
async fn test_window_root_with_forks() {
    // window_size > length of longest fork
    let window_size: usize = 8;

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (_, block_store, pipelined_blocks) = create_block_tree_with_forks(window_size).await;
    let [genesis_block, a1, _a2, a3, _b1, _b2, _c1] = pipelined_blocks;
    let (commit_root, window_root) = get_roots(a3.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    // commit_root
    //      ↓
    //              ┌───────────┐
    //  Genesis ──> │ A1 ──> A2 │ ──> A3
    //              └───────────┘
    //      ↑             ↑
    // window_root   block_window
    // NOTE: Window root calculations are done in two places: `find_window_root` and `find_root`, update
    // both if needed
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(block_window.first().unwrap().id(), a1.id());
    assert_eq!(block_window.len(), 2);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );

    // Prune A1
    block_store.prune_tree(a1.id());
    let (commit_root, window_root) = get_roots(a3.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    //           commit_root
    //                │
    //              ┌ ↓ ────────┐
    //  Genesis ──> │ A1 ──> A2 │ ──> A3
    //              └───────────┘
    //     ↑              ↑
    // window_root  block_window
    assert_eq!(commit_root.id(), a1.id());
    assert_eq!(block_window.first().unwrap().id(), a1.id());
    assert_eq!(block_window.len(), 2);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );

    // ----------------------------------------------------------------------------------------- //

    // window_size < length of longest fork
    let window_size: usize = 1;

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (_, block_store, pipelined_blocks) = create_block_tree_with_forks(window_size).await;
    let [genesis_block, _a1, _a2, _a3, b1, _b2, c1] = pipelined_blocks;
    let current_block = c1.block();
    let (commit_root, window_root) = get_roots(current_block, block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root
    //      ↓
    //  Genesis ──> B1 ──> C1
    //               ↑
    //          window_root
    assert_eq!(commit_root.id(), genesis_block.id());
    // This is zero length because OrderedBlockWindow consists of (window_size - 1) blocks
    assert_eq!(block_window.len(), 0);
    assert_eq!(window_root.unwrap(), b1.id());

    // Prune B1
    block_store.prune_tree(b1.id());
    let (commit_root, window_root) = get_roots(c1.block(), block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), c1.block(), window_size);

    //          commit_root
    //               ↓
    //  Genesis ──> B1 ──> C1
    //               ↑
    //          window_root
    assert_eq!(commit_root.id(), b1.id());
    assert_eq!(block_window.len(), 0);
    assert_eq!(window_root.unwrap(), b1.id());
}

/// Checks `(block in window).round > current_block.round() - window_size`
#[tokio::test]
async fn test_window_root_with_non_sequential_round_forks() {
    // window_size > length of longest fork
    let window_size: usize = 6;

    //       ╭--> A1--> A2--> A3--> A4
    // Genesis--> B1--> B2--> B3
    //             ╰--> C1
    //             ╰--> D1
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, a1_r1, a2_r3, a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;
    let current_block = a4_r9.block();
    let (commit_root, window_root) = get_roots(current_block, block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root     window_root
    //      ↓               ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                               ↑
    //                          block_window
    //
    // (block in window).round > current_block.round() - window_size
    // 3 ≯ 9 - 6, thus block A2 is not included
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(block_window.first().unwrap().id(), a3_r6.id());
    assert_eq!(block_window.len(), 1);
    assert_eq!(window_root.expect("Window root not found"), a2_r3.id());

    // expand window size to 7
    let window_size: usize = 7;
    let (commit_root, window_root) = get_roots(current_block, block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root
    //      ↓
    //                     ┌───────────┐
    //  Genesis ──> A1 ──> │ A2 ──> A3 │ ──> A4
    //                     └───────────┘
    //               ↑            ↑
    //          window_root  block_window
    //
    // (block in window).round > current_block.round() - window_size
    // 3 > 9 - 7, thus block A2 is included
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(block_window.first().unwrap().id(), a2_r3.id());
    assert_eq!(block_window.len(), 2);
    assert_eq!(window_root.expect("Window root not found"), a1_r1.id());

    // Prune A1
    block_store.prune_tree(a1_r1.id());

    // Expand window_size to 100
    let window_size: usize = 100;
    let (commit_root, window_root) = get_roots(current_block, block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    //            commit_root
    //                │
    //              ┌ ↓ ────────────────┐
    //  Genesis ──> │ A1 ──>  A2 ──> A3 │ ──> A4
    //              └───────────────────┘
    //     ↑                    ↑
    // window_root         block_window
    assert_eq!(commit_root.id(), a1_r1.id());
    assert_eq!(block_window.first().unwrap().id(), a1_r1.id());
    assert_eq!(block_window.len(), 3);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );

    // ----------------------------------------------------------------------------------------- //

    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, a1_r1, a2_r3, _a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;
    let current_block = a4_r9.block();

    // Prune a2
    block_store.prune_tree(a2_r3.id());
    let (commit_root, window_root) = get_roots(current_block, block_store.clone(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    //                    commit_root
    //                         │
    //              ┌───────── ↓ ───────┐
    //  Genesis ──> │ A1 ──>  A2 ──> A3 │ ──> A4
    //              └───────────────────┘
    //     ↑                   ↑
    // window_root        block_window
    assert_eq!(commit_root.id(), a2_r3.id());
    assert_eq!(block_window.first().unwrap().id(), a1_r1.id());
    assert_eq!(block_window.len(), 3);
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
}
