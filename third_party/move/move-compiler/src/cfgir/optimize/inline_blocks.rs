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
    // 'start' block has an implicit branch to it.
    counts.insert(start, 1);
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

    // Blocks move from working_blocks to finished_blocks as
    // they are processed (unless they are dropped).
    let mut working_blocks = std::mem::take(blocks);
    // Note that std::mem::take() replaces `*blocks` by
    // the default (a new empty BTreeMap), which we
    // borrow &mut to as finished_blocks.
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

        match block.back().unwrap() {
            sp!(_, Command_::Jump { target, .. }) if single_jump_targets.contains(target) => {
                // Note that only the last merged block will be left for cur.
                remapping.insert(cur, *target);
                let target_block = working_blocks.remove(target).unwrap_or_else(|| {
                    finished_blocks.remove(target).unwrap_or_else(|| {
                        panic!(
                            "ICE: Target {} not found in working_blocks or finished_blocks",
                            target
                        )
                    })
                });
                block.pop_back();
                block.extend(target_block);
                // put cur's block back into working_blocks, as we will revisit it on next iter.
                working_blocks.insert(cur, block);
                // Note that target_block is droppped.
            },
            _ => {
                next = labels.next();
                finished_blocks.insert(cur, block);
            },
        }
    }

    remap_to_last_target(remapping, start, finished_blocks)
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
/// Returns true if a label might have changed.
fn remap_to_last_target(
    mut remapping: BTreeMap<Label, Label>,
    start: Label,
    blocks: &mut BasicBlocks,
) -> bool {
    // The start block can't be relabelled in the current CFG API.
    // But it does not need to be since it will always be the first block, thus it will not run
    // into issues in the bytecode verifier
    remapping.remove(&start);
    if remapping.is_empty() {
        return false;
    }

    // close transitive chains (lab1 -> lab2 -> lab3 becomes lab1 -> lab3).
    for label in blocks.keys() {
        if let Some(target_label) = remapping.get(label) {
            let mut prev_label = label;
            let mut next_label = target_label;
            while prev_label != next_label {
                match remapping.get(next_label) {
                    Some(next_next_label) => {
                        prev_label = next_label;
                        next_label = next_next_label;
                    },
                    None => {
                        break;
                    },
                };
            }
            if next_label != label {
                remapping.insert(*label, *next_label);
            } else {
                remapping.remove(label);
            }
        }
    }
    if !remapping.is_empty() {
        let owned_blocks = std::mem::take(blocks);
        let (_start, remapped_blocks) = remap_labels(&remapping, start, owned_blocks);
        *blocks = remapped_blocks;
        true
    } else {
        false
    }
}
