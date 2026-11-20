// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Builds a function body as a sequence of basic blocks.
//!
//! It decides how many blocks to emit and how many instructions each holds, ends each block with
//! a branch or a return, and keeps the stack and locals consistent where control flow joins. A
//! body is straight-line code, a bounded loop, or (only when breaking the control-flow rule on
//! purpose) an irreducible loop; see [`CfgShape`].

use crate::{
    abilities::has_drop,
    config::DimensionFlag,
    error::Result,
    instructions::{
        ability_ctx, imm_instr, label_instr, name_instr, pick_key_struct, pick_one,
        pick_one_loop_safe, simple_op, struct_instr,
    },
    ir::{Block, BlockId, Instr, Operand, Type, Value},
    state::{FunctionState, LocalInit},
    violations,
};
use arbitrary::Unstructured;

/// Shape of the function's CFG, decided once at the start of body generation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum CfgShape {
    /// Forward edges only.
    Linear,
    /// Bounded-counter loop with self-back-edge at block 1 (the loop header). Block 0 is
    /// the preheader that initializes the counter; blocks 2..N-1 follow normally. The loop body
    /// cannot change the counter or which locals are initialized.
    ReducibleLoop,
    /// A four-block irreducible CFG (block 0 conditionally targets block 2 or falls through
    /// to block 1; block 1 branches to block 2; block 2 conditionally back-edges to block 1
    /// or falls through to block 3 — block 1 doesn't dominate block 2). Used only under
    /// `valid_control_flow = false`.
    Irreducible,
}

/// Upper bound on the dynamic iteration count of any generated bounded-counter loop. The
/// counter is initialized to a value in `0..=LOOP_ITER_MAX`, so the loop runs at most
/// `LOOP_ITER_MAX` times. Kept small so VM execution stays fast.
const LOOP_ITER_MAX: u64 = 16;

/// Entry point: generate the basic-block body for the current function. Block 0 is the
/// entry. Each non-final block ends in a terminator appropriate to the chosen [`CfgShape`]:
/// for the linear shape, a forward `branch`/`br_true`/`br_false`; for the reducible-loop
/// shape, a self-back-edge from the loop header; for the irreducible shape, a back-edge
/// from block 2 to block 1.
pub(crate) fn build_body(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
) -> Result<Vec<Block>> {
    let max_blocks = state.cfg.blocks_per_fun_max.max(1);
    let mut block_count = u.int_in_range(1..=max_blocks)?;
    // Per-function instruction budget, distributed roughly evenly across blocks.
    let total_budget = u.int_in_range(0..=state.cfg.instrs_per_fun_max)?;

    let shape = plan_cfg_shape(u, state, &mut block_count)?;
    let per_block = total_budget / (block_count as u16).max(1);

    let mut blocks: Vec<Block> = Vec::with_capacity(block_count as usize);
    for block_idx in 0..block_count {
        let block_id = BlockId(block_idx as u32);
        state.record_block_entry(block_id);

        let mut instrs: Vec<Instr> = Vec::new();
        let is_last = block_idx + 1 == block_count;

        match (shape, block_idx) {
            (CfgShape::ReducibleLoop, 0) => {
                // Preheader: user body, drain, initialize the counter, branch to the loop
                // header (block 1).
                fill_block(u, state, &mut instrs, per_block)?;
                emit_loop_preheader_terminator(u, state, &mut instrs, block_id, BlockId(1))?;
            },
            (CfgShape::ReducibleLoop, 1) => {
                // Loop header: bounded-counter check, optional state-preserving user body,
                // self-back-edge. The user-body budget is half of `per_block` to give the
                // counter/back-edge scaffolding room.
                let loop_body_budget = per_block / 2;
                emit_loop_header_block(
                    u,
                    state,
                    &mut instrs,
                    BlockId(1),
                    BlockId(2),
                    loop_body_budget,
                )?;
            },
            (CfgShape::Irreducible, 0) => {
                fill_block(u, state, &mut instrs, per_block)?;
                emit_two_way_branch(
                    u,
                    state,
                    &mut instrs,
                    block_id,
                    /* taken = */ BlockId(2),
                    /* fallthrough = */ BlockId(1),
                )?;
            },
            (CfgShape::Irreducible, 1) => {
                fill_block(u, state, &mut instrs, per_block)?;
                emit_unconditional_branch(state, &mut instrs, block_id, BlockId(2))?;
            },
            (CfgShape::Irreducible, 2) => {
                fill_block(u, state, &mut instrs, per_block)?;
                // The back-edge to block 1 — block 1 doesn't dominate block 2, so this
                // makes the CFG irreducible. Fall-through to block 3 (the return block).
                emit_two_way_branch(
                    u,
                    state,
                    &mut instrs,
                    block_id,
                    /* taken = */ BlockId(1),
                    /* fallthrough = */ BlockId(3),
                )?;
            },
            _ => {
                fill_block(u, state, &mut instrs, per_block)?;
                if is_last {
                    emit_final_terminator(u, state, &mut instrs)?;
                } else {
                    emit_branch_terminator(u, state, &mut instrs, block_idx, block_count)?;
                }
            },
        }

        blocks.push(Block {
            id: block_id,
            instrs,
        });
    }

    Ok(blocks)
}

/// Selects an irreducible CFG for an injected control-flow violation, setting `block_count`
/// to four. Otherwise may choose a reducible loop when a counter local is available,
/// falling back to a linear CFG.
fn plan_cfg_shape(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    block_count: &mut u8,
) -> Result<CfgShape> {
    if *block_count < 4 {
        return Ok(CfgShape::Linear);
    }
    if state.should_violate(u, DimensionFlag::ControlFlow)? {
        *block_count = 4;
        return Ok(CfgShape::Irreducible);
    }
    if state.cfg.enable_loops && state.loop_counter_idx.is_some() && u.arbitrary::<bool>()? {
        return Ok(CfgShape::ReducibleLoop);
    }
    Ok(CfgShape::Linear)
}

/// Runs up to `per_block` successful instruction generators, each of which may emit multiple
/// instructions. A retry limit bounds input consumption when generators reject the current state.
fn fill_block(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    per_block: u16,
) -> Result<()> {
    let mut applied: u16 = 0;
    let mut tries: u16 = 0;
    let max_tries = per_block.saturating_mul(4).max(8);
    while applied < per_block && tries < max_tries {
        tries += 1;
        if pick_one(u, state, instrs)? {
            applied += 1;
        }
    }
    Ok(())
}

/// Emit the terminator for a non-final block. The block is drained to entry depth first;
/// then we choose between an unconditional `branch` and a conditional `br_true`/`br_false`
/// (which targets the join block while the fall-through goes to the next block in layout
/// order). For each successor we merge the current state into that successor's recorded
/// entry state so the verifier's abstract interpreter (and our own local generators) see
/// the conservative meet of all incoming paths.
fn emit_branch_terminator(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    block_idx: u8,
    block_count: u8,
) -> Result<()> {
    drain_to_block_entry(state, instrs, BlockId(block_idx as u32))?;

    let next_id = BlockId(block_idx as u32 + 1);
    let last_id = BlockId((block_count - 1) as u32);

    // A conditional skips ahead to the last block, so the fall-through must not be that block.
    // Block 0 always may; later blocks may when guarded reads are enabled, so more blocks can
    // read a resource known to exist.
    let skips_ahead = block_idx + 2 < block_count;
    let kind = if skips_ahead && (block_idx == 0 || state.cfg.guarded_read_prob > 0) {
        u.int_in_range(0u8..=2)?
    } else {
        0
    };

    match kind {
        1 | 2 => {
            // A guarded read: skip to the join block unless the resource exists, so the
            // fall-through block can read it.
            let guard = if kind == 2 && violations::chance(u, state.cfg.guarded_read_prob)? {
                crate::instructions::push_exists_guard(u, state, instrs)?
            } else {
                None
            };
            if guard.is_none() {
                let b: bool = u.arbitrary()?;
                instrs.push(simple_op(if b { "ld_true" } else { "ld_false" }));
                state.push(Type::Bool);
            }
            state.next_guaranteed_resource = guard;
            let opcode = if kind == 1 { "br_true" } else { "br_false" };
            state.pop();
            instrs.push(label_instr(opcode, last_id));
            // Successors: the conditional target plus the fall-through to the next block.
            state.merge_into_successor(last_id);
            state.merge_into_successor(next_id);
        },
        _ => {
            instrs.push(label_instr("branch", next_id));
            state.merge_into_successor(next_id);
        },
    }
    Ok(())
}

/// Pops values above `block_id`'s entry stack depth before a branch terminator.
/// Returns `GenerationStuck` if a value cannot be dropped.
fn drain_to_block_entry(
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    block_id: BlockId,
) -> Result<()> {
    let entry_depth = state
        .block_entry_depths
        .get(&block_id)
        .copied()
        .unwrap_or(0);
    while state.stack_len() > entry_depth {
        let top = state.top().cloned().expect("non-empty stack");
        let ctx = ability_ctx(state);
        if !has_drop(&top, &ctx) {
            return Err(crate::error::Error::GenerationStuck(
                "non-droppable value left on stack at branch terminator",
            ));
        }
        state.pop();
        instrs.push(simple_op("pop"));
    }
    Ok(())
}

/// Emit the final-block terminator. The function may exit via `ret` (matching the declared
/// return signature) or via `abort` (popping a `u64` abort code). Under
/// `valid_stack_balance = false` the injector may skip the drain+push step, leaving whatever
/// happens to be on the stack at function exit — the verifier's `stack_usage_verifier`
/// rejects.
fn emit_final_terminator(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<()> {
    let cfg = state.cfg;
    let violate_balance = violations::wants_violation(u, cfg, DimensionFlag::StackBalance)?;

    // Leave the stack untouched and emit `ret`. This only violates stack balance when the
    // stack differs from the declared return types; otherwise it is a valid return and
    // falls through to the normal exit.
    if violate_balance && state.ty_stack != state.return_types {
        state.record_violation(DimensionFlag::StackBalance);
        instrs.push(simple_op("ret"));
        return Ok(());
    }

    // Every leftover value must have `drop` before the return values can be synthesized.
    while let Some(top) = state.top().cloned() {
        let ctx = ability_ctx(state);
        if !has_drop(&top, &ctx) {
            return Err(crate::error::Error::GenerationStuck(
                "non-droppable value left on the stack at function exit",
            ));
        }
        state.pop();
        instrs.push(simple_op("pop"));
    }

    // Pick between `abort` and `ret`. `abort` works regardless of the declared return
    // signature (it doesn't reach the function exit), so it's always available; `ret`
    // requires us to first push values matching the declared returns. An abort discards
    // every write of the transaction, so it is kept rare enough for state to persist.
    let use_abort = violations::chance(u, cfg.final_abort_prob)?;
    if use_abort {
        let code: u64 = u.arbitrary()?;
        instrs.push(imm_instr("ld_u64", Value::U64(code)));
        state.push(Type::U64);
        state.pop();
        instrs.push(simple_op("abort"));
    } else {
        let return_types = state.return_types.clone();
        for ty in &return_types {
            crate::instructions::push_synthesized_value(u, state, instrs, ty)?;
        }
        instrs.push(simple_op("ret"));
    }
    Ok(())
}

/// Emit the preheader's terminator: drain, initialize the bounded counter with a small
/// random value, and unconditionally branch to the loop header. After this, the counter
/// slot is [`LocalInit::Available`] and the operand stack is empty.
fn emit_loop_preheader_terminator(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    self_id: BlockId,
    header: BlockId,
) -> Result<()> {
    drain_to_block_entry(state, instrs, self_id)?;
    let counter_idx = state
        .loop_counter_idx
        .expect("reducible loop requires a counter local");
    let counter_name = state.local(counter_idx).expect("slot").name.clone();
    let iters: u64 = u.int_in_range(0..=LOOP_ITER_MAX)?;
    instrs.push(imm_instr("ld_u64", Value::U64(iters)));
    state.push(Type::U64);
    instrs.push(name_instr("st_loc", counter_name));
    state.pop();
    state.local_mut(counter_idx).expect("slot").init = LocalInit::Available;
    instrs.push(label_instr("branch", header));
    state.merge_into_successor(header);
    Ok(())
}

/// Emit the loop header block in full: the loop control below, with an optional body of
/// loop-safe instructions between the decrement and the back edge:
///
/// ```text
///   copy_loc __loop_counter  ; push counter
///   ld_u64 0
///   gt                       ; counter > 0
///   br_false <exit>          ; jump out when counter == 0
///   copy_loc __loop_counter
///   ld_u64 1
///   sub                      ; counter - 1
///   st_loc __loop_counter
///   branch <self>            ; back-edge (header dominates itself, so this is reducible)
/// ```
///
/// At the back-edge point the counter slot is still `Available` and the stack is empty,
/// matching this block's recorded entry exactly — so the meet at the back-edge target is a
/// no-op and the verifier's abstract-state propagation reaches the fixed-point trivially.
fn emit_loop_header_block(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    self_id: BlockId,
    exit_id: BlockId,
    body_budget: u16,
) -> Result<()> {
    let counter_idx = state
        .loop_counter_idx
        .expect("reducible loop requires a counter local");
    let counter_name = state.local(counter_idx).expect("slot").name.clone();

    // copy_loc counter; ld_u64 0; gt
    instrs.push(name_instr("copy_loc", counter_name.clone()));
    state.push(Type::U64);
    instrs.push(imm_instr("ld_u64", Value::U64(0)));
    state.push(Type::U64);
    instrs.push(simple_op("gt"));
    state.pop();
    state.pop();
    state.push(Type::Bool);

    // br_false <exit>
    instrs.push(label_instr("br_false", exit_id));
    state.pop();
    state.merge_into_successor(exit_id);

    // copy_loc counter; ld_u64 1; sub; st_loc counter
    instrs.push(name_instr("copy_loc", counter_name.clone()));
    state.push(Type::U64);
    instrs.push(imm_instr("ld_u64", Value::U64(1)));
    state.push(Type::U64);
    instrs.push(simple_op("sub"));
    state.pop();
    state.pop();
    state.push(Type::U64);
    instrs.push(name_instr("st_loc", counter_name));
    state.pop();
    // Counter is still `Available` — st_loc of an already-Available slot keeps it
    // Available, which is exactly what the self-back-edge needs.

    // Optional state-preserving user body. The loop-safe dispatcher emits only
    // operations that don't mutate `LocalInit` (no move_loc / st_loc / borrow_loc) and
    // can be drained to depth 0 (all values droppable). Stack/locals state therefore
    // matches the block-entry state — making the back-edge meet trivial.
    if body_budget > 0 {
        let mut applied: u16 = 0;
        let mut tries: u16 = 0;
        let max_tries = body_budget.saturating_mul(4).max(8);
        while applied < body_budget && tries < max_tries {
            tries += 1;
            if pick_one_loop_safe(u, state, instrs)? {
                applied += 1;
            }
        }
        // Drain the body back to an empty stack, checking the loop-safe set's drop invariant.
        while let Some(top) = state.top().cloned() {
            let ctx = ability_ctx(state);
            if !has_drop(&top, &ctx) {
                return Err(crate::error::Error::GenerationStuck(
                    "loop-body drain: non-droppable value left on stack",
                ));
            }
            state.pop();
            instrs.push(simple_op("pop"));
        }
    }

    // branch <self> (back-edge — reducible because the header dominates itself)
    instrs.push(label_instr("branch", self_id));
    state.merge_into_successor(self_id);

    Ok(())
}

/// Emit a conditional `br_true` with a literal bool on the operand stack, drained to the
/// block's recorded entry depth first. Records both successors. Used by the irreducible
/// shape for blocks 0 and 2 — block 0's conditional jump to block 2, and block 2's back-edge
/// to block 1 (with fall-through to the return block as the loop exit).
fn emit_two_way_branch(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    self_id: BlockId,
    taken: BlockId,
    fallthrough: BlockId,
) -> Result<()> {
    drain_to_block_entry(state, instrs, self_id)?;
    let b: bool = u.arbitrary()?;
    instrs.push(simple_op(if b { "ld_true" } else { "ld_false" }));
    state.push(Type::Bool);
    instrs.push(label_instr("br_true", taken));
    state.pop();
    state.merge_into_successor(taken);
    state.merge_into_successor(fallthrough);
    Ok(())
}

/// Emit an unconditional `branch` to `target`, draining the stack first. Records the
/// single successor. Used by the irreducible shape for block 1's forward edge to block 2.
fn emit_unconditional_branch(
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    self_id: BlockId,
    target: BlockId,
) -> Result<()> {
    drain_to_block_entry(state, instrs, self_id)?;
    instrs.push(label_instr("branch", target));
    state.merge_into_successor(target);
    Ok(())
}

/// Splice a forced `ld_const<address>; borrow_global<K>; pop` triple into the entry block
/// (just before its terminator). Used by [`crate::functions::build_function_body`] when the
/// signature pass dropped the function's `acquires` annotation under
/// `valid_acquires = false` — without an actual global op, the missing annotation wouldn't
/// trip the acquires verifier.
pub(crate) fn append_forced_global_op(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    body: &mut [Block],
) -> Result<()> {
    let Some((key_struct, key_type_args)) = pick_key_struct(u, state)? else {
        return Ok(());
    };
    let Some(first_block) = body.first_mut() else {
        return Ok(());
    };
    let term_pos = first_block.instrs.len().saturating_sub(1);
    let addr = crate::types::pick_account_address(u)?;
    let injected = vec![
        Instr {
            opcode: "ld_const".to_string(),
            operands: vec![
                Operand::Type(Type::Address),
                Operand::Imm(Value::Address(addr)),
            ],
        },
        struct_instr("borrow_global", key_struct.clone(), key_type_args),
        // Drop the ref so the block ends with the same stack as before our injection.
        simple_op("pop"),
    ];
    // Operand-stack invariants are preserved before/after the splice (push address,
    // pop→ref, pop ref). We don't update `state.ty_stack` because the splice happens
    // after the body builder has already sealed the stack for the terminator.
    for (i, instr) in injected.into_iter().enumerate() {
        first_block.instrs.insert(term_pos + i, instr);
    }
    Ok(())
}
