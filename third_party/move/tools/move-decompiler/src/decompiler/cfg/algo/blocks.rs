// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::super::datastructs::*;

pub fn remove_block<BlockContent: BlockContentTrait>(
    blocks: &mut Vec<BasicBlock<usize, BlockContent>>,
    block_id: usize,
) {
    let update = |dest: &mut usize| {
        if *dest == block_id {
            panic!("block {} still referenced", block_id);
        } else if *dest > block_id {
            *dest -= 1;
        }
    };
    let update_set = |set: &mut HashSet<usize>| {
        let mut new_set = HashSet::new();
        for &id in set.iter() {
            if id == block_id {
                panic!("block {} still referenced", block_id);
            } else if id > block_id {
                new_set.insert(id - 1);
            } else {
                new_set.insert(id);
            }
        }
        *set = new_set;
    };
    let update_terminator = |terminator: &mut Terminator<usize>| match terminator {
        Terminator::Branch { target } => {
            update(target);
        }
        Terminator::IfElse {
            if_block,
            else_block,
        } => {
            update(if_block);
            update(else_block);
        }
        Terminator::While {
            inner_block,
            outer_block,
            content_blocks,
        } => {
            update(inner_block);
            update(outer_block);
            update_set(content_blocks);
        }
        Terminator::Break { target } => {
            update(target);
        }
        Terminator::Continue { target } => {
            update(target);
        }
        Terminator::Ret | Terminator::Abort | Terminator::Normal => {}
    };
    for block in blocks.iter_mut() {
        if block.idx != block_id {
            update(&mut block.idx);
        }
        update_terminator(&mut block.next);
        if let Some((idx, contents)) = &mut block.unconditional_loop_entry {
            update(idx);
            update_set(contents);
        }
        update_set(&mut block.topo_after);
        update_set(&mut block.topo_before);
        if let Some((_, terminator)) = &mut block.short_circuit_terminator {
            update_terminator(terminator);
        }
    }
    blocks.remove(block_id);
}
