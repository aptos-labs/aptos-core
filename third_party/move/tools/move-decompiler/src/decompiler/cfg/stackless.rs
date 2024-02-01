// Revela decompiler. Copyright (c) Verichains, 2023-2024

use std::collections::{BTreeMap, BTreeSet, HashSet};

use move_stackless_bytecode::stackless_bytecode::{AttrId, Bytecode, Label};

use super::{
    algo::{
        self,
        blocks_stackless::{AnnotatedBytecodeData, StacklessBlockContent},
    },
    datastructs::*,
    metadata::{WithMetadata, WithMetadataExt},
};

pub fn decompile(
    insts: &[Bytecode],
    initial_variables: &HashSet<usize>,
) -> Result<WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>, anyhow::Error> {
    let blocks: Vec<BasicBlock<usize, StacklessBlockContent>> =
        algo::blocks_stackless::split_basic_blocks_stackless_bytecode(insts)
            .map_err(|e| anyhow::anyhow!("Unable to split into basic blocks: {}", e))?;
    let mut blocks = algo::topo::topo_sort(blocks, true)?;
    rewrite_labels(&mut blocks)?;

    cleanup_tail_jumps_for_terminated_blocks(&mut blocks)?;
    cleanup_dummy_dispatch_blocks(&mut blocks)?;
    rewrite_labels(&mut blocks)?;

    algo::loop_reconstruction::loop_reconstruction(&mut blocks, initial_variables)?;

    let mut blocks = algo::topo::topo_sort(blocks, false)?;
    rewrite_labels(&mut blocks)?;

    annotate_jumps(&mut blocks)?;
    annotate_short_circuit_jumps(&mut blocks)?;

    let blocks_cloned = blocks.clone();
    let mut program = build_program(&blocks_cloned, blocks.iter(), false)?;
    insert_terminated_blocks_if_needed(&blocks, &mut program, None)?;

    cleanup_jumps(
        &mut program.inner_mut().blocks,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );

    trim_else(&mut program, None, false);
    trim_continue(&mut program, false);
    trim_dead_break_continue(&mut program);

    apply_short_circuit_jumps(&mut program);

    cleanup_labels(&mut program);

    Ok(program)
}

fn check_wrong_jump_program(
    program: &WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    next_block: Option<usize>,
) -> Option<usize> {
    if program.blocks.len() == 1 {
        if let HyperBlock::ConnectedBlocks(blocks) = program.blocks[0].inner() {
            if blocks.len() == 1 {
                if let Terminator::Branch { target } = blocks[0].next {
                    if next_block.is_some() && next_block.unwrap() != target {
                        return Some(target);
                    }
                }
            }
        }
    }
    return None;
}

fn insert_terminated_blocks_if_needed(
    global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    next_block: Option<usize>,
) -> Result<Option<usize>, anyhow::Error> {
    if let Some(target) = check_wrong_jump_program(program, next_block) {
        if let Some(t) = build_terminated_program(&global_blocks, target) {
            *program = t.clone();
            return Ok(None);
        }
    }
    let p = program.inner_mut();

    let mut next_block = next_block;

    for block in p.blocks.iter_mut().rev() {
        match block.inner_mut() {
            HyperBlock::ConnectedBlocks(blocks) => {
                if let Some(blk) = blocks.first() {
                    next_block = Some(blk.idx);
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                let _last_else =
                    insert_terminated_blocks_if_needed(global_blocks, else_unit, next_block)?;
                let _last_if =
                    insert_terminated_blocks_if_needed(global_blocks, if_unit, next_block)?;
                next_block = None;
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                insert_terminated_blocks_if_needed(global_blocks, outer, next_block)?;
                insert_terminated_blocks_if_needed(global_blocks, inner, None)?;
                next_block = None;
            }
        }
    }

    Ok(next_block)
}

#[allow(dead_code)]
fn to_inner<T: Clone>(x: &WithMetadata<T>) -> &T {
    x.inner()
}

fn to_inner_mut<T: Clone>(x: &mut WithMetadata<T>) -> &mut T {
    x.inner_mut()
}

fn annotate_jumps(
    blocks: &mut [BasicBlock<usize, StacklessBlockContent>],
) -> Result<(), anyhow::Error> {
    for block in blocks {
        match block.next {
            Terminator::While { .. } => {
                if !annotate_final_jump(block, JumpType::While, true)? {
                    return Err(anyhow::anyhow!("While block must end with a jump"));
                };
            }
            Terminator::Break { .. } => {
                annotate_final_jump(block, JumpType::Break, false)?;
            }
            Terminator::Continue { .. } => {
                annotate_final_jump(block, JumpType::Continue, false)?;
            }
            Terminator::Normal
            | Terminator::Ret
            | Terminator::Abort
            | Terminator::IfElse { .. }
            | Terminator::Branch { .. } => {}
        }
    }

    Ok(())
}

fn annotate_final_jump(
    block: &mut BasicBlock<usize, StacklessBlockContent>,
    jump_type: JumpType,
    require_conditional: bool,
) -> Result<bool, anyhow::Error> {
    if let Some(bytecode) = block.content.code.last_mut() {
        if let Bytecode::Jump(_, _) = bytecode.bytecode {
            if !require_conditional {
                bytecode.jump_type = jump_type;
                return Ok(true);
            }
        } else if let Bytecode::Branch(_, _, _, _) = bytecode.bytecode {
            bytecode.jump_type = jump_type;
            return Ok(true);
        }
    }

    Ok(false)
}

fn annotate_short_circuit_jumps(
    blocks: &mut [BasicBlock<usize, StacklessBlockContent>],
) -> Result<(), anyhow::Error> {
    let is_return_block = blocks
        .iter()
        .map(|block| {
            if matches!(block.next, Terminator::Ret | Terminator::Abort)
                && block.content.code.len() == 2
            {
                if let (Bytecode::Label(..), Bytecode::Ret(..) | Bytecode::Abort(..)) = (
                    &block.content.code[0].bytecode,
                    &block.content.code[1].bytecode,
                ) {
                    Some(block.content.code[1].clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut immediate_jump_target = blocks
        .iter()
        .map(|block| {
            if block.content.code.len() == 2 {
                if let (Terminator::Branch { target }, Bytecode::Label(..), Bytecode::Jump(..)) = (
                    &block.next,
                    &block.content.code[0].bytecode,
                    &block.content.code[1].bytecode,
                ) {
                    Some(*target)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut visited = vec![0; blocks.len()];
    let mut visit_n = 0;
    for i in 0..immediate_jump_target.len() {
        if visited[i] > 0 {
            continue;
        }

        if let Some(target) = immediate_jump_target[i] {
            visit_n += 1;
            visited[i] = visit_n;

            if visited[target] == 0 {
                let mut current_group = Vec::new();
                current_group.push(target);
                let mut is_cycle = false;
                let mut current = target;

                while let Some(next) = immediate_jump_target[current] {
                    if visited[next] == visit_n {
                        is_cycle = true;
                        break;
                    } else {
                        current_group.push(immediate_jump_target[next].unwrap_or(next));
                        if visited[next] > 0 {
                            break;
                        }
                        current = next;
                    }
                }

                if is_cycle {
                    current_group.iter_mut().for_each(|x| {
                        immediate_jump_target[*x] = Some(usize::MAX);
                    });
                } else {
                    let target = current_group[current_group.len() - 1];
                    if immediate_jump_target[target] == None {
                        immediate_jump_target[target] = Some(target);
                    }
                    for j in 0..(current_group.len() - 1) {
                        immediate_jump_target[current_group[j]] = immediate_jump_target[target];
                    }
                }
            }

            immediate_jump_target[i] = immediate_jump_target[target];
        }
    }

    for i in 0..blocks.len() {
        if let Terminator::Branch { target } = blocks[i].next {
            let final_target = immediate_jump_target[target].unwrap_or(target);

            if final_target == usize::MAX {
                // it's a cycle, do nothing
                continue;
            }

            if let Some(op) = &is_return_block[final_target] {
                if let Some(last_op) = blocks[i].content.code.last() {
                    if matches!(last_op.bytecode, Bytecode::Jump(..)) {
                        blocks[i].short_circuit_terminator = Some((
                            StacklessBlockContent {
                                code: vec![op.clone()],
                            },
                            blocks[final_target].next.clone(),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// ensure each block has a unique label, and rewrite all labels to block index
fn rewrite_labels(
    blocks: &mut [BasicBlock<usize, StacklessBlockContent>],
) -> Result<(), anyhow::Error> {
    let mut live_labels = BTreeSet::new();

    for block in blocks.iter() {
        for bytecode in &block.content.code {
            match bytecode.bytecode {
                Bytecode::Branch(_, a, b, _) => {
                    live_labels.insert(a);
                    live_labels.insert(b);
                }

                Bytecode::Jump(_, a) => {
                    live_labels.insert(a);
                }

                _ => {}
            }
        }
    }

    for block in blocks.iter_mut() {
        for bytecode in &mut block.content.code {
            if let Bytecode::Label(.., lbl) = bytecode.bytecode {
                if !live_labels.contains(&lbl) {
                    bytecode.removed = true;
                }
            }
        }
    }

    let mut label2block: BTreeMap<Label, usize> = BTreeMap::new();
    let mut block2label: BTreeMap<usize, Label> = BTreeMap::new();

    let mut label_remap = BTreeMap::new();

    for (idx, block) in blocks.iter().enumerate() {
        if block.idx != idx {
            return Err(anyhow::anyhow!(
                "Block {} is not in the right order, expected {}",
                block.idx,
                idx
            ));
        }

        for bytecode in &block.content.code {
            if bytecode.removed {
                continue;
            }

            if let Bytecode::Label(_, label) = bytecode.bytecode {
                if let Some(last_idx) = label2block.get(&label) {
                    if *last_idx != idx {
                        return Err(anyhow::anyhow!(
                            "Label {} is used in multiple blocks",
                            label.as_usize()
                        ));
                    }
                } else {
                    label2block.insert(label, idx);
                }

                if let Some(last_label) = block2label.get(&idx) {
                    if *last_label != label {
                        label_remap.insert(label, *last_label);
                    }
                } else {
                    block2label.insert(idx, label);
                }
            }
        }
    }

    fn update(label2block: &BTreeMap<Label, usize>, label: &mut Label) {
        if let Some(idx) = label2block.get(label) {
            *label = Label::new(*idx);
        } else {
            panic!("Error: label {} not found", label.as_usize());
        }
    }

    for block in blocks.iter_mut() {
        for bytecode in block.content.code.iter_mut() {
            if bytecode.removed {
                if let Bytecode::Label(idx, _) = bytecode.bytecode {
                    bytecode.bytecode = Bytecode::Nop(idx.clone());
                }
            } else {
                if let Bytecode::Label(idx, label) = bytecode.bytecode {
                    if label_remap.get(&label).is_some() {
                        bytecode.bytecode = Bytecode::Nop(idx.clone());
                        bytecode.removed = true;
                    }
                }
                match &mut bytecode.bytecode {
                    Bytecode::Branch(_, t, f, _) => {
                        update(&label2block, t);
                        update(&label2block, f);
                    }
                    Bytecode::Jump(_, d) => {
                        update(&label2block, d);
                    }
                    Bytecode::Label(_, l) => {
                        update(&label2block, l);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn debug_dump_program(
    program: &CodeUnitBlock<usize, StacklessBlockContent>,
    lvl: u32,
    show_bytecode: bool,
) {
    let prefix = "  ".repeat(lvl as usize);
    for block in program.blocks.iter().map(to_inner) {
        match block {
            HyperBlock::ConnectedBlocks(blocks) => {
                println!("{}//Connected block", prefix);
                for block in blocks.iter().map(to_inner) {
                    println!("{}Block {} {:?}", prefix, block.idx, block.next);
                    if show_bytecode {
                        block.content.code.iter().for_each(|bytecode| {
                            println!(
                                "{}  {:?} removed={:?} jump_type={:?}",
                                prefix, bytecode.bytecode, bytecode.removed, bytecode.jump_type
                            );
                        });
                    }
                }
                println!("{}//End connected block", prefix);
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                println!("{}If", prefix);
                debug_dump_program(if_unit.inner(), lvl + 1, show_bytecode);
                println!("{}Else", prefix);
                debug_dump_program(else_unit.inner(), lvl + 1, show_bytecode);
            }

            HyperBlock::WhileBlocks {
                inner,
                outer,
                unconditional,
                start_block: start_label,
                exit_block: exit_label,
            } => {
                println!(
                    "{}While unconditional={}, start_label={}, exit_label={}",
                    prefix, unconditional, start_label, exit_label
                );
                debug_dump_program(inner.inner(), lvl + 1, show_bytecode);
                println!("{}EndWhile", prefix);

                debug_dump_program(outer.inner(), lvl + 1, show_bytecode);
            }
        }
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn debug_dump_blocks(blocks: &[BasicBlock<usize, StacklessBlockContent>]) {
    for block in blocks {
        println!(
            "Block {} {:?} unconditional_loop_entry={:?}",
            block.idx, block.next, block.unconditional_loop_entry
        );

        block.content.code.iter().for_each(|bytecode| {
            println!("  {:?}", bytecode.bytecode);
        });
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn debug_dump_blocks_graph(blocks: &[BasicBlock<usize, StacklessBlockContent>]) {
    println!("digraph G {{");
    for block in blocks {
        for nxt in block.next.next_blocks() {
            println!("{} -> {}", block.idx, nxt);
        }
    }
    println!("}}");
}

fn cleanup_tail_jumps_for_terminated_blocks(
    blocks: &mut [BasicBlock<usize, StacklessBlockContent>],
) -> Result<(), anyhow::Error> {
    for block in blocks.iter_mut() {
        if matches!(block.next, Terminator::Ret | Terminator::Abort) {
            while let Some(&mut AnnotatedBytecodeData {
                bytecode: Bytecode::Jump(..),
                ..
            }) = block.content.code.last_mut().map(|x| x.inner_mut())
            {
                block.content.code.pop();
            }

            if let Some(&AnnotatedBytecodeData {
                bytecode: Bytecode::Abort(..) | Bytecode::Ret(..),
                ..
            }) = block.content.code.last().map(|x| x.inner())
            {
                // do nothing
            } else {
                return Err(anyhow::anyhow!(
                    "Terminated block not end with Ret or Abort"
                ));
            }
        }
    }

    Ok(())
}

/// Remove continue statements in the end of a loop
fn trim_continue(
    program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    in_direct_loop: bool,
) {
    let mut iter = program
        .inner_mut()
        .blocks
        .iter_mut()
        .map(to_inner_mut)
        .peekable();

    while let Some(block) = iter.next() {
        let last_in_direct_loop = in_direct_loop && iter.peek().is_none();
        match block {
            HyperBlock::ConnectedBlocks(blocks) => {
                if last_in_direct_loop {
                    if let Some(last) = blocks.last_mut() {
                        if let Terminator::Continue { .. } = last.inner().next {
                            last.inner_mut().implicit_terminator = true;
                        }
                    }
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                trim_continue(if_unit, last_in_direct_loop);
                trim_continue(else_unit, last_in_direct_loop);
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                trim_continue(inner, true);
                trim_continue(outer, last_in_direct_loop);
            }
        }
    }
}

// Remove auto-generated break/continue that is not needed
fn trim_dead_break_continue(
    program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
) {
    let program = program.inner_mut();
    while program.blocks.len() >= 2
        && program.blocks[program.blocks.len() - 2]
            .inner()
            .is_terminated_in_loop()
        && program.blocks[program.blocks.len() - 1]
            .inner()
            .is_terminated_in_loop()
        && is_empty_hyper_block(&program.blocks[program.blocks.len() - 1])
    {
        program.blocks.pop();
    }

    for block in &mut program.blocks.iter_mut().map(to_inner_mut) {
        match block {
            HyperBlock::ConnectedBlocks(blocks) => {
                while blocks.len() >= 2
                    && blocks[blocks.len() - 1].inner().content.code.is_empty()
                    && matches!(
                        blocks[blocks.len() - 1].inner().next,
                        Terminator::Break { .. } | Terminator::Continue { .. }
                    )
                    && matches!(
                        blocks[blocks.len() - 2].inner().next,
                        Terminator::Break { .. } | Terminator::Continue { .. }
                    )
                {
                    blocks.pop();
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                trim_dead_break_continue(if_unit);
                trim_dead_break_continue(else_unit);
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                trim_dead_break_continue(inner);
                trim_dead_break_continue(outer);
            }
        }
    }
}

fn is_empty_hyper_block(block: &WithMetadata<HyperBlock<usize, StacklessBlockContent>>) -> bool {
    match block.inner() {
        HyperBlock::ConnectedBlocks(blocks) => blocks.iter().all(|b| is_empty_basic_block(b)),
        HyperBlock::IfElseBlocks { .. } => false,
        HyperBlock::WhileBlocks { .. } => false,
    }
}

fn is_empty_basic_block(b: &WithMetadata<BasicBlock<usize, StacklessBlockContent>>) -> bool {
    b.inner().content.code.is_empty()
}

/// Remove blocks that has only labels and a final jump, merging the labels into the target block
fn cleanup_dummy_dispatch_blocks(
    blocks: &mut Vec<BasicBlock<usize, StacklessBlockContent>>,
) -> Result<(), anyhow::Error> {
    fn check_is_dummy_dispatch_block(
        block: &BasicBlock<usize, StacklessBlockContent>,
    ) -> Option<(Vec<Label>, Label)> {
        let mut iter = block.content.code.iter().peekable();
        let mut labels = Vec::new();

        while let Some(bytecode) = iter.next() {
            match &bytecode.bytecode {
                Bytecode::Label(_, label) => {
                    labels.push(*label);
                }
                Bytecode::Jump(_, label) => {
                    // only the last jump will be considered
                    if iter.peek().is_none() {
                        return Some((labels, *label));
                    }
                    // or it isnt be a dummy one
                    return None;
                }
                Bytecode::Call(_, _, oper, _, _) => {
                    use move_stackless_bytecode::stackless_bytecode::Operation;
                    match oper {
                        // currently only Drop|Release operations have no affect to the control flow
                        Operation::Drop | Operation::Release => {}
                        _ => return None,
                    }
                }
                _ => {
                    return None;
                }
            }
        }
        None
    }

    while let Some((from_block, to_block)) = {
        let mut next_merge = None;
        for i in 0..blocks.len() {
            if let Some(_) = check_is_dummy_dispatch_block(&blocks[i]) {
                if let Terminator::Branch { target } = blocks[i].next {
                    if target == i {
                        return Err(anyhow::anyhow!("Found self-loop branch"));
                    }
                    next_merge = Some((i, target));
                    break;
                } else {
                    panic!("Unexpected terminator");
                    // return Err(anyhow::anyhow!("Unexpected terminator"));
                }
            }
        }

        next_merge
    } {
        // pop the jump
        blocks[from_block].content.code.pop();

        // pop until the last label (other bytecodes can be discarded as checked above)
        while let Some(bytecode) = blocks[from_block].content.code.last() {
            if let Bytecode::Label(..) = bytecode.bytecode {
                break;
            } else {
                blocks[from_block].content.code.pop();
            }
        }

        // prepend the labels to the target block
        let mut target_block = blocks[to_block].content.code.clone();
        target_block.splice(0..0, blocks[from_block].content.code.clone());
        blocks[to_block].content.code = target_block;

        // update the jump target
        for block in blocks.iter_mut() {
            match block.next {
                Terminator::Branch { mut target } => {
                    if target == from_block {
                        target = to_block;
                    }
                    block.next = Terminator::Branch { target };
                }

                Terminator::IfElse {
                    mut if_block,
                    mut else_block,
                } => {
                    if if_block == from_block {
                        if_block = to_block;
                    }
                    if else_block == from_block {
                        else_block = to_block;
                    }
                    block.next = Terminator::IfElse {
                        if_block,
                        else_block,
                    };
                }

                Terminator::Break { .. }
                | Terminator::Continue { .. }
                | Terminator::While { .. } => {
                    panic!("Must not have loop-related terminators in this stage");
                }

                Terminator::Normal | Terminator::Ret | Terminator::Abort => {}
            }
        }

        algo::blocks::remove_block(blocks, from_block);
    }

    Ok(())
}

fn is_empty_program(program: &CodeUnitBlock<usize, StacklessBlockContent>) -> bool {
    program
        .content_iter()
        .all(|content| content.content.code.is_empty())
}

/// Move else block out of IfElse statement when the if block is terminated
///  or the if is last statement and body is empty: if (x) { } else { ... } -> if (x) continue; ...
/// There is an exception for now, assert!(cond, value)
///   will be compiled to if (cond) { } else { abort(value) }
///   keep the else block in this case so future ast optimizer can remove the if statement
fn trim_else(
    program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    in_loop: Option<usize>,
    is_last_block_in_loop: bool,
) {
    let mut new_blocks = Vec::new();
    let mut iter = program.inner_mut().blocks.iter_mut().peekable();

    while let Some(block) = iter.next() {
        let current_is_last_block_in_loop = is_last_block_in_loop && iter.peek().is_none();
        match block.inner_mut() {
            HyperBlock::ConnectedBlocks(_) => {
                new_blocks.push(block.clone());
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                trim_else(if_unit.as_mut(), in_loop, current_is_last_block_in_loop);
                trim_else(else_unit.as_mut(), in_loop, current_is_last_block_in_loop);

                let (rewrite, add_continue) = {
                    let mut r = false;
                    let mut c = false;
                    // this pattern is for a non-reconstructable while(complex_condition){} loop
                    if current_is_last_block_in_loop
                        && matches!(
                            if_unit.inner().terminator(),
                            Some(&Terminator::Continue { .. })
                        )
                        && matches!(
                            else_unit.inner().terminator(),
                            Some(&Terminator::Break { .. })
                        )
                    {
                        // do nothing
                    } else if if_unit.inner().is_terminated() {
                        r = true;
                    } else if in_loop.is_some() {
                        if if_unit.inner().is_terminated_in_loop() {
                            r = true;
                        } else if current_is_last_block_in_loop && is_empty_program(if_unit.inner())
                        {
                            r = true;
                            c = true;
                        }
                    };

                    if r {
                        if else_unit.is_abort() {
                            r = false;
                        }
                    }

                    (r, c)
                };

                if rewrite {
                    let mut new_t = if_unit.clone();
                    if add_continue {
                        let mut new_block: BasicBlock<usize, StacklessBlockContent> =
                            BasicBlock::default();

                        new_block.next = Terminator::Continue {
                            target: in_loop.unwrap(),
                        };

                        new_t.inner_mut().blocks.push(
                            HyperBlock::ConnectedBlocks(vec![new_block.with_metadata()])
                                .with_metadata(),
                        );
                    }

                    new_blocks.push(
                        HyperBlock::IfElseBlocks {
                            if_unit: new_t,
                            else_unit: Box::new(
                                CodeUnitBlock {
                                    blocks: Vec::new(),
                                    terminate: false,
                                }
                                .with_metadata(),
                            ),
                        }
                        .with_metadata(),
                    );

                    new_blocks.extend(else_unit.inner().blocks.iter().cloned());
                } else {
                    new_blocks.push(block.clone());
                }
            }
            HyperBlock::WhileBlocks {
                inner,
                outer,
                start_block,
                ..
            } => {
                trim_else(inner.as_mut(), Some(*start_block), true);
                trim_else(outer.as_mut(), in_loop, current_is_last_block_in_loop);
                new_blocks.push(block.clone());
            }
        }
    }
    program.inner_mut().blocks = new_blocks;
}

fn collect_starting_labels(content: &StacklessBlockContent, labels: &mut BTreeSet<Label>) -> bool {
    for bytecode in &content.code {
        if let Bytecode::Label(_, label) = bytecode.bytecode {
            labels.insert(label);
        } else {
            return true;
        }
    }
    false
}

fn apply_short_circuit_jumps(
    program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
) {
    for block in program.blocks.iter_mut() {
        match block.inner_mut() {
            HyperBlock::ConnectedBlocks(blocks) => {
                for block in blocks.iter_mut() {
                    if let Some((content, next)) = block.short_circuit_terminator.clone() {
                        if let Some(op) = block.content.code.last() {
                            if op.removed {
                                continue;
                            }

                            if matches!(op.bytecode, Bytecode::Jump(..)) {
                                block.content.code.pop();

                                block.content.code.extend(content.code.iter().cloned());

                                block.next = next.clone();
                                block.short_circuit_terminator = None;
                            }
                        }
                    }
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                apply_short_circuit_jumps(if_unit);
                apply_short_circuit_jumps(else_unit);
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                apply_short_circuit_jumps(inner);
                apply_short_circuit_jumps(outer);
            }
        }
    }
}

fn cleanup_labels(program: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>) {
    let program = program.inner_mut();
    let mut live_labels = BTreeSet::new();
    for block in program.content_iter() {
        for bytecode in &block.content.code {
            if bytecode.removed {
                continue;
            }

            if let Bytecode::Jump(_, label) = bytecode.bytecode {
                live_labels.insert(label);
            }
        }
    }

    for block in program.content_iter_mut() {
        for bytecode in &mut block.content.code {
            if let Bytecode::Label(_, label) = bytecode.bytecode {
                if !live_labels.contains(&label) {
                    bytecode.removed = true;
                }
            }
        }
    }
}

/// Remove trivial jumps wrt current control flow
fn cleanup_jumps(
    blocks: &mut Vec<WithMetadata<HyperBlock<usize, StacklessBlockContent>>>,
    next_labels: &BTreeSet<Label>,
    loop_start_labels: &BTreeSet<Label>,
    loop_exit_labels: &BTreeSet<Label>,
) {
    let mut iter = blocks.iter_mut().map(to_inner_mut).peekable();
    while let Some(block) = iter.next() {
        let labels = if let Some(next_block) = iter.peek() {
            // if we have next block, the tail labels are from it
            let mut labels = BTreeSet::new();
            for basic_block in next_block.content_iter() {
                if !collect_starting_labels(&basic_block.content, &mut labels) {
                    break;
                }
            }

            labels
        } else {
            // no next block, next labels are from parent
            next_labels.clone()
        };

        match block {
            HyperBlock::ConnectedBlocks(basic_blocks) => {
                let mut basic_iter = basic_blocks.iter_mut().map(to_inner_mut).peekable();
                while let Some(basic_block) = basic_iter.next() {
                    cleanup_loop_jumps_in_basic_block_for_labels(
                        basic_block,
                        loop_start_labels,
                        loop_exit_labels,
                    );
                    if let Some(next_basic_block) = basic_iter.peek() {
                        let mut next_labels = BTreeSet::new();
                        collect_starting_labels(&next_basic_block.content, &mut next_labels);
                        cleanup_tail_jump_in_basic_block_for_labels(
                            basic_block,
                            &next_labels,
                            &loop_start_labels,
                        );
                    } else {
                        // final block, we'll cleanup the labels sent from parent
                        cleanup_tail_jump_in_basic_block_for_labels(
                            basic_block,
                            &labels,
                            &loop_start_labels,
                        );
                    }
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                // t & f are disjoint, so we need to cleanup both
                cleanup_jumps(
                    &mut if_unit.inner_mut().blocks,
                    &labels,
                    loop_start_labels,
                    loop_exit_labels,
                );
                cleanup_jumps(
                    &mut else_unit.inner_mut().blocks,
                    &labels,
                    loop_start_labels,
                    loop_exit_labels,
                );
            }

            HyperBlock::WhileBlocks {
                inner,
                outer,
                start_block,
                exit_block,
                ..
            } => {
                let inner_loop_exit_labels = BTreeSet::from([Label::new(*exit_block)]);
                let inner_loop_labels = BTreeSet::from([Label::new(*start_block)]);
                cleanup_jumps(
                    &mut inner.inner_mut().blocks,
                    &BTreeSet::new(),
                    &inner_loop_labels,
                    &inner_loop_exit_labels,
                );
                cleanup_jumps(
                    &mut outer.inner_mut().blocks,
                    &labels,
                    loop_start_labels,
                    loop_exit_labels,
                );
            }
        }
    }
}

fn cleanup_loop_jumps_in_basic_block_for_labels(
    block: &mut BasicBlock<usize, StacklessBlockContent>,
    loop_labels: &BTreeSet<Label>,
    loop_exit_labels: &BTreeSet<Label>,
) {
    let relevant_labels = match block.next {
        Terminator::Continue { .. } => loop_labels,
        Terminator::Break { .. } => loop_exit_labels,
        _ => return,
    };

    if let Some(bytecode) = block.content.code.last_mut() {
        if let Bytecode::Jump(_, label) = bytecode.bytecode {
            if relevant_labels.contains(&label) {
                bytecode.removed = true;
            }
        }
    }
}

fn cleanup_tail_jump_in_basic_block_for_labels(
    block: &mut BasicBlock<usize, StacklessBlockContent>,
    labels: &BTreeSet<Label>,
    loop_labels: &BTreeSet<Label>,
) {
    if let Terminator::Branch { .. } | Terminator::Continue { .. } | Terminator::Break { .. } =
        block.next
    {
        if let Some(bytecode) = block.content.code.last_mut() {
            if let Bytecode::Jump(_, label) = bytecode.bytecode {
                if loop_labels.contains(&label) || labels.contains(&label) {
                    bytecode.removed = true;
                }
            }
        }
    }
}

fn build_program(
    global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    blocks: core::slice::Iter<BasicBlock<usize, StacklessBlockContent>>,
    skip_first_unconditional_loop: bool,
) -> Result<WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>, anyhow::Error> {
    let mut p = CodeUnitBlock {
        blocks: Vec::new(),
        terminate: false,
    }
    .with_metadata();

    let mut chaining_blocks = Vec::new();
    fn flush(
        chaining_blocks: &mut Vec<WithMetadata<BasicBlock<usize, StacklessBlockContent>>>,
        p: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    ) {
        if !chaining_blocks.is_empty() {
            p.inner_mut()
                .blocks
                .push(HyperBlock::ConnectedBlocks(chaining_blocks.clone()).with_metadata());
            chaining_blocks.clear();
        }
    }

    let mut iter = blocks;
    let mut first_node = true;
    loop {
        let backup_iter = iter.clone();
        if let Some(node) = iter.next() {
            if let (true, Some((exit, _))) = (
                (!first_node || !skip_first_unconditional_loop),
                &node.unconditional_loop_entry,
            ) {
                flush(&mut chaining_blocks, &mut p);
                iter = backup_iter.clone();
                let paths = follow_loop_boundaries(
                    global_blocks,
                    &mut iter,
                    node.idx,
                    node.idx,
                    *exit,
                    true,
                )?;
                p.inner_mut().blocks.push(paths);
            } else {
                match node.next {
                    Terminator::Normal => {
                        return Err(anyhow::anyhow!(
                            "There must be no Normal node at this stage"
                        ));
                    }

                    Terminator::Branch { .. } => {
                        chaining_blocks.push(node.clone().with_metadata());
                    }

                    Terminator::Break { .. } | Terminator::Continue { .. } => {
                        chaining_blocks.push(node.clone().with_metadata());
                        flush(&mut chaining_blocks, &mut p);
                    }

                    Terminator::Ret | Terminator::Abort => {
                        chaining_blocks.push(node.clone().with_metadata());
                        flush(&mut chaining_blocks, &mut p);
                        p.inner_mut().terminate = true;
                    }

                    Terminator::IfElse {
                        if_block,
                        else_block,
                    } => {
                        chaining_blocks.push(node.clone().with_metadata());
                        flush(&mut chaining_blocks, &mut p);
                        let paths = follow_ifelse_boundaries(
                            global_blocks,
                            &mut iter,
                            if_block,
                            else_block,
                        )?;
                        p.inner_mut().blocks.push(paths);
                    }

                    Terminator::While {
                        inner_block,
                        outer_block,
                        ..
                    } => {
                        chaining_blocks.push(node.clone().with_metadata());
                        flush(&mut chaining_blocks, &mut p);
                        let paths = follow_loop_boundaries(
                            global_blocks,
                            &mut iter,
                            node.idx,
                            inner_block,
                            outer_block,
                            false,
                        )?;
                        p.inner_mut().blocks.push(paths);
                    }
                }
            }
        } else {
            break;
        }

        first_node = false;
    }

    flush(&mut chaining_blocks, &mut p);

    Ok(p)
}

fn follow_loop_boundaries(
    global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    iter: &mut core::slice::Iter<BasicBlock<usize, StacklessBlockContent>>,
    start: usize,
    inner: usize,
    outer: usize,
    unconditional: bool,
) -> Result<WithMetadata<HyperBlock<usize, StacklessBlockContent>>, anyhow::Error> {
    let mut inner_nodes = Vec::new();
    let mut outer_nodes = Vec::new();

    let mut inner_paths = BTreeSet::from([inner]);
    let mut outer_paths = BTreeSet::from([outer]);

    while let Some(next_block) = iter.next() {
        match (
            inner_paths.get(&next_block.idx),
            outer_paths.get(&next_block.idx),
        ) {
            (Some(_), _) => {
                inner_nodes.push(next_block.clone());
                if let Terminator::Break { target } | Terminator::Continue { target } =
                    next_block.next
                {
                    if target == outer {
                        outer_paths.insert(target);
                    } else {
                        inner_paths.insert(target);
                    };
                } else {
                    inner_paths.extend(next_block.next.next_blocks().iter().copied());
                }
            }
            (_, Some(_)) => {
                outer_nodes.push(next_block.clone());
                outer_paths.extend(next_block.next.next_blocks().iter().copied());
            }
            _ => {}
        }
    }

    let inner_program = build_program(global_blocks, inner_nodes.iter(), unconditional)?;
    let outer_program = build_program(global_blocks, outer_nodes.iter(), false)?;

    Ok(HyperBlock::WhileBlocks {
        inner: Box::new(inner_program),
        outer: Box::new(outer_program),
        unconditional,
        start_block: start,
        exit_block: outer,
    }
    .with_metadata())
}

fn follow_ifelse_boundaries(
    global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    iter: &mut core::slice::Iter<BasicBlock<usize, StacklessBlockContent>>,
    t: usize,
    f: usize,
) -> Result<WithMetadata<HyperBlock<usize, StacklessBlockContent>>, anyhow::Error> {
    let mut true_nodes = Vec::new();
    let mut false_nodes = Vec::new();

    let mut true_paths = BTreeSet::from([t]);
    let mut false_paths = BTreeSet::from([f]);

    let mut first_branch = None;

    loop {
        let backup_iter = iter.clone();
        if let Some(n) = iter.next() {
            let in_true_path = true_paths.get(&n.idx).is_some();
            let in_false_path = false_paths.get(&n.idx).is_some();

            if in_true_path && in_false_path {
                // both paths are merged
                let min_intersection = *true_paths.intersection(&false_paths).next().unwrap();
                if min_intersection != n.idx {
                    // this function's input is already topo sorted, so this should not happen
                    return Err(anyhow::anyhow!(
                        "Both paths are merged at {}, but the current node is {}",
                        min_intersection,
                        n.idx
                    ));
                };
                // this instruction is not in the if-else structure, we need to rollback
                *iter = backup_iter;
                break;
            }

            if in_true_path {
                if first_branch.is_none() {
                    first_branch = Some(true);
                }
                true_nodes.push(n.clone());
                true_paths.extend(n.next.next_blocks().iter().copied());
            }
            if in_false_path {
                if !in_true_path {
                    if first_branch.is_none() {
                        first_branch = Some(false);
                    }
                    false_nodes.push(n.clone());
                }

                false_paths.extend(n.next.next_blocks().iter().copied());
            }
        } else {
            break;
        }
    }

    let mut true_program = build_program(global_blocks, true_nodes.iter(), false)?;
    let mut false_program = build_program(global_blocks, false_nodes.iter(), false)?;

    if true_program.inner().blocks.is_empty() {
        true_program = program_branch_to(global_blocks, t);
    }

    if false_program.inner().blocks.is_empty() {
        false_program = program_branch_to(global_blocks, f);
    }

    Ok(HyperBlock::IfElseBlocks {
        if_unit: Box::new(true_program),
        else_unit: Box::new(false_program),
    }
    .with_metadata())
}

fn program_branch_to(
    _global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    target: usize,
) -> WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>> {
    let mut p = CodeUnitBlock {
        blocks: Vec::new(),
        terminate: false,
    }
    .with_metadata();
    let mut block: BasicBlock<usize, StacklessBlockContent> = BasicBlock::default();
    block.content.code.push(
        AnnotatedBytecodeData {
            original_offset: usize::MAX,
            bytecode: Bytecode::Jump(AttrId::new(u16::MAX as usize), Label::new(target)),
            jump_type: JumpType::Unknown,
            removed: false,
        }
        .with_metadata(),
    );
    block.next = Terminator::Branch { target };
    p.inner_mut()
        .blocks
        .push(HyperBlock::ConnectedBlocks(vec![block.with_metadata()]).with_metadata());

    p
}

fn build_terminated_program(
    global_blocks: &[BasicBlock<usize, StacklessBlockContent>],
    target: usize,
) -> Option<WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>> {
    if global_blocks[target].next.is_terminated() {
        let mut p = CodeUnitBlock {
            blocks: Vec::new(),
            terminate: false,
        }
        .with_metadata();
        p.inner_mut().blocks.push(
            HyperBlock::ConnectedBlocks(vec![global_blocks[target].clone().with_metadata()])
                .with_metadata(),
        );

        Some(p)
    } else {
        if let Terminator::IfElse {
            if_block,
            else_block,
        } = &global_blocks[target].next
        {
            let Some(if_program) = build_terminated_program(global_blocks, *if_block) else {
                return None;
            };
            let Some(else_program) = build_terminated_program(global_blocks, *else_block) else {
                return None;
            };
            let mut p = CodeUnitBlock {
                blocks: Vec::new(),
                terminate: false,
            }
            .with_metadata();
            p.inner_mut().blocks.push(
                HyperBlock::ConnectedBlocks(vec![global_blocks[target].clone().with_metadata()])
                    .with_metadata(),
            );
            p.inner_mut().blocks.push(
                HyperBlock::IfElseBlocks {
                    if_unit: Box::new(if_program),
                    else_unit: Box::new(else_program),
                }
                .with_metadata(),
            );

            Some(p)
        } else {
            None
        }
    }
}
