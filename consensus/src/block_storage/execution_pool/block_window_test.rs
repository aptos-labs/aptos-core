// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for Execution Pool and their behaviors on the block_store
//!
//! Note: For the sake of testing, some functions use
//! [`prune_tree`](BlockStore::prune_tree) to mimic some of the functionality present in
//! [`commit_callback`](crate::block_storage::block_tree::BlockTree::commit_callback)
//! however they are still different and should be treated as such. But this is why you may
//! sometimes see the `window_root` behind the `commit_root`.
//! This should not happen in production.

use crate::block_storage::{
    execution_pool::common_test::{
        create_block_tree_no_forks, create_block_tree_no_forks_inner, create_block_tree_with_forks,
        create_block_tree_with_forks_unordered_parents,
        create_block_tree_with_forks_unordered_parents_and_nil_blocks,
        get_blocks_from_block_store_and_window,
    },
    BlockReader,
};

#[tokio::test]
async fn test_execution_pool_block_window_with_forks() {
    let window_size = Some(3u64);

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
    let window_size = Some(10u64);

    // Genesis ──> A1 ──> A2 ──> A3
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;
    let [genesis_block, a1, a2, a3] = pipelined_blocks;

    // ┌──────────────────────┐
    // │ Genesis ─> A1 ──> A2 │ ──> A3
    // └──────────────────────┘
    let blocks =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks.first().unwrap().id(), genesis_block.id());
    assert_eq!(blocks.get(1).unwrap().id(), a1.id());
    assert_eq!(blocks.get(2).unwrap().id(), a2.id());
}

#[tokio::test]
async fn test_execution_pool_block_window_with_pruning() {
    const NUM_BLOCKS: usize = 5;
    let window_size = Some(3u64);

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
    let window_size = Some(3u64);

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
    let window_size = Some(1u64);
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, _, _, _, _] = pipelined_blocks;

    // Window size must be greater than 0, should panic
    let window_size = Some(0u64);
    block_store.find_window_root(genesis_block.id(), window_size);
}

#[tokio::test]
async fn test_window_root_no_forks() {
    // window_size > NUM_BLOCKS
    const NUM_BLOCKS: usize = 5;
    let window_size = Some(8u64);
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, _a1, a2, _a3, a4] = pipelined_blocks;
    let (commit_root, window_root) = block_store.get_roots(a4.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    // commit_root      block_window
    //      ↓                ↓
    //  ┌───────────────────────────────┐
    //  │ Genesis ──>  A1 ──> A2 ──> A3 │ ──> A4
    //  └───────────────────────────────┘
    //      ↑
    // window_root
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 4);

    // Prune A2
    block_store.prune_tree(a2.id());
    let (commit_root, window_root) = block_store.get_roots(a4.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    //                   commit_root
    //                        │
    //  ┌──────────────────── ↓ ───────┐
    //  │ Genesis ──> A1 ──> A2 ──> A3 │ ──> A4
    //  └──────────────────────────────┘
    //       ↑
    //   window_root
    assert_eq!(commit_root.id(), a2.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 4);

    // ----------------------------------------------------------------------------------------- //

    // window_size < NUM_BLOCKS
    let window_size = Some(2u64);
    let (_, block_store, pipelined_blocks) =
        create_block_tree_no_forks::<{ NUM_BLOCKS }>(NUM_BLOCKS as u64, window_size).await;

    // Genesis ──> A1 ──> ... ──> A4
    let [genesis_block, _, a2, a3, a4] = pipelined_blocks;
    let (commit_root, window_root) = block_store.get_roots(a4.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    // commit_root              block_window
    //      ↓                       ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                              ↑
    //                         window_root
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(window_root.expect("Window root not found"), a3.id());
    assert_eq!(block_window.len(), 1);

    // Prune A2
    block_store.prune_tree(a2.id());
    let (commit_root, window_root) = block_store.get_roots(a4.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a4.block(), window_size);

    //               commit_root  block_window
    //                      ↓       ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                              ↑
    //                         window_root
    assert_eq!(commit_root.id(), a2.id());
    assert_eq!(window_root.expect("Window root not found"), a3.id());
    assert_eq!(block_window.len(), 1);
}

#[tokio::test]
async fn test_window_root_with_forks() {
    // window_size > length of longest fork
    let window_size = Some(8u64);

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (_, block_store, pipelined_blocks) = create_block_tree_with_forks(window_size).await;
    let [genesis_block, a1, _a2, a3, _b1, _b2, _c1] = pipelined_blocks;
    let (commit_root, window_root) = block_store.get_roots(a3.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    // commit_root   block_window
    //      ↓             ↓
    //  ┌───────────────────────┐
    //  │ Genesis ──> A1 ──> A2 │ ──> A3
    //  └───────────────────────┘
    //      ↑
    //  window_root
    // NOTE: Window root calculations are done in two places: `find_window_root` and `find_root`, update
    // both if needed
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 3);

    // Prune A1
    block_store.prune_tree(a1.id());
    let (commit_root, window_root) = block_store.get_roots(a3.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), a3.block(), window_size);

    //   commit_root   block_window
    //           └────┐   ↓
    //  ┌──────────── ↓ ────────┐
    //  │ Genesis ──> A1 ──> A2 │ ──> A3
    //  └───────────────────────┘
    //      ↑
    //  window_root
    assert_eq!(commit_root.id(), a1.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 3);

    // ----------------------------------------------------------------------------------------- //

    // window_size < length of longest fork
    let window_size = Some(1u64);

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (_, block_store, pipelined_blocks) = create_block_tree_with_forks(window_size).await;
    let [genesis_block, _a1, _a2, _a3, b1, _b2, c1] = pipelined_blocks;
    let current_block = c1.block();
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root
    //      ↓
    //  Genesis ──> B1 ──> C1
    //                     ↑
    //               window_root
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(window_root.unwrap(), c1.id());
    // This is zero length because OrderedBlockWindow consists of (window_size - 1) blocks
    assert_eq!(block_window.len(), 0);

    // Prune B1
    block_store.prune_tree(b1.id());
    let (commit_root, window_root) = block_store.get_roots(c1.block(), window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), c1.block(), window_size);

    //          commit_root
    //               ↓
    //  Genesis ──> B1 ──> C1
    //                     ↑
    //               window_root
    assert_eq!(commit_root.id(), b1.id());
    assert_eq!(window_root.unwrap(), c1.id());
    assert_eq!(block_window.len(), 0);
}

/// Checks `(block in window).round > current_block.round() - window_size`
#[tokio::test]
async fn test_window_root_with_non_sequential_round_forks() {
    // window_size > length of longest fork
    let window_size = Some(6u64);

    //       ╭--> A1--> A2--> A3--> A4
    // Genesis--> B1--> B2--> B3
    //             ╰--> C1
    //             ╰--> D1
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, a1_r1, a2_r3, a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;
    let current_block = a4_r9.block();
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root              window_root
    //      ↓                        ↓
    //                            ┌────┐
    //  Genesis ──> A1 ──> A2 ──> │ A3 │ ──> A4
    //                            └────┘
    //                               ↑
    //                          block_window
    //
    // (block in window).round > current_block.round() - window_size
    // 3 ≯ 9 - 6, thus block A2 is not included
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(window_root.expect("Window root not found"), a3_r6.id());
    assert_eq!(block_window.len(), 1);

    // expand window size to 7
    let window_size = Some(7u64);
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    // commit_root          block_window
    //      ↓                    ↓
    //                     ┌───────────┐
    //  Genesis ──> A1 ──> │ A2 ──> A3 │ ──> A4
    //                     └───────────┘
    //                       ↑
    //                  window_root
    //
    // (block in window).round > current_block.round() - window_size
    // 3 > 9 - 7, thus block A2 is included
    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(window_root.expect("Window root not found"), a2_r3.id());
    assert_eq!(block_window.len(), 2);

    // Prune A1
    block_store.prune_tree(a1_r1.id());

    // Expand window_size to 100
    let window_size = Some(100u64);
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    //      commit_root  block_window
    //            └───┐       ↓
    //  ┌──────────── ↓ ────────────────┐
    //  │ Genesis ──> A1 ──>  A2 ──> A3 │ ──> A4
    //  └───────────────────────────────┘
    //       ↑
    //  window_root
    assert_eq!(commit_root.id(), a1_r1.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 4);

    // ----------------------------------------------------------------------------------------- //

    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents(window_size).await;
    let [genesis_block, _a1_r1, a2_r3, _a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;
    let current_block = a4_r9.block();

    // Prune a2
    block_store.prune_tree(a2_r3.id());
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    //            commit_root  block_window
    //                  └──────┐    ↓
    //  ┌───────────────────── ↓ ───────┐
    //  │ Genesis ──> A1 ──>  A2 ──> A3 │ ──> A4
    //  └───────────────────────────────┘
    //       ↑
    //  window_root
    assert_eq!(commit_root.id(), a2_r3.id());
    assert_eq!(
        window_root.expect("Window root not found"),
        genesis_block.id()
    );
    assert_eq!(block_window.len(), 4);
}

/// Checks to make sure:
/// 1. nil blocks can have a window
/// 2. nil blocks can be in a window
#[tokio::test]
async fn test_window_root_with_non_sequential_round_forks_and_nil_blocks() {
    // window_size > length of longest fork
    let window_size = Some(6u64);

    //                            nil_block
    //                               ↓
    //       ╭--> A1--> A2--> A3--> A4
    // Genesis--> B1--> B2--> B3
    //             │    ↑
    //             │  nil_block
    //             ╰--> C1
    //             ╰--> D1
    //
    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents_and_nil_blocks(window_size).await;
    let [genesis_block, _a1_r1, _a2_r3, a3_r6, a4_r9, _b1_r2, _b2_r4, _b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    // a4_r9 is the nil block
    let current_block = a4_r9.block();
    let (commit_root, window_root) = block_store.get_roots(current_block, window_size);
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    assert_eq!(commit_root.id(), genesis_block.id());
    assert_eq!(window_root.expect("Window root not found"), a3_r6.id());
    assert_eq!(block_window.len(), 1);

    // ----------------------------------------------------------------------------------------- //

    let (_, block_store, pipelined_blocks) =
        create_block_tree_with_forks_unordered_parents_and_nil_blocks(window_size).await;
    let [genesis_block, _a1_r1, _a2_r3, _a3_r6, _a4_r9, _b1_r2, b2_r4, b3_r5, _c1_r7, _d1_r8] =
        pipelined_blocks;

    // Nil block is at b2_r4. Setting current block to b3_r5.
    // Checking to make sure the nil block is included in the window
    let current_block = b3_r5.block();
    let block_window =
        get_blocks_from_block_store_and_window(block_store.clone(), current_block, window_size);

    assert_eq!(block_window.first().unwrap().id(), genesis_block.id());

    // Nil block is included in OrderedBlockWindow
    assert_eq!(block_window.get(2).unwrap().id(), b2_r4.id());
    assert!(b2_r4.block().is_nil_block());
    assert_eq!(block_window.len(), 3);
}
