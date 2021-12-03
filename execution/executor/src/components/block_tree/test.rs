// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::components::block_tree::{epoch_genesis_block_id, BlockLookup, BlockTree};
use diem_crypto::{hash::PRE_GENESIS_BLOCK_ID, HashValue};
use diem_infallible::Mutex;
use diem_types::{block_info::BlockInfo, epoch_state::EpochState, ledger_info::LedgerInfo};
use executor_types::{ExecutedChunk, ExecutedTrees};
use std::sync::Arc;

impl BlockTree {
    pub fn new_empty() -> Self {
        let block_lookup = Arc::new(BlockLookup::new());
        let result_view = ExecutedTrees::new_empty();
        let root = block_lookup
            .fetch_or_add_block(
                *PRE_GENESIS_BLOCK_ID,
                ExecutedChunk::new_empty(result_view),
                None,
            )
            .unwrap();

        Self {
            root: Mutex::new(root),
            block_lookup,
        }
    }

    pub fn size(&self) -> usize {
        self.block_lookup.inner.lock().0.len()
    }
}

fn id(index: u64) -> HashValue {
    let bytes = index.to_be_bytes();
    let mut buf = [0; HashValue::LENGTH];
    buf[HashValue::LENGTH - 8..].copy_from_slice(&bytes);
    HashValue::new(buf)
}

fn empty_chunk() -> ExecutedChunk {
    ExecutedChunk::new_empty(ExecutedTrees::new_empty())
}

fn gen_ledger_info(block_id: HashValue, reconfig: bool) -> LedgerInfo {
    LedgerInfo::new(
        BlockInfo::new(
            1,
            0,
            block_id,
            HashValue::zero(),
            0,
            0,
            if reconfig {
                Some(EpochState::empty())
            } else {
                None
            },
        ),
        HashValue::zero(),
    )
}

fn create_tree() -> BlockTree {
    //    * ---> 1 ---> 2
    //    |      |
    //    |      └----> 3 ---> 4
    //    |             |
    //    |             └----> 5
    //    |
    //    └----> 6 ---> 7 ---> 8
    //           |
    //           └----> 9 ---> 10
    //                  |
    //                  └----> 11
    // *: PRE_GENESIS_BLOCK_ID
    let block_tree = BlockTree::new_empty();

    block_tree
        .add_block(*PRE_GENESIS_BLOCK_ID, id(1), empty_chunk())
        .unwrap();
    block_tree.add_block(id(1), id(2), empty_chunk()).unwrap();
    block_tree.add_block(id(1), id(3), empty_chunk()).unwrap();
    block_tree.add_block(id(3), id(4), empty_chunk()).unwrap();
    block_tree.add_block(id(3), id(5), empty_chunk()).unwrap();
    block_tree
        .add_block(*PRE_GENESIS_BLOCK_ID, id(6), empty_chunk())
        .unwrap();
    block_tree.add_block(id(6), id(7), empty_chunk()).unwrap();
    block_tree.add_block(id(7), id(8), empty_chunk()).unwrap();
    block_tree.add_block(id(6), id(9), empty_chunk()).unwrap();
    block_tree.add_block(id(9), id(10), empty_chunk()).unwrap();
    block_tree.add_block(id(9), id(11), empty_chunk()).unwrap();
    block_tree
}

#[test]
fn test_branch() {
    let block_tree = create_tree();
    // put counting blocks as a separate line to avoid core dump
    // if assertion fails.
    let num_blocks = block_tree.size();
    assert_eq!(num_blocks, 12);
    block_tree.prune(&gen_ledger_info(id(9), false)).unwrap();
    let num_blocks = block_tree.size();
    assert_eq!(num_blocks, 3);
    assert_eq!(block_tree.root_block().id, id(9));
}

#[test]
fn test_reconfig_id_update() {
    let block_tree = create_tree();
    let ledger_info = gen_ledger_info(id(1), true);
    block_tree.prune(&ledger_info).unwrap();
    let num_blocks = block_tree.size();
    // reconfig suffix blocks are ditched
    assert_eq!(num_blocks, 1);
    assert_eq!(
        block_tree.root_block().id,
        epoch_genesis_block_id(&ledger_info)
    );
}

#[test]
fn test_add_duplicate_block() {
    let block_tree = create_tree();
    block_tree.add_block(id(1), id(2), empty_chunk()).unwrap();
    block_tree.add_block(id(1), id(2), empty_chunk()).unwrap();
    // can't change parent
    assert!(block_tree.add_block(id(1), id(7), empty_chunk()).is_err());
}

#[test]
fn test_add_block_missing_parent() {
    let block_tree = create_tree();
    assert!(block_tree
        .add_block(id(99), id(100), empty_chunk())
        .is_err());
}
