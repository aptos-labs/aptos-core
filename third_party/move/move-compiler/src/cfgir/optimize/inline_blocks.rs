// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cfgir::{
        ast::remap_labels,
        cfg::{BlockCFG, CFG},
    },
    hlir::ast::{BasicBlocks, Command_, FunctionSignature, Label, SingleType},
    parser::ast::Var,
    shared::unique_map::UniqueMap,
};
use std::collections::{BTreeMap, BTreeSet};

/// returns true if anything changed
pub fn optimize(
    _signature: &FunctionSignature,
    _locals: &UniqueMap<Var, SingleType>,
    cfg: &mut BlockCFG,
) -> bool {
    let changed = optimize_(cfg.start_block(), cfg.blocks_mut());
    if changed {
        let dead_blocks = cfg.recompute();
        assert!(dead_blocks.is_empty())
    }
    changed
}

fn optimize_(start: Label, blocks: &mut BasicBlocks) -> bool {
    let single_target_labels = find_single_target_labels(start, blocks);
    inline_single_target_blocks(&single_target_labels, start, blocks)
}

// Return a list of labels that have just a single branch to them.

fn find_single_target_labels(start: Label, blocks: &BasicBlocks) -> BTreeSet<Label> {
    use Command_ as C;
    let mut counts = BTreeMap::new();
    // 'start' block doesn't count as single entry for these purposes.  Give it 2.
    counts.insert(start, 2);
    for block in blocks.values() {
        match &block.back().unwrap().value {
            C::JumpIf {
                cond: _cond,
                if_true,
                if_false,
            } => {
                *counts.entry(*if_true).or_insert(0) += 1;
                *counts.entry(*if_false).or_insert(0) += 1
            },
            C::Jump { target, .. } => *counts.entry(*target).or_insert(0) += 1,
            _ => (),
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(lbl, _)| lbl)
        .collect()
}

#[allow(clippy::needless_collect)]
fn inline_single_target_blocks(
    single_jump_targets: &BTreeSet<Label>,
    start: Label,
    blocks: &mut BasicBlocks,
) -> bool {
    //cleanup of needless_collect would result in mut and non mut borrows, and compilation warning.
    let labels_vec = blocks.keys().cloned().collect::<Vec<_>>();
    let mut changed = false;

    // all blocks
    let mut working_blocks = std::mem::take(blocks);
    let finished_blocks = blocks;

    let mut remapping = BTreeMap::new();

    // Iterate through labels.
    let mut labels = labels_vec.into_iter();
    let mut next = labels.next();
    while let Some(cur) = next {

        // temporarily get cur's block for mutability.
        let mut block = match working_blocks.remove(&cur) {
            None => {
                next = labels.next();
                continue;
            },
            Some(b) => b,
        };

        // If cur will be merged into another block, skip it for now.
        if single_jump_targets.contains(&cur) {
            next = labels.next();
            finished_blocks.insert(cur, block);
            continue;
        }

        match block.back().unwrap() {
            // Do not need to worry about infinitely unwrapping loops as loop heads will always
            // be the target of at least 2 jumps: the jump to the loop and the "continue" jump
            // This is always true as long as we start the count for the start label at 1
            sp!(_, Command_::Jump { target, .. }) if single_jump_targets.contains(target) => {
                // Note that only the last merged block will be left for cur.
                remapping.insert(cur, *target);
                match working_blocks.remove(target) {
                    Some(target_block) => {
                        block.pop_back();
                        block.extend(target_block);
                    },
                    None => {
                        match finished_blocks.remove(target) {
                            Some(target_block) => {
                                block.pop_back();
                                block.extend(target_block);
                            },
                            None => {
                                panic!(
                                    "ICE: Target {} not found in working_blocks or finished_blocks",
                                    target
                                );
                            },
                        };
                    },
                };
                changed = true;
                // put cur's block back into working_blocks, as we will revisit it on next iter.
                working_blocks.insert(cur, block);
                // Note that target block is droppped.
            },
            _ => {
                next = labels.next();
                finished_blocks.insert(cur, block);
            },
        }
    }


    // let changed = !remapping.is_empty();
    remap_to_last_target(remapping, start, &mut working_blocks);
    changed
}

/// In order to preserve loop invariants at the bytecode level, when a block is "inlined", that
/// block needs to be relabelled as the "inlined" block
/// Without this, blocks that were outside of loops could be inlined into the loop-body, breaking
/// invariants needed at the bytecode level.
/// For example:
/// Before:
///   A: block_a; jump B
///   B: block_b
///
///   s.t. A is the only one jumping to B
///
/// After:
///   B: block_a; block_b
fn remap_to_last_target(
    mut remapping: BTreeMap<Label, Label>,
    start: Label,
    blocks: &mut BasicBlocks,
) {
    // The start block can't be relabelled in the current CFG API.
    // But it does not need to be since it will always be the first block, thus it will not run
    // into issues in the bytecode verifier
    remapping.remove(&start);
    if remapping.is_empty() {
        return;
    }
    // populate remapping for non changed blocks
    for label in blocks.keys() {
        remapping.entry(*label).or_insert(*label);
    }
    let owned_blocks = std::mem::take(blocks);
    let (_start, remapped_blocks) = remap_labels(&remapping, start, owned_blocks);
    *blocks = remapped_blocks;
}
