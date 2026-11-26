// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! One generator per instruction, and the dispatcher that picks among them.
//!
//! Each `try_*` generator checks whether its instruction is valid in the current
//! [`FunctionState`]. If so, it emits the instruction, updates the state, and returns `true`;
//! otherwise it returns `false` and the dispatcher tries another.

use crate::{
    abilities::{has_copy, has_drop, type_abilities, AbilityCtx},
    borrows::{BorrowRoot, BorrowStep, Mutability},
    config::{Config, DimensionFlag},
    error::Result,
    ir::{
        BlockId, Function, Instr, Operand, QualifiedFunction, QualifiedStruct, StructDecl,
        StructLayout, Type, Value,
    },
    state::{FunctionState, LocalInit},
    types::{
        infer_type_args, is_integer, is_signer_ref, is_unsigned_integer, is_valid_type_arg,
        mentions_function, pick_account_address, pick_concrete_primitives, substitute, type_depth,
    },
};
use arbitrary::Unstructured;
use move_binary_format::file_format::Visibility;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    int256::{I256, U256},
    language_storage::ModuleId,
};

// ============================================================================
// Dispatcher
// ============================================================================

type Generator =
    fn(&mut Unstructured<'_>, &mut FunctionState<'_, '_>, &mut Vec<Instr>) -> Result<bool>;

const GENERATORS: &[Generator] = &[
    try_ld_u64,
    try_ld_u8,
    try_ld_u16,
    try_ld_u32,
    try_ld_u128,
    try_ld_u256,
    try_ld_i8,
    try_ld_i16,
    try_ld_i32,
    try_ld_i64,
    try_ld_i128,
    try_ld_i256,
    try_ld_bool,
    try_ld_address,
    try_ld_const_vector,
    try_pop,
    try_copy_loc,
    try_move_loc,
    try_st_loc,
    try_add,
    try_sub,
    try_mul,
    try_div,
    try_mod,
    try_bit_and,
    try_bit_or,
    try_xor,
    try_shl,
    try_shr,
    try_eq,
    try_neq,
    try_and,
    try_or,
    try_not,
    try_pack,
    try_unpack,
    try_pack_variant,
    try_unpack_variant,
    try_test_variant,
    try_imm_borrow_variant_field,
    try_mut_borrow_variant_field,
    try_imm_borrow_loc,
    try_mut_borrow_loc,
    try_imm_borrow_field,
    try_mut_borrow_field,
    try_read_ref,
    try_write_ref,
    try_freeze_ref,
    try_call,
    try_cast_u8,
    try_cast_u16,
    try_cast_u32,
    try_cast_u64,
    try_cast_u128,
    try_cast_u256,
    try_cast_i8,
    try_cast_i16,
    try_cast_i32,
    try_cast_i64,
    try_cast_i128,
    try_cast_i256,
    try_lt,
    try_le,
    try_gt,
    try_ge,
    try_vec_pack,
    try_vec_unpack,
    try_vec_len,
    try_vec_imm_borrow,
    try_vec_mut_borrow,
    try_vec_push_back,
    try_vec_pop_back,
    try_vec_swap,
    try_borrow_global,
    try_mut_borrow_global,
    try_move_from,
    try_move_to,
    try_exists,
    try_pack_closure,
    try_call_closure,
    crate::natives::try_call_native,
    try_instantiation_loop_call,
];

/// Global storage operations, which [`Config::global_ops_bias`] favors.
const GLOBAL_GENERATORS: &[Generator] = &[
    try_move_to,
    try_modify_resource,
    try_exists,
    try_borrow_global,
    try_mut_borrow_global,
    try_move_from,
];

/// Emits one step of a block: sometimes an in-place sequence or a storage operation, otherwise
/// the first applicable generator from a random start. Returns `false` if none applies.
pub(crate) fn pick_one(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if state.cfg.in_place_prob > 0
        && crate::violations::chance(u, state.cfg.in_place_prob)?
        && crate::in_place::try_work_in_place(u, state, instrs)?
    {
        return Ok(true);
    }
    if crate::violations::chance(u, state.cfg.global_ops_bias)? {
        // Scripts cannot touch resources directly, so they reach storage through calls.
        if try_call_storing(u, state, instrs)? {
            return Ok(true);
        }
        if try_rotating(GLOBAL_GENERATORS, u, state, instrs)? {
            return Ok(true);
        }
    }
    try_rotating(GENERATORS, u, state, instrs)
}

/// Tries the generators of `table` in order from a random start, wrapping around, until one
/// succeeds.
fn try_rotating(
    table: &[Generator],
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let start = u.int_in_range(0..=table.len() - 1)?;
    for offset in 0..table.len() {
        if table[(start + offset) % table.len()](u, state, instrs)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Subset of [`GENERATORS`] that never mutates `LocalInit` state or introduces unclosed
/// borrows. Used inside the loop-header body so the back-edge target's recorded entry state
/// (stack=[], locals init unchanged from preheader) still matches when the back-edge fires.
///
/// What's deliberately excluded:
/// - `try_move_loc`, `try_st_loc` — change locals' `Available`/`Moved`/`Uninit` lattice
///   point, breaking back-edge meet equality.
/// - `try_*_borrow_loc` / `try_*_borrow_field` / `try_*_borrow_global` / `try_read_ref` /
///   `try_write_ref` / `try_freeze_ref` — issue or consume references; if any borrow
///   remains outstanding at the back-edge, the verifier's reference-safety pass rejects.
/// - `try_pack` / `try_unpack` / variants / vector borrows — produce non-droppable values
///   or borrow-graph entries that can't be trivially drained before the back-edge.
/// - `try_call` / `try_pack_closure` / `try_call_closure` / global ops / `try_move_to` —
///   side effects on `accessed_key_structs`, ability constraints on returns, or unbounded
///   reference creation.
/// - `try_ld_const_vector` — pushes a `vector<T>` which has `drop` and so is technically
///   droppable, but introduces constant-pool corruption hooks that interact with the
///   `valid_constants` dimension flag and complicate the abstract-state invariants.
///
/// [`try_copy_loc_droppable`] is the loop-safe variant of `try_copy_loc`: it only fires for
/// locals whose type has both `copy` *and* `drop`, so the produced value can be popped at
/// loop-body drain time. Without the `drop` filter, a `T: copy`-only generic parameter
/// would produce a value the drain can't pop.
///
/// Every value left on the stack must have `drop`, so the loop body can restore an empty
/// stack before its back edge.
const LOOP_SAFE_GENERATORS: &[Generator] = &[
    try_ld_u64,
    try_ld_u8,
    try_ld_u16,
    try_ld_u32,
    try_ld_u128,
    try_ld_u256,
    try_ld_i8,
    try_ld_i16,
    try_ld_i32,
    try_ld_i64,
    try_ld_i128,
    try_ld_i256,
    try_ld_bool,
    try_ld_address,
    try_copy_loc_droppable,
    try_pop,
    try_add,
    try_sub,
    try_mul,
    try_div,
    try_mod,
    try_bit_and,
    try_bit_or,
    try_xor,
    try_shl,
    try_shr,
    try_eq,
    try_neq,
    try_and,
    try_or,
    try_not,
    try_cast_u8,
    try_cast_u64,
    try_cast_i8,
    try_cast_i16,
    try_cast_i32,
    try_cast_i64,
    try_cast_i128,
    try_cast_i256,
    try_lt,
    try_le,
    try_gt,
    try_ge,
];

/// Like [`pick_one`] but restricted to the loop-safe generator subset.
pub(crate) fn pick_one_loop_safe(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    try_rotating(LOOP_SAFE_GENERATORS, u, state, instrs)
}

// ============================================================================
// Individual generators
// ============================================================================

/// Defines generators that each call a shared helper with fixed leading arguments.
macro_rules! delegating_generators {
    ($($name:ident => $helper:ident($($arg:expr),+);)+) => {
        $(
            fn $name(
                u: &mut Unstructured<'_>,
                state: &mut FunctionState<'_, '_>,
                instrs: &mut Vec<Instr>,
            ) -> Result<bool> {
                $helper($($arg,)+ u, state, instrs)
            }
        )+
    };
}

delegating_generators! {
    try_copy_loc => try_copy_loc_where(false);
    try_copy_loc_droppable => try_copy_loc_where(true);
    try_vec_imm_borrow => try_vec_borrow_indexed(false, "vec_borrow");
    try_vec_mut_borrow => try_vec_borrow_indexed(true, "vec_mut_borrow");
    try_borrow_global => try_global_borrow(false, "borrow_global");
    try_mut_borrow_global => try_global_borrow(true, "mut_borrow_global");
    try_imm_borrow_field => try_borrow_field(false, "borrow_field");
    try_mut_borrow_field => try_borrow_field(true, "mut_borrow_field");
    try_imm_borrow_variant_field => try_borrow_variant_field(false, "borrow_variant_field");
    try_mut_borrow_variant_field => try_borrow_variant_field(true, "mut_borrow_variant_field");
    try_ld_u8 => try_ld_integer(&Type::U8);
    try_cast_u8 => try_cast("cast_u8", Type::U8);
    try_ld_u16 => try_ld_integer(&Type::U16);
    try_cast_u16 => try_cast("cast_u16", Type::U16);
    try_ld_u32 => try_ld_integer(&Type::U32);
    try_cast_u32 => try_cast("cast_u32", Type::U32);
    try_ld_u64 => try_ld_integer(&Type::U64);
    try_cast_u64 => try_cast("cast_u64", Type::U64);
    try_ld_u128 => try_ld_integer(&Type::U128);
    try_cast_u128 => try_cast("cast_u128", Type::U128);
    try_ld_u256 => try_ld_integer(&Type::U256);
    try_cast_u256 => try_cast("cast_u256", Type::U256);
    try_ld_i8 => try_ld_integer(&Type::I8);
    try_cast_i8 => try_cast("cast_i8", Type::I8);
    try_ld_i16 => try_ld_integer(&Type::I16);
    try_cast_i16 => try_cast("cast_i16", Type::I16);
    try_ld_i32 => try_ld_integer(&Type::I32);
    try_cast_i32 => try_cast("cast_i32", Type::I32);
    try_ld_i64 => try_ld_integer(&Type::I64);
    try_cast_i64 => try_cast("cast_i64", Type::I64);
    try_ld_i128 => try_ld_integer(&Type::I128);
    try_cast_i128 => try_cast("cast_i128", Type::I128);
    try_ld_i256 => try_ld_integer(&Type::I256);
    try_cast_i256 => try_cast("cast_i256", Type::I256);
    try_add => try_same_type_binop("add", is_integer, None);
    try_sub => try_same_type_binop("sub", is_integer, None);
    try_mul => try_same_type_binop("mul", is_integer, None);
    try_div => try_same_type_binop("div", is_integer, None);
    try_mod => try_same_type_binop("mod", is_integer, None);
    try_bit_and => try_same_type_binop("bit_and", is_unsigned_integer, None);
    try_bit_or => try_same_type_binop("bit_or", is_unsigned_integer, None);
    try_xor => try_same_type_binop("xor", is_unsigned_integer, None);
    try_and => try_same_type_binop("and", is_bool, None);
    try_or => try_same_type_binop("or", is_bool, None);
    try_lt => try_same_type_binop("lt", is_integer, Some(Type::Bool));
    try_le => try_same_type_binop("le", is_integer, Some(Type::Bool));
    try_gt => try_same_type_binop("gt", is_integer, Some(Type::Bool));
    try_ge => try_same_type_binop("ge", is_integer, Some(Type::Bool));
    try_shl => try_shift("shl");
    try_shr => try_shift("shr");
    try_eq => try_cmp("eq");
    try_neq => try_cmp("neq");
}

fn try_ld_integer(
    ty: &Type,
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    push_integer_literal(u, state, instrs, ty)?;
    Ok(true)
}

/// Pops two operands of one type that `operand_ok` accepts and pushes `result`, or the operand
/// type when `result` is `None`. Covers arithmetic and comparisons (any integer type),
/// bitwise operations (the verifier requires unsigned integers) and `and` / `or`.
fn try_same_type_binop(
    opcode: &str,
    operand_ok: fn(&Type) -> bool,
    result: Option<Type>,
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let depth = state.stack_len();
    if depth < 2 {
        return Ok(false);
    }
    let top = &state.ty_stack[depth - 1];
    if !operand_ok(top) || top != &state.ty_stack[depth - 2] {
        return Ok(false);
    }
    let result = result.unwrap_or_else(|| top.clone());
    state.pop();
    state.pop();
    state.push(result);
    instrs.push(simple_op(opcode));
    Ok(true)
}

fn is_bool(ty: &Type) -> bool {
    matches!(ty, Type::Bool)
}

fn try_ld_const_vector(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    // `u8` vectors use the compact `Value::Bytes`. Lengths stay small so the constant pool
    // does not bloat; 32-byte elements get the smallest cap.
    let element_choice: u8 = u.int_in_range(0..=12)?;
    let (elem_ty, payload) = match element_choice {
        0 | 1 => (
            Type::U8,
            Value::Bytes(const_items(u, 16, |u| Ok(u.arbitrary()?))?),
        ),
        2 => (
            Type::U16,
            Value::Vector(const_items(u, 8, |u| Ok(Value::U16(u.arbitrary()?)))?),
        ),
        3 => (
            Type::U32,
            Value::Vector(const_items(u, 8, |u| Ok(Value::U32(u.arbitrary()?)))?),
        ),
        4 => (
            Type::U64,
            Value::Vector(const_items(u, 8, |u| Ok(Value::U64(u.arbitrary()?)))?),
        ),
        5 => (
            Type::U128,
            Value::Vector(const_items(u, 8, |u| Ok(Value::U128(u.arbitrary()?)))?),
        ),
        6 => (
            Type::U256,
            Value::Vector(const_items(u, 4, |u| {
                Ok(Value::U256(U256::from_le_bytes(u.arbitrary()?)))
            })?),
        ),
        7 => (
            Type::I8,
            Value::Vector(const_items(u, 8, |u| Ok(Value::I8(u.arbitrary()?)))?),
        ),
        8 => (
            Type::I16,
            Value::Vector(const_items(u, 8, |u| Ok(Value::I16(u.arbitrary()?)))?),
        ),
        9 => (
            Type::I32,
            Value::Vector(const_items(u, 8, |u| Ok(Value::I32(u.arbitrary()?)))?),
        ),
        10 => (
            Type::I64,
            Value::Vector(const_items(u, 8, |u| Ok(Value::I64(u.arbitrary()?)))?),
        ),
        11 => (
            Type::I128,
            Value::Vector(const_items(u, 8, |u| Ok(Value::I128(u.arbitrary()?)))?),
        ),
        _ if u.arbitrary::<bool>()? => (
            Type::I256,
            Value::Vector(const_items(u, 4, |u| {
                Ok(Value::I256(I256::from_le_bytes(u.arbitrary()?)))
            })?),
        ),
        _ => (
            Type::Bool,
            Value::Vector(const_items(u, 8, |u| Ok(Value::Bool(u.arbitrary()?)))?),
        ),
    };
    emit_ld_const(u, state, instrs, Type::Vector(Box::new(elem_ty)), payload)?;
    Ok(true)
}

/// Up to `max_len` constant elements drawn by `item`.
fn const_items<T>(
    u: &mut Unstructured<'_>,
    max_len: u8,
    mut item: impl FnMut(&mut Unstructured<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    let len = u.int_in_range(0..=max_len)?;
    (0..len).map(|_| item(u)).collect()
}

/// Emits `ld_const` of `value`. Each `ld_const` is a candidate site for the `valid_constants`
/// violation, which the post-process step applies by corrupting this constant.
fn emit_ld_const(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    ty: Type,
    value: Value,
) -> Result<()> {
    let const_idx = state.record_ld_const_emit();
    instrs.push(ld_const_instr(ty.clone(), value));
    state.push(ty);
    if state.wants_violation(u, DimensionFlag::Constants)? {
        state
            .post_process_intents
            .push(crate::ir::PostProcessIntent::CorruptConstant { index: const_idx });
    }
    Ok(())
}

fn try_ld_address(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let addr = pick_account_address(u)?;
    emit_ld_const(u, state, instrs, Type::Address, Value::Address(addr))?;
    Ok(true)
}

fn try_ld_bool(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let b: bool = u.arbitrary()?;
    instrs.push(simple_op(if b { "ld_true" } else { "ld_false" }));
    state.push(Type::Bool);
    Ok(true)
}

fn try_pop(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let ctx = ability_ctx(state);
    if !has_drop(&top, &ctx) {
        return Ok(false);
    }
    state.pop();
    instrs.push(simple_op("pop"));
    Ok(true)
}

/// `copy_loc` of an `Available` local whose type has `copy` (and `drop` when `require_drop`)
/// and that has no outstanding mutable borrow. The loop-safe set requires `drop` so the
/// loop-body drain at the back edge can pop the copied value.
fn try_copy_loc_where(
    require_drop: bool,
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let ctx = ability_ctx(state);
    let candidates: Vec<u16> = state
        .locals
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            l.init == LocalInit::Available
                && has_copy(&l.ty, &ctx)
                && (!require_drop || has_drop(&l.ty, &ctx))
                && !state.has_mut_borrow_on_local(*i as u16)
        })
        .map(|(i, _)| i as u16)
        .collect();
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick = *u.choose(&candidates)?;
    let (name, ty) = {
        let slot = state.local(pick).expect("slot");
        (slot.name.clone(), slot.ty.clone())
    };
    state.push(ty);
    instrs.push(name_instr("copy_loc", name));
    Ok(true)
}

fn try_move_loc(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    // A locals-safety violation permits moving from an uninitialized or moved-out slot.
    let violate = state.wants_violation(u, DimensionFlag::LocalsSafety)?;
    // Prefer unavailable locals when injecting a violation. If none are eligible, fall back
    // to an available local without recording a violation.
    let mut violating = false;
    let candidates: Vec<u16> = if violate {
        let unsafe_candidates: Vec<u16> = state
            .locals
            .iter()
            .enumerate()
            .filter(|(i, l)| l.init != LocalInit::Available && !state.is_local_borrowed(*i as u16))
            .map(|(i, _)| i as u16)
            .collect();
        if !unsafe_candidates.is_empty() {
            violating = true;
            unsafe_candidates
        } else {
            state
                .locals
                .iter()
                .enumerate()
                .filter(|(i, l)| {
                    l.init == LocalInit::Available && !state.is_local_borrowed(*i as u16)
                })
                .map(|(i, _)| i as u16)
                .collect()
        }
    } else {
        // `&signer` parameters stay available: global operations re-derive their account from
        // them with `copy_loc`.
        state
            .locals
            .iter()
            .enumerate()
            .filter(|(i, l)| {
                l.init == LocalInit::Available
                    && !state.is_local_borrowed(*i as u16)
                    && !is_signer_ref(&l.ty)
            })
            .map(|(i, _)| i as u16)
            .collect()
    };
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick = *u.choose(&candidates)?;
    let (name, ty) = {
        let slot = state.local(pick).expect("slot");
        (slot.name.clone(), slot.ty.clone())
    };
    if violating {
        state.record_violation(DimensionFlag::LocalsSafety);
    }
    state.local_mut(pick).unwrap().init = LocalInit::Moved;
    state.push(ty);
    instrs.push(name_instr("move_loc", name));
    Ok(true)
}

fn try_st_loc(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    // References in locals are untracked, so storing a tracked one would hide a live borrow.
    if state.top_ref_id().is_some() {
        return Ok(false);
    }
    let violate_types = state.wants_violation(u, DimensionFlag::TypeSafety)?;
    // `st_loc` requires the prior local value (if any) be droppable, since it's discarded
    // by the write. With type-safety violations on, the picker may also accept a slot whose
    // declared type doesn't match the operand on top of the stack — the violation is
    // recorded only when the pick actually lands on such a slot.
    let ctx = ability_ctx(state);
    let candidates: Vec<u16> = state
        .locals
        .iter()
        .enumerate()
        .filter(|(_, l)| violate_types || l.ty == top)
        .filter(|(_, l)| l.init != LocalInit::Available || has_drop(&l.ty, &ctx))
        // `st_loc` overwrites the local, conflicting with any outstanding borrow.
        .filter(|(i, _)| !state.is_local_borrowed(*i as u16))
        .map(|(i, _)| i as u16)
        .collect();
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick = *u.choose(&candidates)?;
    let (name, slot_ty) = {
        let slot = state.local(pick).expect("slot");
        (slot.name.clone(), slot.ty.clone())
    };
    if slot_ty != top {
        state.record_violation(DimensionFlag::TypeSafety);
    }
    state.pop();
    state.local_mut(pick).unwrap().init = LocalInit::Available;
    instrs.push(name_instr("st_loc", name));
    Ok(true)
}

/// `shl` / `shr` are asymmetric: the top of the stack is a `u8` shift count and the second
/// element is the integer value being shifted. The result is the integer with the same type
/// as the second operand. The verifier requires the *value* operand to be an unsigned
/// integer (`type_safety.rs` rejects signed values for shifts).
fn try_shift(
    opcode: &str,
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let n = state.stack_len();
    if n < 2 {
        return Ok(false);
    }
    let shift_ty = &state.ty_stack[n - 1];
    let value_ty = &state.ty_stack[n - 2];
    if !matches!(shift_ty, Type::U8) || !is_unsigned_integer(value_ty) {
        return Ok(false);
    }
    let result_ty = value_ty.clone();
    state.pop(); // shift count
    state.pop(); // value
    state.push(result_ty);
    instrs.push(simple_op(opcode));
    Ok(true)
}

fn try_cmp(
    opcode: &str,
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let n = state.stack_len();
    if n < 2 {
        return Ok(false);
    }
    let a = &state.ty_stack[n - 1];
    let b = &state.ty_stack[n - 2];
    // `eq` / `neq` accept any two operands of the same droppable type: references, vectors,
    // structs, enums and closures compare by value.
    if a != b || !has_drop(a, &ability_ctx(state)) {
        return Ok(false);
    }
    if !state.cfg.enable_function_value_ops && mentions_function(a) {
        return Ok(false);
    }
    state.pop();
    state.pop();
    state.push(Type::Bool);
    instrs.push(simple_op(opcode));
    Ok(true)
}

fn try_not(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(top) = state.top() else {
        return Ok(false);
    };
    if !matches!(top, Type::Bool) {
        return Ok(false);
    }
    instrs.push(simple_op("not"));
    Ok(true)
}

fn try_cast(
    opcode: &str,
    result_ty: Type,
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(top) = state.top() else {
        return Ok(false);
    };
    if !is_integer(top) {
        return Ok(false);
    }
    state.pop();
    state.push(result_ty);
    instrs.push(simple_op(opcode));
    Ok(true)
}

// ============================================================================
// Vector operations
// ============================================================================

fn try_vec_pack(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    // `vec_pack<T> N` pops N consecutive elements of type T from the top of the stack and
    // pushes a `vector<T>`. Counts of 0 are also valid (push empty vector). We bound N at
    // 4 to keep generated tests compact.
    let elem_ty = state
        .top()
        .filter(|ty| is_synthesizable_primitive(ty))
        .cloned();
    let max_count = match &elem_ty {
        Some(ty) => count_matching_top(state, ty),
        None => 0,
    };
    let count = u.int_in_range(0..=max_count.min(4) as u8)?;
    // When count > 0 we need a concrete element type from the stack; for count = 0 pick a
    // primitive directly so we always have a vector type to push.
    let elem_ty = if count > 0 {
        elem_ty.expect("a positive count matched the top element")
    } else {
        match u.int_in_range(0u8..=2)? {
            0 => Type::U64,
            1 => Type::U8,
            _ => Type::Bool,
        }
    };
    for _ in 0..count {
        state.pop();
    }
    state.push(Type::Vector(Box::new(elem_ty.clone())));
    instrs.push(Instr {
        opcode: "vec_pack".to_string(),
        operands: vec![
            Operand::Type(elem_ty),
            Operand::Imm(Value::U64(count as u64)),
        ],
    });
    // After emitting a `VecPack`, an instruction-consistency violation can rewrite a
    // pack count above `u16::MAX` during post-processing.
    if state.wants_violation(u, DimensionFlag::InstructionConsistency)? {
        state
            .post_process_intents
            .push(crate::ir::PostProcessIntent::CorruptInstructionConsistency);
    }
    Ok(true)
}

fn try_vec_unpack(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Vector(elem_ty) = top else {
        return Ok(false);
    };
    // The static unpack count must match the runtime vector length or the VM aborts; for
    // verifier-level fuzzing any count in 0..=4 is fine since the verifier only checks
    // the stack effect.
    let count: u64 = u.int_in_range(0..=4u64)?;
    state.pop();
    for _ in 0..count {
        state.push((*elem_ty).clone());
    }
    instrs.push(Instr {
        opcode: "vec_unpack".to_string(),
        operands: vec![
            Operand::Type((*elem_ty).clone()),
            Operand::Imm(Value::U64(count)),
        ],
    });
    Ok(true)
}

fn try_vec_len(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(_, inner) = top else {
        return Ok(false);
    };
    let Type::Vector(elem_ty) = *inner else {
        return Ok(false);
    };
    state.pop();
    state.push(Type::U64);
    instrs.push(type_instr("vec_len", (*elem_ty).clone()));
    Ok(true)
}

fn try_vec_borrow_indexed(
    want_mut: bool,
    opcode: &str,
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let n = state.stack_len();
    if n < 2 {
        return Ok(false);
    }
    // Top: u64 (index). Below: &(mut)? vector<T>.
    if !matches!(state.ty_stack[n - 1], Type::U64) {
        return Ok(false);
    }
    let Type::Reference(is_mut, inner) = &state.ty_stack[n - 2] else {
        return Ok(false);
    };
    if *is_mut != want_mut {
        return Ok(false);
    }
    let inner = inner.clone();
    let Type::Vector(elem_ty) = *inner else {
        return Ok(false);
    };
    // Snapshot the parent vector ref's borrow entry before we pop it — we need its root
    // and path to derive the child element-ref's borrow. Without this, the parent ref's
    // entry would be released by `state.pop()` and the new element ref would push as
    // untracked, leaving the borrow graph out of sync with the verifier (which keeps the
    // element ref borrowed from the vector).
    let parent_ref_id = state.ref_stack[n - 2];
    let (parent_root, parent_path) = match parent_ref_id.and_then(|id| state.borrows.get(id)) {
        Some(entry) => (entry.root.clone(), entry.path.clone()),
        // Without a tracked parent, skip the operation to avoid creating an untracked child.
        None => return Ok(false),
    };

    state.pop(); // index
    state.pop(); // parent ref (consumes its borrow-graph entry)

    // Vec element borrows can't be path-precise (indices are runtime values), so the child
    // shares the parent's path. That makes any subsequent borrow of the parent conflict
    // with this outstanding element ref, matching the verifier's conservative model.
    let child_mut = Mutability::of(want_mut);
    let child_ty = Type::Reference(want_mut, elem_ty.clone());
    let child_id = state
        .borrows
        .force_create(parent_root, parent_path, child_mut, parent_ref_id);
    state.push_ref(child_ty, child_id);
    instrs.push(type_instr(opcode, *elem_ty));
    Ok(true)
}

fn try_vec_push_back(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let n = state.stack_len();
    if n < 2 {
        return Ok(false);
    }
    // Top: T (value). Below: &mut vector<T>.
    let value_ty = state.ty_stack[n - 1].clone();
    let Type::Reference(true, inner) = &state.ty_stack[n - 2] else {
        return Ok(false);
    };
    let inner = inner.clone();
    let Type::Vector(elem_ty) = *inner else {
        return Ok(false);
    };
    if value_ty != *elem_ty {
        return Ok(false);
    }
    state.pop(); // value
    state.pop(); // ref
    instrs.push(type_instr("vec_push_back", *elem_ty));
    Ok(true)
}

fn try_vec_pop_back(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(true, inner) = top else {
        return Ok(false);
    };
    let Type::Vector(elem_ty) = *inner else {
        return Ok(false);
    };
    state.pop();
    state.push((*elem_ty).clone());
    instrs.push(type_instr("vec_pop_back", *elem_ty));
    Ok(true)
}

fn try_vec_swap(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_vectors {
        return Ok(false);
    }
    let n = state.stack_len();
    if n < 3 {
        return Ok(false);
    }
    // Top: u64. Below: u64. Below-below: &mut vector<T>.
    if !matches!(state.ty_stack[n - 1], Type::U64) || !matches!(state.ty_stack[n - 2], Type::U64) {
        return Ok(false);
    }
    let Type::Reference(true, inner) = &state.ty_stack[n - 3] else {
        return Ok(false);
    };
    let inner = inner.clone();
    let Type::Vector(elem_ty) = *inner else {
        return Ok(false);
    };
    state.pop();
    state.pop();
    state.pop();
    instrs.push(type_instr("vec_swap", *elem_ty));
    Ok(true)
}

// ============================================================================
// Closures (pack_closure / call_closure)
// ============================================================================
//
// `pack_closure F, MASK` constructs a function value from a function reference and a
// capture mask. MASK is a u64 bitmap; bit `i` is set when the `i`-th positional parameter
// of `F` is captured (popped from the stack and stored in the closure). Captured args sit
// on top of the stack at pack-time, in ascending bit order (lowest captured bit at the
// bottom, highest at the top — the verifier's `clos_pack` pops in reverse iteration order;
// see `move-bytecode-verifier/src/type_safety.rs::clos_pack`).
//
// The resulting closure type is `|uncaptured_params| -> results has <abilities>`. To enable
// mid-stream `call_closure` (i.e. calling the closure with the *uncaptured* params on the
// stack underneath), `try_pack_closure` also pushes synthesized values for the uncaptured
// params *before* the captured ones. Then `pack_closure` pops only the captured ones, the
// uncaptured stay underneath the freshly-pushed closure, and a later `try_call_closure`
// can fire when the stack already has `[..., uncaptured_*, closure]`.
//
// When MASK captures every param, `uncaptured_params` is empty and the stack holds just
// `[..., closure]`.
//
// Generics: `pack_closure F<...>, MASK` routes through `PackClosureGeneric`; the assembler
// distinguishes via the presence of type-args on the function reference.

fn try_pack_closure(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_closures {
        return Ok(false);
    }
    let here = state.module_ctx.module_id.clone();
    let callable_siblings = acyclic_sibling_filter(u, state)?;
    let mut candidates: Vec<(ModuleId, &Function)> = Vec::new();
    if let Some(here) = here.as_ref() {
        for (position, f) in state.module_ctx.functions.iter().enumerate() {
            if callable_siblings(position) && closure_target_ok(f) {
                candidates.push((here.clone(), f));
            }
        }
    }
    for m in &state.module_ctx.external_modules {
        for f in &m.functions {
            if closure_target_ok(f) && matches!(f.visibility, Visibility::Public) {
                candidates.push((m.id.clone(), f));
            }
        }
    }
    candidates.retain(|(module, function)| below_call_site_cap(state, module, function));
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick = u.choose(&candidates)?.clone();
    let (target_module, target_fun) = (pick.0, pick.1.clone());
    record_call_site(state, &target_module, &target_fun);

    // Generic closures: pick concrete type args for each of `target_fun`'s type parameters.
    // Substitution under all-primitive type args trivially satisfies any `copy/drop/store`
    // constraint (closure_target_ok already restricted constraints to subsets of those).
    // Type args go on the function reference; the assembler routes to `PackClosureGeneric`
    // when type args are present, else `PackClosure`.
    let type_args: Vec<Type> = pick_concrete_primitives(u, target_fun.type_params.len())?;

    let n_params = target_fun.params.len();
    let mut mask = pick_closure_mask(u, n_params)?;
    // A recursive target's depth is captured as a shallow literal, like a direct call's.
    let depth_param = is_bounded_recursive(state, &target_module, &target_fun)
        .then(|| crate::recursion::depth_parameter(&target_fun))
        .flatten();
    if let Some(depth) = depth_param {
        mask |= 1u64 << depth;
    }

    // Partition params into captured / uncaptured by bit order. Both lists keep the
    // original parameter order so the resulting closure's arg signature matches what the
    // verifier reconstructs in `clos_pack`.
    let mut captured_types: Vec<(usize, Type)> = Vec::with_capacity(n_params);
    let mut uncaptured_types: Vec<Type> = Vec::with_capacity(n_params);
    for (i, p) in target_fun.params.iter().enumerate() {
        let concrete = substitute(&p.ty, &type_args);
        if mask & (1u64 << i) != 0 {
            captured_types.push((i, concrete));
        } else {
            uncaptured_types.push(concrete);
        }
    }

    // Push uncaptured args first (they stay underneath the closure after pack), then
    // captured (pack pops them off the top).
    for ty in &uncaptured_types {
        push_synthesized_value(u, state, instrs, ty)?;
    }
    for (index, ty) in &captured_types {
        if Some(*index) == depth_param {
            let depth = u.int_in_range(0..=NESTED_RECURSION_DEPTH_MAX)?;
            instrs.push(imm_instr("ld_u64", Value::U64(depth)));
            state.push(Type::U64);
        } else {
            push_synthesized_value(u, state, instrs, ty)?;
        }
    }
    // pack_closure pops captured args off the stack; uncaptured stay where we put them.
    for _ in &captured_types {
        state.pop();
    }

    let closure_results: Vec<Type> = target_fun
        .return_types
        .iter()
        .map(|t| substitute(t, &type_args))
        .collect();
    let closure_ty = Type::Function {
        args: uncaptured_types.clone(),
        results: closure_results,
        abilities: move_core_types::ability::AbilitySet::PUBLIC_FUNCTIONS, // copy + drop
    };
    state.push(closure_ty);
    instrs.push(Instr {
        opcode: "pack_closure".to_string(),
        operands: vec![
            Operand::Function(
                QualifiedFunction {
                    module: target_module,
                    name: target_fun.name.clone(),
                },
                type_args,
            ),
            Operand::Imm(Value::U64(mask)),
        ],
    });
    Ok(true)
}

/// Pick a closure mask: a u64 with at most `n_params` bits significant. Each bit decides
/// independently (≈50% probability) whether the corresponding parameter is captured. When
/// `n_params >= 64` we let the raw mask flow through unmasked (the verifier's
/// instruction-consistency check enforces `bits_set(mask) <= n_params`, and 64-param
/// functions are already excluded by `closure_target_ok`'s `params.len() <= 4` cap).
fn pick_closure_mask(u: &mut Unstructured<'_>, n_params: usize) -> Result<u64> {
    let raw: u64 = u.arbitrary()?;
    if n_params >= 64 {
        Ok(raw)
    } else {
        let span = if n_params == 0 {
            0
        } else {
            (1u64 << n_params) - 1
        };
        Ok(raw & span)
    }
}

fn try_call_closure(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_closures {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Function {
        args,
        results,
        abilities,
    } = top
    else {
        return Ok(false);
    };
    // call_closure expects the stack as `[..., arg_0, ..., arg_n-1, closure]`. Check that
    // the uncaptured-arg types match what's underneath the closure (in parameter order,
    // bottom to top — same order as `clos_pack` records and `call_closure` pops in
    // reverse).
    let n = state.stack_len();
    if n < 1 + args.len() {
        return Ok(false);
    }
    let base = n - 1 - args.len();
    for (i, expected) in args.iter().enumerate() {
        if &state.ty_stack[base + i] != expected {
            return Ok(false);
        }
    }
    let closure_ty = Type::Function {
        args,
        results: results.clone(),
        abilities,
    };
    // Pop the closure first, then the args (matching the verifier's pop order).
    state.pop();
    let arg_count = match &closure_ty {
        Type::Function { args, .. } => args.len(),
        _ => unreachable!("closure_ty was constructed as Function above"),
    };
    for _ in 0..arg_count {
        state.pop();
    }
    for r in &results {
        state.push(r.clone());
    }
    instrs.push(type_instr("call_closure", closure_ty));
    Ok(true)
}

fn closure_target_ok(f: &Function) -> bool {
    // Move's closure machinery only accepts functions explicitly marked `#[persistent]` as
    // pack_closure targets. We also require a non-empty result list so the rendered closure
    // type unambiguously parses as `|args| results has abilities` rather than ambiguating
    // the trailing `has`-clause with a non-existent result type.
    //
    // Generics are accepted here: `try_pack_closure` substitutes with primitive type args.
    // Since `pick_type_param_constraint` only emits subsets of `{copy, drop, store}`, every
    // primitive satisfies the declared constraints — no need to filter on constraint shape.
    let is_persistent = f.attributes.iter().any(|a| {
        matches!(
            a,
            move_binary_format::file_format::FunctionAttribute::Persistent
        )
    });
    let n = f.type_params.len();
    is_persistent
        && f.acquires.is_empty()
        && f.params.len() <= 4
        && !f.return_types.is_empty()
        && f.params
            .iter()
            .all(|p| is_synthesizable_primitive_after_subst(&p.ty, n))
        && f.return_types
            .iter()
            .all(|t| is_synthesizable_primitive_after_subst(t, n))
}

// ============================================================================
// Global storage operations (borrow_global, mut_borrow_global, move_from, exists)
// ============================================================================
//
// These ops operate on resources — singleton key-able structs declared in the current
// module. Each pops an `address` from the stack and pushes either the resource value (`T`),
// a reference (`&T` / `&mut T`), or `bool`. The function's `acquires` annotation must list
// resources borrowed or moved out; `exists` requires no annotation. The signature pass
// initially declares every eligible local key struct, then unused annotations are pruned.

fn try_global_borrow(
    want_mut: bool,
    opcode: &str,
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_globals {
        return Ok(false);
    }
    let mutability = Mutability::of(want_mut);
    if let Some(guaranteed) = state.guaranteed_resource.clone() {
        let root = BorrowRoot::global(&guaranteed.resource);
        if !matches!(state.top(), Some(Type::Address))
            && state.borrows.can_borrow(&root, &[], mutability)
        {
            push_account(state, instrs, &guaranteed.account);
            return emit_global_borrow(
                want_mut,
                opcode,
                guaranteed.resource,
                guaranteed.type_args,
                state,
                instrs,
            );
        }
    }
    if !allow_unguarded_read(u, state)? {
        return Ok(false);
    }
    let Some((qs, type_args)) = pick_key_struct(u, state)? else {
        return Ok(false);
    };
    // The verifier indexes global resources by struct-def (no type args), so different
    // instantiations of the same generic resource share a single borrow slot. Use the
    // struct name alone as the conflict key to match.
    let root = BorrowRoot::global(&qs);
    if !state.borrows.can_borrow(&root, &[], mutability) {
        return Ok(false);
    }
    push_address_if_needed(u, state, instrs)?;
    emit_global_borrow(want_mut, opcode, qs, type_args, state, instrs)
}

/// Emits `opcode` (a global borrow) of `qs<type_args>` from the address on top of the stack.
fn emit_global_borrow(
    want_mut: bool,
    opcode: &str,
    qs: QualifiedStruct,
    type_args: Vec<Type>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    state.pop();
    let ref_ty = Type::Reference(
        want_mut,
        Box::new(Type::Struct(qs.clone(), type_args.clone())),
    );
    let ref_id = state.borrows.create(
        BorrowRoot::global(&qs),
        vec![],
        Mutability::of(want_mut),
        None,
    );
    state.push_ref(ref_ty, ref_id);
    state.accessed_key_structs.insert(qs.name.clone());
    instrs.push(struct_instr(opcode, qs, type_args));
    Ok(true)
}

fn try_move_from(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_globals {
        return Ok(false);
    }
    // The block's guaranteed resource, read once: moving it out ends the guarantee.
    let guaranteed = state
        .guaranteed_resource
        .clone()
        .filter(|_| !matches!(state.top(), Some(Type::Address)));
    let (qs, type_args) = match &guaranteed {
        Some(guaranteed) => (guaranteed.resource.clone(), guaranteed.type_args.clone()),
        None => {
            if !allow_unguarded_read(u, state)? {
                return Ok(false);
            }
            let Some(picked) = pick_key_struct(u, state)? else {
                return Ok(false);
            };
            picked
        },
    };
    // `move_from` consumes the resource — verifier rejects if any outstanding reference to
    // it exists. Use a `Mut` probe (which rejects on any overlapping outstanding borrow,
    // including `Imm`s) to confirm the resource isn't currently borrowed.
    let root = BorrowRoot::global(&qs);
    if !state.borrows.can_borrow(&root, &[], Mutability::Mut) {
        return Ok(false);
    }
    match &guaranteed {
        Some(guaranteed) => {
            push_account(state, instrs, &guaranteed.account);
            state.guaranteed_resource = None;
        },
        None => push_address_if_needed(u, state, instrs)?,
    }
    state.pop();
    state.push(Type::Struct(qs.clone(), type_args.clone()));
    state.accessed_key_structs.insert(qs.name.clone());
    instrs.push(struct_instr("move_from", qs, type_args));
    Ok(true)
}

fn try_exists(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_globals {
        return Ok(false);
    }
    let Some((qs, type_args)) = pick_key_struct(u, state)? else {
        return Ok(false);
    };
    push_address_if_needed(u, state, instrs)?;
    state.pop();
    state.push(Type::Bool);
    instrs.push(struct_instr("exists", qs, type_args));
    Ok(true)
}

/// `move_to<K>(&signer, K)` — publish a fresh resource value into the signer's account. The
/// generator stages the operand stack itself: `copy_loc` of a `&signer` param, then synthesize
/// each field of the picked key-struct, then `pack <K>` to assemble the value, then `move_to`.
/// Skipped when no `&signer` param is available, no key struct has fully-synthesizable fields,
/// or the resource has any outstanding borrow on it (the verifier requires exclusive access).
///
/// Acquires note: `move_to` *creates* a resource and does not require the function to declare
/// `acquires K` (only the read ops — `move_from`, `*borrow_global*` — do). So this generator
/// intentionally does NOT touch `state.accessed_key_structs`; the orchestrator's `acquires`
/// pruning is driven purely by the read-side global ops.
fn try_move_to(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_globals {
        return Ok(false);
    }
    // Find an Available `&signer` local. Refs have copy, so `copy_loc` leaves the param
    // intact for later uses.
    let signer_idx = state.locals.iter().enumerate().find_map(|(i, l)| {
        if l.init == LocalInit::Available && is_signer_ref(&l.ty) {
            Some(i as u16)
        } else {
            None
        }
    });
    let Some(signer_idx) = signer_idx else {
        return Ok(false);
    };

    let Some((qs, type_args)) = pick_key_struct(u, state)? else {
        return Ok(false);
    };
    if state.stored_key_structs.contains(&qs.name) {
        return Ok(false);
    }
    let decl = state
        .module_ctx
        .local_struct(&qs)
        .cloned()
        .expect("pick_key_struct returned a name not in module_ctx.structs");
    let StructLayout::Singleton(fields) = &decl.layout else {
        return Ok(false);
    };
    // Every field must be synthesizable after substituting `type_args`.
    let concrete_field_types: Vec<Type> = fields
        .iter()
        .map(|f| substitute(&f.ty, &type_args))
        .collect();
    if !concrete_field_types
        .iter()
        .all(|ty| is_synthesizable(ty, state))
    {
        return Ok(false);
    }
    // `move_to` writes the resource — verifier requires no outstanding borrow on its slot.
    let root = BorrowRoot::global(&qs);
    if !state.borrows.can_borrow(&root, &[], Mutability::Mut) {
        return Ok(false);
    }

    // 1) Push `&signer` via copy_loc. Refs carry no borrow-graph entry under the existing
    //    convention (see `try_copy_loc`), so we use `state.push` not `state.push_ref`.
    let signer_name = state.local(signer_idx).expect("signer slot").name.clone();
    instrs.push(name_instr("copy_loc", signer_name));
    state.push(Type::Reference(false, Box::new(Type::Signer)));

    // 2) Push each field's synthesized value.
    for ty in &concrete_field_types {
        push_synthesized_value(u, state, instrs, ty)?;
    }
    // 3) `pack <K>` — consume the field values, push the struct.
    for _ in &concrete_field_types {
        state.pop();
    }
    state.push(Type::Struct(qs.clone(), type_args.clone()));
    instrs.push(struct_instr("pack", qs.clone(), type_args.clone()));

    // 4) `move_to <K>` — consume `[..., &signer, K]`.
    state.pop(); // struct value
    state.pop(); // &signer (no tracker entry to release)
    state.stored_key_structs.insert(qs.name.clone());
    instrs.push(struct_instr("move_to", qs, type_args));
    Ok(true)
}

/// The instructions of one `move_to` of a resource declared by `module_ctx`, signed by one of
/// `function`'s `&signer` parameters, as a stack-neutral sequence to prepend to its body.
/// Returns `None` when the function has no signer or the module no storable resource.
pub(crate) fn resource_store_prefix(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_ctx: &crate::state::ModuleCtx,
    function: &Function,
) -> Result<Option<Vec<Instr>>> {
    let params = function
        .params
        .iter()
        .map(|local| crate::state::LocalSlot {
            name: local.name.clone(),
            ty: local.ty.clone(),
            init: LocalInit::Available,
        })
        .collect();
    let constraints = function
        .type_params
        .iter()
        .map(|param| param.constraints)
        .collect();
    let mut state = FunctionState::new(
        cfg,
        module_ctx,
        params,
        Vec::new(),
        constraints,
        function.return_types.clone(),
        0,
    );
    let mut instrs = Vec::new();
    Ok(try_move_to(u, &mut state, &mut instrs)?.then_some(instrs))
}

/// Whether an acquiring read may target a resource that is not known to exist. Such a read
/// aborts the transaction with `MISSING_DATA` whenever the resource is absent.
fn allow_unguarded_read(u: &mut Unstructured<'_>, state: &FunctionState<'_, '_>) -> Result<bool> {
    let prob = state.cfg.unguarded_read_prob;
    Ok(prob == u8::MAX || crate::violations::chance(u, prob)?)
}

/// Pushes `exists<K>(address)` for a resource of the current module and an account from the
/// pool, returning the guarantee a block entered on `true` gets. `None` when the module has no
/// resource.
pub(crate) fn push_exists_guard(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<Option<crate::state::GuaranteedResource>> {
    if !state.cfg.enable_globals {
        return Ok(None);
    }
    let Some((resource, type_args)) = pick_key_struct(u, state)? else {
        return Ok(None);
    };
    let account = pick_account_source(u, state)?;
    push_account(state, instrs, &account);
    instrs.push(struct_instr("exists", resource.clone(), type_args.clone()));
    state.pop();
    state.push(Type::Bool);
    Ok(Some(crate::state::GuaranteedResource {
        resource,
        type_args,
        account,
    }))
}

/// The account for a global operation: usually a `&signer` parameter's address when the
/// function has one, since that is where `move_to` stores, otherwise an account from the pool.
fn pick_account_source(
    u: &mut Unstructured<'_>,
    state: &FunctionState<'_, '_>,
) -> Result<crate::state::AccountSource> {
    let signers: Vec<Identifier> = state
        .locals
        .iter()
        .filter(|local| local.init == LocalInit::Available && is_signer_ref(&local.ty))
        .map(|local| local.name.clone())
        .collect();
    if state.cfg.enable_natives && !signers.is_empty() && u.ratio(3u8, 4u8)? {
        return Ok(crate::state::AccountSource::Signer(
            u.choose(&signers)?.clone(),
        ));
    }
    Ok(crate::state::AccountSource::Literal(pick_account_address(
        u,
    )?))
}

/// Pushes the address `account` names.
fn push_account(
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    account: &crate::state::AccountSource,
) {
    match account {
        crate::state::AccountSource::Literal(address) => {
            let _ = state.record_ld_const_emit();
            instrs.push(ld_const_instr(Type::Address, Value::Address(*address)));
        },
        crate::state::AccountSource::Signer(local) => {
            instrs.push(name_instr("copy_loc", local.clone()));
            instrs.push(Instr {
                opcode: "call".to_string(),
                operands: vec![Operand::Function(
                    QualifiedFunction {
                        module: ModuleId::new(
                            move_core_types::account_address::AccountAddress::ONE,
                            Identifier::new("signer").expect("identifier"),
                        ),
                        name: Identifier::new("address_of").expect("identifier"),
                    },
                    Vec::new(),
                )],
            });
        },
    }
    state.push(Type::Address);
}

/// If the top of stack isn't already a `Type::Address`, emit an inline `ld_const <address>`
/// to push one. Lets global-op generators fire even when prior instructions didn't leave
/// an address on top — without this, the dispatcher's rejection sampling rarely converges
/// on the right (address, global-op) sequence.
fn push_address_if_needed(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<()> {
    if matches!(state.top(), Some(Type::Address)) {
        return Ok(());
    }
    let addr = pick_account_address(u)?;
    let _ = state.record_ld_const_emit();
    instrs.push(ld_const_instr(Type::Address, Value::Address(addr)));
    state.push(Type::Address);
    Ok(())
}

pub(crate) fn pick_key_struct(
    u: &mut Unstructured<'_>,
    state: &FunctionState<'_, '_>,
) -> Result<Option<(QualifiedStruct, Vec<Type>)>> {
    let Some(module_id) = state.module_ctx.module_id.clone() else {
        return Ok(None);
    };
    let candidates: Vec<(Identifier, usize)> = state
        .module_ctx
        .structs
        .iter()
        .filter(|s| {
            matches!(s.layout, StructLayout::Singleton(_))
                && s.abilities
                    .has_ability(move_core_types::ability::Ability::Key)
        })
        .map(|s| (s.name.clone(), s.type_params.len()))
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    let pick = u.choose(&candidates)?.clone();
    let (name, n_type_params) = pick;
    // Bytecode-level acquires is by struct-def index, so any instantiation of a generic
    // key resource is covered by the function's `acquires <name>` annotation alone — we
    // pick the type args here independently per global-op site. Callers that emit an
    // *acquiring* opcode (`MutBorrowGlobal`, `ImmBorrowGlobal`, `MoveFrom` — anything the
    // acquires verifier tracks) must record the access via
    // [`FunctionState::accessed_key_structs`] so the orchestrator's pruning step keeps the
    // function's declared `acquires` set tight. `Exists` is the lone exception — the
    // acquires verifier doesn't count it, so its caller does not register.
    let type_args = if n_type_params == 0 {
        Vec::new()
    } else if crate::violations::chance(u, state.cfg.canonical_resource_prob)? {
        // One instantiation per resource, so stores and later reads name the same instance.
        // `u64` satisfies every constraint a key struct's type parameter can declare.
        vec![Type::U64; n_type_params]
    } else {
        // Find the picked struct's declaration so we can honor each type-param's declared
        // constraints, which may be subsets of `copy + drop + store`.
        let decl = state
            .module_ctx
            .structs
            .iter()
            .find(|d| d.name == name)
            .expect("pick_key_struct: declaration must exist for chosen name");
        let mut args = Vec::with_capacity(n_type_params);
        for tp in &decl.type_params {
            args.push(crate::types::pick_concrete_type(
                u,
                &state.module_ctx.structs,
                &state.module_ctx.external_modules,
                &module_id,
                tp.constraints,
                state.cfg.max_type_depth,
            )?);
        }
        args
    };
    Ok(Some((
        QualifiedStruct {
            module: module_id,
            name,
        },
        type_args,
    )))
}

/// Length of the type-matching prefix of `state.ty_stack` (starting at the top), going down
/// while each entry equals `ty`.
fn count_matching_top(state: &FunctionState<'_, '_>, ty: &Type) -> usize {
    let n = state.stack_len();
    let mut count = 0;
    while count < n && &state.ty_stack[n - 1 - count] == ty {
        count += 1;
    }
    count
}

/// The type arguments with which `decl`, whose fields have `raw_field_types`, can be packed
/// from the values on top of the stack, and whether they satisfy the declared constraints.
/// `None` when no instantiation fits within the type depth limit.
fn infer_pack_type_args(
    u: &mut Unstructured<'_>,
    state: &FunctionState<'_, '_>,
    ctx: &AbilityCtx<'_>,
    module_id: &ModuleId,
    decl: &StructDecl,
    raw_field_types: &[Type],
) -> Result<Option<(Vec<Type>, bool)>> {
    let Some(base) = state.stack_len().checked_sub(raw_field_types.len()) else {
        return Ok(None);
    };
    let Some(type_args) = infer_type_args(
        &decl.type_params,
        raw_field_types,
        &state.ty_stack[base..],
        u,
        &state.module_ctx.structs,
        &state.module_ctx.external_modules,
        module_id,
        state.cfg.max_type_depth,
    )?
    else {
        return Ok(None);
    };
    if !packed_depth_ok(&type_args, state.cfg.max_type_depth) {
        return Ok(None);
    }
    let constraints_ok = type_args.iter().zip(&decl.type_params).all(|(arg, tp)| {
        is_valid_type_arg(arg) && tp.constraints.is_subset(type_abilities(arg, ctx))
    });
    Ok(Some((type_args, constraints_ok)))
}

/// Whether a struct instantiated with `type_args` stays within `max_depth`. Packing nests
/// stack-derived type args, so repeated packs would otherwise grow the type without bound.
fn packed_depth_ok(type_args: &[Type], max_depth: u8) -> bool {
    type_args.iter().map(type_depth).max().unwrap_or(0) < max_depth as usize
}

fn try_pack(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(module_id) = state.module_ctx.module_id.clone() else {
        return Ok(false);
    };
    // When `valid_signature_v2 = false`, allow candidates whose back-derived type arguments
    // are references or don't satisfy the declared constraints (e.g. a `&u64` value bound to
    // a type parameter).
    // Recorded only when a constraint-failing candidate is actually picked — the random
    // candidate pick often lands on a valid one.
    let want_sig_v2_violation = state.wants_violation(u, DimensionFlag::SignatureV2)?;
    let ctx = ability_ctx(state);
    // Each candidate is `(decl_idx, type_args, field_types_substituted)`. We back-derive
    // type args from the stack via unification, so generic instantiations naturally take on
    // whatever primitive (or reference, under signature-v2 violation) is sitting on the
    // operand stack.
    let mut valid: Vec<(usize, Vec<Type>, Vec<Type>)> = Vec::new();
    let mut invalid: Vec<(usize, Vec<Type>, Vec<Type>)> = Vec::new();
    for (i, decl) in state.module_ctx.structs.iter().enumerate() {
        let raw_field_types: Vec<Type> = match &decl.layout {
            StructLayout::Singleton(fs) => fs.iter().map(|f| f.ty.clone()).collect(),
            StructLayout::Variants(_) => continue,
        };
        let Some((type_args, constraints_ok)) =
            infer_pack_type_args(u, state, &ctx, &module_id, decl, &raw_field_types)?
        else {
            continue;
        };
        let field_types: Vec<Type> = raw_field_types
            .iter()
            .map(|ty| substitute(ty, &type_args))
            .collect();
        if constraints_ok {
            valid.push((i, type_args, field_types));
        } else if want_sig_v2_violation {
            invalid.push((i, type_args, field_types));
        }
    }
    // Prefer invalid candidates when we want to fire the sig_v2 violation; only this branch
    // actually surfaces the verifier rejection. Fall back to valid candidates if no invalid
    // ones exist for this stack snapshot.
    let (mut candidates, violation_fired) = if want_sig_v2_violation && !invalid.is_empty() {
        (invalid, true)
    } else {
        (valid, false)
    };
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick_idx_in_candidates = u.int_in_range(0..=candidates.len() - 1)?;
    let (decl_idx, type_args, field_types) = candidates.swap_remove(pick_idx_in_candidates);
    if violation_fired {
        state.record_violation(DimensionFlag::SignatureV2);
    }
    let decl = &state.module_ctx.structs[decl_idx];
    let qs = QualifiedStruct {
        module: module_id,
        name: decl.name.clone(),
    };
    for _ in 0..field_types.len() {
        state.pop();
    }
    state.push(Type::Struct(qs.clone(), type_args.clone()));
    instrs.push(struct_instr("pack", qs, type_args));
    Ok(true)
}

fn try_imm_borrow_loc(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_references {
        return Ok(false);
    }
    // `imm_borrow_loc` is allowed against any Available local with no outstanding *mut*
    // borrow. Multiple imm borrows of the same root are fine. We exclude reference-typed
    // locals (e.g. `&signer` params): the verifier rejects `borrow_loc` of a reference
    // (`BORROWLOC_REFERENCE_ERROR`).
    let candidates: Vec<u16> = state
        .locals
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            l.init == LocalInit::Available
                && !crate::types::is_reference(&l.ty)
                && state
                    .borrows
                    .can_borrow(&BorrowRoot::Local(*i as u16), &[], Mutability::Imm)
        })
        .map(|(i, _)| i as u16)
        .collect();
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick = *u.choose(&candidates)?;
    let (name, ty) = {
        let slot = state.local(pick).expect("slot");
        (slot.name.clone(), slot.ty.clone())
    };
    let ref_ty = Type::Reference(false, Box::new(ty));
    let ref_id = state
        .borrows
        .create(BorrowRoot::Local(pick), vec![], Mutability::Imm, None);
    state.push_ref(ref_ty, ref_id);
    instrs.push(name_instr("borrow_loc", name));
    Ok(true)
}

fn try_mut_borrow_loc(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_references {
        return Ok(false);
    }
    let violate_refs = state.wants_violation(u, DimensionFlag::ReferenceSafety)?;

    let candidates: Vec<u16> = state
        .locals
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            l.init == LocalInit::Available
                && !crate::types::is_reference(&l.ty)
                && state
                    .borrows
                    .can_borrow(&BorrowRoot::Local(*i as u16), &[], Mutability::Mut)
        })
        .map(|(i, _)| i as u16)
        .collect();
    if candidates.is_empty() {
        // Nothing emitted — the violation decision (if any) stays unrecorded.
        return Ok(false);
    }
    let pick = *u.choose(&candidates)?;
    let (name, ty) = {
        let slot = state.local(pick).expect("slot");
        (slot.name.clone(), slot.ty.clone())
    };
    let ref_ty = Type::Reference(true, Box::new(ty));
    let ref_id = state
        .borrows
        .create(BorrowRoot::Local(pick), vec![], Mutability::Mut, None);
    state.push_ref(ref_ty.clone(), ref_id);
    instrs.push(name_instr("mut_borrow_loc", name.clone()));

    // Reference-safety violation: copy the local while the mutable borrow is live, which the
    // verifier rejects with `COPYLOC_EXISTS_BORROW_ERROR`. A second `mut_borrow_loc` would not
    // do: sibling mutable borrows of a local are legal until one is used against the other.
    let local_ty = state.local(pick).expect("slot").ty.clone();
    if violate_refs && has_copy(&local_ty, &ability_ctx(state)) {
        state.record_violation(DimensionFlag::ReferenceSafety);
        state.push(local_ty);
        instrs.push(name_instr("copy_loc", name));
    }
    Ok(true)
}

/// Overwrites one primitive field of the block's guaranteed resource: pushes the new value,
/// then `mut_borrow_global`, `mut_borrow_field` and `write_ref`. The write modifies a
/// resource an earlier transaction stored; without it, generated code almost never changes
/// stored state, since `write_ref` needs the value below the reference.
fn try_modify_resource(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(guaranteed) = state.guaranteed_resource.clone() else {
        return Ok(false);
    };
    let Some(decl) = state.module_ctx.local_struct(&guaranteed.resource).cloned() else {
        return Ok(false);
    };
    let StructLayout::Singleton(fields) = &decl.layout else {
        return Ok(false);
    };
    let writable: Vec<(u16, Type)> = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| (idx as u16, substitute(&field.ty, &guaranteed.type_args)))
        .filter(|(_, ty)| is_synthesizable_primitive(ty))
        .collect();
    let Some((field_idx, field_ty)) = u.choose(&writable).ok().cloned() else {
        return Ok(false);
    };
    let root = BorrowRoot::global(&guaranteed.resource);
    if matches!(state.top(), Some(Type::Address))
        || !state.borrows.can_borrow(&root, &[], Mutability::Mut)
    {
        return Ok(false);
    }
    push_synthesized_value(u, state, instrs, &field_ty)?;
    push_account(state, instrs, &guaranteed.account);
    emit_global_borrow(
        true,
        "mut_borrow_global",
        guaranteed.resource,
        guaranteed.type_args,
        state,
        instrs,
    )?;
    if !borrow_field_at(true, "mut_borrow_field", field_idx, state, instrs)? {
        return Err(crate::error::Error::GenerationStuck(
            "resource field borrow failed after a global borrow",
        ));
    }
    if !try_write_ref(u, state, instrs)? {
        return Err(crate::error::Error::GenerationStuck(
            "write through a resource field reference failed",
        ));
    }
    Ok(true)
}

/// Common implementation for `borrow_field` / `mut_borrow_field`. Requires the top of stack
/// be a (mut|imm) reference to a singleton struct declared in the current module. Consumes
/// the parent reference and emits a new reference rooted at the same underlying location, but
/// reaching one field deeper.
fn try_borrow_field(
    want_mut: bool,
    opcode: &str,
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_references {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(parent_is_mut, parent_inner) = &top else {
        return Ok(false);
    };
    // For `mut_borrow_field`, the parent must be `&mut`. For `borrow_field` either flavor is
    // acceptable (you can read-borrow into an imm or freeze a mut).
    if want_mut && !parent_is_mut {
        return Ok(false);
    }
    let Type::Struct(qs, type_args) = parent_inner.as_ref() else {
        return Ok(false);
    };
    // Snapshot parent struct's type args; we'll thread them through the emitted opcode and
    // use them to substitute the field's raw type. For non-generic decls `type_args` is
    // empty and substitute is a no-op.
    let parent_type_args = type_args.clone();
    let decl = state.module_ctx.local_struct(qs).cloned();
    let Some(decl) = decl else {
        return Ok(false);
    };
    if decl.type_params.len() != parent_type_args.len() {
        return Ok(false);
    }
    let StructLayout::Singleton(fields) = decl.layout else {
        return Ok(false);
    };
    if fields.is_empty() {
        return Ok(false);
    }
    let field_idx = u.int_in_range(0..=(fields.len() - 1) as u16)?;
    borrow_field_at(want_mut, opcode, field_idx, state, instrs)
}

/// Borrows field `field_idx` of the struct the top-of-stack reference points to.
fn borrow_field_at(
    want_mut: bool,
    opcode: &str,
    field_idx: u16,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(Type::Reference(_, parent_inner)) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Struct(qs, parent_type_args) = parent_inner.as_ref() else {
        return Ok(false);
    };
    let Some(decl) = state.module_ctx.local_struct(qs).cloned() else {
        return Ok(false);
    };
    let StructLayout::Singleton(fields) = decl.layout else {
        return Ok(false);
    };
    let Some(field) = fields.get(field_idx as usize) else {
        return Ok(false);
    };
    let parent_type_args = parent_type_args.clone();
    let qs = qs.clone();
    let child_inner_ty = substitute(&field.ty, &parent_type_args);

    // Snapshot parent root/path before consuming, so we can derive the child borrow.
    let parent_ref_id = state.top_ref_id();
    let (parent_root, parent_path) = match parent_ref_id.and_then(|id| state.borrows.get(id)) {
        Some(entry) => (entry.root.clone(), entry.path.clone()),
        // A reference created outside the borrow graph has no tracked root for the child.
        None => return Ok(false),
    };

    state.pop(); // consumes parent from graph

    let mut child_path = parent_path;
    child_path.push(BorrowStep::Field(field_idx));

    let child_mut = Mutability::of(want_mut);
    let child_ty = Type::Reference(want_mut, Box::new(child_inner_ty));
    let child_id = state
        .borrows
        .force_create(parent_root, child_path, child_mut, None);
    state.push_ref(child_ty, child_id);

    instrs.push(Instr {
        opcode: opcode.to_string(),
        operands: vec![Operand::Field {
            owner: qs.clone(),
            type_args: parent_type_args,
            field: field.name.clone(),
        }],
    });
    Ok(true)
}

fn try_write_ref(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    // `write_ref` requires the top of stack be `&mut T` and the second-from-top be `T`.
    let n = state.stack_len();
    if n < 2 {
        return Ok(false);
    }
    let top = state.ty_stack[n - 1].clone();
    let Type::Reference(true, inner) = top else {
        return Ok(false);
    };
    let value_ty = state.ty_stack[n - 2].clone();
    if *inner != value_ty {
        return Ok(false);
    }
    // `write_ref` drops the previous value at the reference target, so the inner type must
    // have `Drop`.
    let ctx = ability_ctx(state);
    if !has_drop(inner.as_ref(), &ctx) {
        return Ok(false);
    }
    state.pop(); // ref (releases graph entry)
    state.pop(); // value
    instrs.push(simple_op("write_ref"));
    Ok(true)
}

fn try_read_ref(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    // `read_ref` requires the top of stack be a reference, and the underlying type must have
    // `Copy` (read_ref makes a copy of the referenced value).
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(_, inner) = top else {
        return Ok(false);
    };
    let inner_ty: Type = *inner;
    let ctx = ability_ctx(state);
    if !has_copy(&inner_ty, &ctx) {
        return Ok(false);
    }
    // Pop the ref slot (this auto-releases the borrow-graph entry via `pop`).
    state.pop();
    state.push(inner_ty);
    instrs.push(simple_op("read_ref"));
    Ok(true)
}

fn try_call(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let allow_storing = !state.calls_storing_function;
    emit_call(u, state, instrs, |function| {
        allow_storing || !stores_resource(function)
    })
}

fn stores_resource(function: &Function) -> bool {
    function
        .body
        .iter()
        .flat_map(|block| &block.instrs)
        .any(|instr| instr.opcode == "move_to")
}

/// Calls a function whose body stores a resource, passing on one of the caller's signers.
fn try_call_storing(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if state.calls_storing_function {
        return Ok(false);
    }
    emit_call(u, state, instrs, stores_resource)
}

/// Largest recursion depth generated code passes to a recursive callee. Run steps calling an
/// entry function directly still draw any depth.
const NESTED_RECURSION_DEPTH_MAX: u64 = 4;

/// Whether `module::function` is a bounded-recursive function: chosen so at signature time
/// (a sibling whose body may not exist yet) or recognised by its prologue.
fn is_bounded_recursive(
    state: &FunctionState<'_, '_>,
    module: &ModuleId,
    function: &Function,
) -> bool {
    (state.module_ctx.module_id.as_ref() == Some(module)
        && state.module_ctx.recursive.contains(&function.name))
        || crate::recursion::recursive_call(function).is_some()
}

/// Whether the current function may add another call site to `module::function`. Struct API
/// wrappers are leaves and stay unlimited.
fn below_call_site_cap(
    state: &FunctionState<'_, '_>,
    module: &ModuleId,
    function: &Function,
) -> bool {
    let cap = state.cfg.max_call_sites_per_callee;
    if cap == 0 || function.is_struct_api_wrapper() {
        return true;
    }
    let target = QualifiedFunction {
        module: module.clone(),
        name: function.name.clone(),
    };
    state.call_sites.get(&target).copied().unwrap_or(0) < cap
}

/// Counts a call site to `module::function`.
fn record_call_site(state: &mut FunctionState<'_, '_>, module: &ModuleId, function: &Function) {
    let target = QualifiedFunction {
        module: module.clone(),
        name: function.name.clone(),
    };
    let count = state.call_sites.entry(target).or_insert(0);
    *count = count.saturating_add(1);
}

/// Restricts calls and packed closures to later siblings to avoid arbitrary call cycles.
/// `recursion_prob` may lift the restriction. Bounded self-recursion is inserted separately.
fn acyclic_sibling_filter(
    u: &mut Unstructured<'_>,
    state: &FunctionState<'_, '_>,
) -> Result<impl Fn(usize) -> bool + use<>> {
    let caller_position = state.function_name.as_ref().and_then(|name| {
        state
            .module_ctx
            .functions
            .iter()
            .position(|function| function.name == *name)
    });
    let allow_recursion = state.cfg.recursion_prob == u8::MAX
        || crate::violations::chance(u, state.cfg.recursion_prob)?;
    Ok(move |position: usize| {
        allow_recursion || caller_position.is_none_or(|caller| position > caller)
    })
}

/// Emits a call to a visible function accepted by `keep` with synthesized arguments.
fn emit_call(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    keep: impl Fn(&Function) -> bool,
) -> Result<bool> {
    // Collect all callable target functions: functions of the current module, plus any public
    // function from `external_modules`. Restricted to functions whose params we know how to
    // synthesize and whose returns we can drain at function exit (primitives, or droppable
    // struct returns — see `call_target_ok`).
    let mut targets: Vec<(ModuleId, &Function)> = Vec::new();
    // `None` inside a script, which can call public functions of published modules.
    let here = state.module_ctx.module_id.clone();
    let callable_siblings = acyclic_sibling_filter(u, state)?;
    let has_signer = state
        .locals
        .iter()
        .any(|local| local.init == LocalInit::Available && is_signer_ref(&local.ty));
    {
        let ctx = ability_ctx(state);
        if let Some(here) = &here {
            for (position, f) in state.module_ctx.functions.iter().enumerate() {
                // A same-module callee's `acquires` would have to be declared by the caller.
                if callable_siblings(position)
                    && keep(f)
                    && f.acquires.is_empty()
                    && call_target_ok(f, &ctx, has_signer)
                {
                    targets.push((here.clone(), f));
                }
            }
        }
        for m in &state.module_ctx.external_modules {
            for f in &m.functions {
                if !keep(f) || !call_target_ok(f, &ctx, has_signer) {
                    continue;
                }
                let visible = match f.visibility {
                    Visibility::Public => true,
                    Visibility::Friend => here
                        .as_ref()
                        .is_some_and(|here| m.friends.iter().any(|fr| fr == here)),
                    Visibility::Private => false,
                };
                if visible {
                    targets.push((m.id.clone(), f));
                }
            }
        }
    }
    targets.retain(|(module, function)| below_call_site_cap(state, module, function));
    if targets.is_empty() {
        return Ok(false);
    }
    let pick = u.choose(&targets)?;
    let target_module = pick.0.clone();
    let target_fun = pick.1.clone();
    state.calls_storing_function |= stores_resource(&target_fun);
    record_call_site(state, &target_module, &target_fun);

    // Instantiate generic type parameters with random primitives (each picked
    // independently per parameter). All primitives satisfy `copy+drop+store`, so
    // constraint satisfaction is trivial; `call_target_ok` ensures the post-substitution
    // signature stays synthesizable through [`push_synthesized_value`].
    let type_args: Vec<Type> = pick_concrete_primitives(u, target_fun.type_params.len())?;
    // Synthesize argument values matching the callee's substituted signature.
    // A recursive callee gets a shallow depth, so nested recursion adds a small factor rather
    // than multiplying large depths.
    let depth_param = is_bounded_recursive(state, &target_module, &target_fun)
        .then(|| crate::recursion::depth_parameter(&target_fun))
        .flatten();
    for (index, p) in target_fun.params.iter().enumerate() {
        let concrete = substitute(&p.ty, &type_args);
        if is_signer_ref(&concrete) {
            push_signer_ref(u, state, instrs)?;
        } else if Some(index) == depth_param {
            let depth = u.int_in_range(0..=NESTED_RECURSION_DEPTH_MAX)?;
            instrs.push(imm_instr("ld_u64", Value::U64(depth)));
            state.push(Type::U64);
        } else {
            push_synthesized_value(u, state, instrs, &concrete)?;
        }
    }
    for _ in &target_fun.params {
        state.pop();
    }
    for ret_ty in &target_fun.return_types {
        let concrete = substitute(ret_ty, &type_args);
        state.push(concrete);
    }
    instrs.push(Instr {
        opcode: "call".to_string(),
        operands: vec![Operand::Function(
            QualifiedFunction {
                module: target_module,
                name: target_fun.name.clone(),
            },
            type_args,
        )],
    });
    // A bounds violation corrupts the first function definition's handle index after assembly.
    // It affects the module's table references independently of this call instruction.
    if state.wants_violation(u, DimensionFlag::Bounds)? {
        state
            .post_process_intents
            .push(crate::ir::PostProcessIntent::CorruptBounds);
    }
    Ok(true)
}

/// `valid_instantiation_loops` violation: a generic function calls itself with `vector<T0>`
/// in place of `T0`, which the `instantiation_loops` pass rejects with
/// `LOOP_IN_INSTANTIATION_GRAPH`. Params must not mention `T0` so arguments can be
/// synthesized, and returns are popped.
fn try_instantiation_loop_call(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let (Some(module_id), Some(name)) = (
        state.module_ctx.module_id.clone(),
        state.function_name.clone(),
    ) else {
        return Ok(false);
    };
    let Some(self_fun) = state.module_ctx.functions.iter().find(|f| f.name == name) else {
        return Ok(false);
    };
    let ctx = ability_ctx(state);
    let self_fun = self_fun.clone();
    let mentions_t0 = |ty: &Type| crate::types::mentions_type_param(ty, 0);
    if self_fun.type_params.is_empty()
        || !self_fun.acquires.is_empty()
        || self_fun.params.iter().any(|p| mentions_t0(&p.ty))
        || !self_fun
            .params
            .iter()
            .all(|p| is_synthesizable_primitive(&p.ty))
        || !self_fun
            .return_types
            .iter()
            .all(|ty| !mentions_t0(ty) && has_drop(ty, &ctx))
    {
        return Ok(false);
    }
    if !state.wants_violation(u, DimensionFlag::InstantiationLoops)? {
        return Ok(false);
    }
    let type_args: Vec<Type> = (0..self_fun.type_params.len() as u16)
        .map(|idx| match idx {
            0 => Type::Vector(Box::new(Type::TypeParam(0))),
            _ => Type::TypeParam(idx),
        })
        .collect();
    for param in &self_fun.params {
        push_synthesized_value(u, state, instrs, &param.ty)?;
    }
    for _ in &self_fun.params {
        state.pop();
    }
    state.record_violation(DimensionFlag::InstantiationLoops);
    instrs.push(Instr {
        opcode: "call".to_string(),
        operands: vec![Operand::Function(
            QualifiedFunction {
                module: module_id,
                name,
            },
            type_args,
        )],
    });
    for _ in &self_fun.return_types {
        instrs.push(simple_op("pop"));
    }
    Ok(true)
}

/// Pushes a copy of one of the caller's available `&signer` parameters.
fn push_signer_ref(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<()> {
    let signers: Vec<Identifier> = state
        .locals
        .iter()
        .filter(|local| local.init == LocalInit::Available && is_signer_ref(&local.ty))
        .map(|local| local.name.clone())
        .collect();
    let name = u.choose(&signers)?.clone();
    instrs.push(name_instr("copy_loc", name));
    // Refs carry no borrow-graph entry under the existing convention (see `try_move_to`).
    state.push(Type::Reference(false, Box::new(Type::Signer)));
    Ok(())
}

/// Whether `f` can be called with synthesized arguments. `&signer` parameters are accepted
/// when the caller has a `&signer` to pass on.
fn call_target_ok(f: &Function, ctx: &AbilityCtx<'_>, has_signer: bool) -> bool {
    // Arguments must be synthesizable primitives after substitution, or available signers.
    // Returns may also be structs, including foreign ones, but must have `drop` so the
    // final-block drain can discard any unused values.
    let n = f.type_params.len();
    if !f.params.iter().all(|p| {
        is_synthesizable_primitive_after_subst(&p.ty, n) || (has_signer && is_signer_ref(&p.ty))
    }) {
        return false;
    }
    // `u64` has the same abilities as every primitive chosen at call sites, so it represents
    // their effect on the instantiated return type's `drop` ability.
    let representative_args = vec![Type::U64; n];
    for ret in &f.return_types {
        let concrete = substitute(ret, &representative_args);
        if !is_returnable(&concrete) {
            return false;
        }
        if !has_drop(&concrete, ctx) {
            return false;
        }
    }
    true
}

/// Return shapes accepted by `try_call`: primitives and structs, including foreign structs.
fn is_returnable(ty: &Type) -> bool {
    is_synthesizable_primitive(ty) || matches!(ty, Type::Struct(_, _))
}

/// Like [`is_synthesizable_primitive`] but accepts `TypeParam(_)` referring to one of the
/// function's type parameters, which will be instantiated with primitives at the call site.
fn is_synthesizable_primitive_after_subst(ty: &Type, n_type_params: usize) -> bool {
    matches!(ty, Type::TypeParam(idx) if usize::from(*idx) < n_type_params)
        || is_synthesizable_primitive(ty)
}

/// Primitive types whose values can be synthesized directly: integers, bools, and addresses.
pub(crate) fn is_synthesizable_primitive(t: &Type) -> bool {
    is_integer(t) || matches!(t, Type::Bool | Type::Address)
}

/// Pushes an integer literal of type `ty`. First samples boundary values using
/// [`Config::boundary_literal_prob`], then chooses uniform or small values using
/// [`Config::random_literal_prob`]. Small values reduce arithmetic aborts; boundary values
/// exercise overflow, underflow, and division by zero.
pub(crate) fn push_integer_literal(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    ty: &Type,
) -> Result<()> {
    let (opcode, value) = integer_literal(u, state.cfg, ty)?;
    instrs.push(imm_instr(opcode, value));
    state.push(ty.clone());
    Ok(())
}

/// An integer literal of type `ty` drawn as [`push_integer_literal`] describes, with its load
/// opcode.
pub(crate) fn integer_literal(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    ty: &Type,
) -> Result<(&'static str, Value)> {
    let kind = if crate::violations::chance(u, cfg.boundary_literal_prob)? {
        LiteralKind::Boundary(u.int_in_range(0u8..=4)?)
    } else if crate::violations::chance(u, cfg.random_literal_prob)? {
        LiteralKind::Random
    } else {
        LiteralKind::Small
    };
    macro_rules! literal {
        ($variant:ident, $ty:ty, $opcode:literal) => {{
            let value: $ty = match kind {
                LiteralKind::Boundary(0) => 0,
                LiteralKind::Boundary(1) => 1,
                LiteralKind::Boundary(2) => <$ty>::MAX,
                LiteralKind::Boundary(3) => <$ty>::MIN,
                LiteralKind::Boundary(_) => <$ty>::MAX - 1,
                LiteralKind::Random => u.arbitrary()?,
                LiteralKind::Small => <$ty>::try_from(u.int_in_range(0u8..=SMALL_LITERAL_MAX)?)
                    .expect("small literals fit every integer type"),
            };
            ($opcode, Value::$variant(value))
        }};
    }
    macro_rules! wide_literal {
        ($variant:ident, $ty:ty, $opcode:literal) => {{
            let value = match kind {
                LiteralKind::Boundary(0) => <$ty>::ZERO,
                LiteralKind::Boundary(1) => <$ty>::ONE,
                LiteralKind::Boundary(2) => <$ty>::MAX,
                LiteralKind::Boundary(3) => <$ty>::MIN,
                LiteralKind::Boundary(_) => {
                    <$ty>::checked_sub(<$ty>::MAX, <$ty>::ONE).expect("maximum exceeds one")
                },
                LiteralKind::Random => <$ty>::from_le_bytes(u.arbitrary()?),
                LiteralKind::Small => <$ty>::from(u.int_in_range(0u8..=SMALL_LITERAL_MAX)?),
            };
            ($opcode, Value::$variant(value))
        }};
    }
    let (opcode, value) = match ty {
        Type::U8 => literal!(U8, u8, "ld_u8"),
        Type::U16 => literal!(U16, u16, "ld_u16"),
        Type::U32 => literal!(U32, u32, "ld_u32"),
        Type::U64 => literal!(U64, u64, "ld_u64"),
        Type::U128 => literal!(U128, u128, "ld_u128"),
        Type::I8 => literal!(I8, i8, "ld_i8"),
        Type::I16 => literal!(I16, i16, "ld_i16"),
        Type::I32 => literal!(I32, i32, "ld_i32"),
        Type::I64 => literal!(I64, i64, "ld_i64"),
        Type::I128 => literal!(I128, i128, "ld_i128"),
        Type::U256 => wide_literal!(U256, move_core_types::int256::U256, "ld_u256"),
        Type::I256 => wide_literal!(I256, move_core_types::int256::I256, "ld_i256"),
        Type::Bool
        | Type::Address
        | Type::Signer
        | Type::Vector(_)
        | Type::Struct(..)
        | Type::Reference(..)
        | Type::TypeParam(_)
        | Type::Function { .. } => {
            return Err(crate::error::Error::GenerationStuck(
                "integer literal requested for a non-integer type",
            ))
        },
    };
    Ok((opcode, value))
}

/// Maximum magnitude of a small integer literal, limiting arithmetic overflow and cast failures.
const SMALL_LITERAL_MAX: u8 = 100;

#[derive(Clone, Copy)]
enum LiteralKind {
    Small,
    /// Index into zero, one, maximum, minimum, maximum minus one.
    Boundary(u8),
    Random,
}

/// Pushes a synthesized primitive, vector, or non-generic local struct or enum value.
/// Callers must check [`is_synthesizable`] to bound recursive construction. Unsupported
/// types return `GenerationStuck`; input exhaustion can also fail generation.
pub(crate) fn push_synthesized_value(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    ty: &Type,
) -> Result<()> {
    match ty {
        Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256 => push_integer_literal(u, state, instrs, ty)?,
        Type::Bool => {
            let b: bool = u.arbitrary()?;
            instrs.push(simple_op(if b { "ld_true" } else { "ld_false" }));
            state.push(Type::Bool);
        },
        Type::Address => {
            let addr = pick_account_address(u)?;
            let _ = state.record_ld_const_emit();
            instrs.push(ld_const_instr(Type::Address, Value::Address(addr)));
            state.push(Type::Address);
        },
        Type::Vector(elem) => {
            let count = u.int_in_range(0..=SYNTHESIZED_VECTOR_LEN_MAX)?;
            for _ in 0..count {
                push_synthesized_value(u, state, instrs, elem)?;
            }
            for _ in 0..count {
                state.pop();
            }
            instrs.push(Instr {
                opcode: "vec_pack".to_string(),
                operands: vec![
                    Operand::Type((**elem).clone()),
                    Operand::Imm(Value::U64(count)),
                ],
            });
            state.push(ty.clone());
        },
        Type::Struct(qs, type_args)
            if type_args.is_empty() && state.module_ctx.local_struct(qs).is_some() =>
        {
            let decl = state
                .module_ctx
                .local_struct(qs)
                .cloned()
                .expect("checked by the guard");
            let (fields, variant) = match decl.layout {
                StructLayout::Singleton(fields) => (fields, None),
                StructLayout::Variants(mut variants) => {
                    let variant = variants.swap_remove(u.int_in_range(0..=variants.len() - 1)?);
                    (variant.fields, Some(variant.name))
                },
            };
            for field in &fields {
                push_synthesized_value(u, state, instrs, &field.ty)?;
            }
            for _ in &fields {
                state.pop();
            }
            let mut operands = vec![Operand::Struct(qs.clone(), Vec::new())];
            let opcode = match variant {
                Some(name) => {
                    operands.push(Operand::Local(name));
                    "pack_variant"
                },
                None => "pack",
            };
            instrs.push(Instr {
                opcode: opcode.to_string(),
                operands,
            });
            state.push(ty.clone());
        },
        _ => {
            return Err(crate::error::Error::GenerationStuck(
                "call synth: unsupported argument type",
            ));
        },
    }
    Ok(())
}

/// Maximum vector and struct nesting accepted by [`is_synthesizable`].
const SYNTHESIS_DEPTH_MAX: usize = 3;

/// Most elements in a vector [`push_synthesized_value`] builds.
const SYNTHESIZED_VECTOR_LEN_MAX: u64 = 4;

/// Whether [`push_synthesized_value`] can build a value of `ty`: a primitive, or a vector or a
/// non-generic struct or enum of the current module whose elements or fields it can build,
/// nested at most [`SYNTHESIS_DEPTH_MAX`] deep.
pub(crate) fn is_synthesizable(ty: &Type, state: &FunctionState<'_, '_>) -> bool {
    synthesizable_within(ty, state, SYNTHESIS_DEPTH_MAX)
}

fn synthesizable_within(ty: &Type, state: &FunctionState<'_, '_>, depth: usize) -> bool {
    match ty {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address => true,
        Type::Signer | Type::Reference(..) | Type::TypeParam(_) | Type::Function { .. } => false,
        Type::Vector(elem) => depth
            .checked_sub(1)
            .is_some_and(|depth| synthesizable_within(elem, state, depth)),
        Type::Struct(qs, type_args) => {
            let Some(depth) = depth.checked_sub(1) else {
                return false;
            };
            type_args.is_empty()
                && state.module_ctx.local_struct(qs).is_some_and(|decl| {
                    decl.type_params.is_empty()
                        && match &decl.layout {
                            StructLayout::Singleton(fields) => fields
                                .iter()
                                .all(|field| synthesizable_within(&field.ty, state, depth)),
                            StructLayout::Variants(variants) => {
                                !variants.is_empty()
                                    && variants.iter().all(|variant| {
                                        variant.fields.iter().all(|field| {
                                            synthesizable_within(&field.ty, state, depth)
                                        })
                                    })
                            },
                        }
                })
        },
    }
}

fn try_freeze_ref(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(true, inner) = top else {
        return Ok(false);
    };
    // Convert `&mut T` to `&T` in both the operand stack and the borrow graph, keeping the
    // borrow's id.
    let (_, ref_id) = state.pop_with_ref().expect("non-empty stack");
    let new_ty = Type::Reference(false, inner);
    match ref_id {
        Some(ref_id) => {
            state.borrows.freeze(ref_id);
            state.push_ref(new_ty, ref_id);
        },
        None => state.push(new_ty),
    }
    instrs.push(simple_op("freeze_ref"));
    Ok(true)
}

fn try_unpack(
    _u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let Some(Type::Struct(qs, type_args)) = state.top().cloned() else {
        return Ok(false);
    };
    let Some(decl) = state.module_ctx.local_struct(&qs) else {
        return Ok(false);
    };
    if decl.type_params.len() != type_args.len() {
        return Ok(false);
    }
    let StructLayout::Singleton(fields) = &decl.layout else {
        return Ok(false);
    };
    let field_types: Vec<Type> = fields
        .iter()
        .map(|f| substitute(&f.ty, &type_args))
        .collect();
    state.pop();
    for ft in field_types {
        state.push(ft);
    }
    instrs.push(struct_instr("unpack", qs, type_args));
    Ok(true)
}

fn try_pack_variant(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_variants {
        return Ok(false);
    }
    let Some(module_id) = state.module_ctx.module_id.clone() else {
        return Ok(false);
    };
    let ctx = ability_ctx(state);
    // (decl_idx, variant_idx, type_args).
    let mut candidates: Vec<(usize, usize, Vec<Type>)> = Vec::new();
    for (i, decl) in state.module_ctx.structs.iter().enumerate() {
        let StructLayout::Variants(variants) = &decl.layout else {
            continue;
        };
        for (vi, variant) in variants.iter().enumerate() {
            let raw_field_types: Vec<Type> = variant.fields.iter().map(|f| f.ty.clone()).collect();
            if let Some((type_args, true)) =
                infer_pack_type_args(u, state, &ctx, &module_id, decl, &raw_field_types)?
            {
                candidates.push((i, vi, type_args));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(false);
    }
    let pick_idx = u.int_in_range(0..=candidates.len() - 1)?;
    let (decl_idx, variant_idx, type_args) = candidates.swap_remove(pick_idx);
    let decl = &state.module_ctx.structs[decl_idx];
    let StructLayout::Variants(variants) = &decl.layout else {
        return Ok(false);
    };
    let variant = &variants[variant_idx];
    let qs = QualifiedStruct {
        module: module_id,
        name: decl.name.clone(),
    };
    let variant_name = variant.name.clone();
    let field_count = variant.fields.len();
    for _ in 0..field_count {
        state.pop();
    }
    state.push(Type::Struct(qs.clone(), type_args.clone()));
    instrs.push(instr("pack_variant", vec![
        Operand::Struct(qs, type_args),
        Operand::Local(variant_name),
    ]));
    Ok(true)
}

fn try_unpack_variant(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_variants {
        return Ok(false);
    }
    let Some(Type::Struct(qs, type_args)) = state.top().cloned() else {
        return Ok(false);
    };
    let Some(decl) = state.module_ctx.local_struct(&qs).cloned() else {
        return Ok(false);
    };
    if decl.type_params.len() != type_args.len() {
        return Ok(false);
    }
    let StructLayout::Variants(variants) = decl.layout else {
        return Ok(false);
    };
    if variants.is_empty() {
        return Ok(false);
    }
    let variant_idx = u.int_in_range(0..=(variants.len() - 1))?;
    let variant = &variants[variant_idx];
    let field_types: Vec<Type> = variant
        .fields
        .iter()
        .map(|f| substitute(&f.ty, &type_args))
        .collect();
    let variant_name = variant.name.clone();
    state.pop();
    for ft in field_types {
        state.push(ft);
    }
    instrs.push(instr("unpack_variant", vec![
        Operand::Struct(qs, type_args),
        Operand::Local(variant_name),
    ]));
    Ok(true)
}

fn try_test_variant(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_variants {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(_, inner) = top else {
        return Ok(false);
    };
    let Type::Struct(qs, type_args) = *inner else {
        return Ok(false);
    };
    let Some(decl) = state.module_ctx.local_struct(&qs).cloned() else {
        return Ok(false);
    };
    if decl.type_params.len() != type_args.len() {
        return Ok(false);
    }
    let StructLayout::Variants(variants) = decl.layout else {
        return Ok(false);
    };
    if variants.is_empty() {
        return Ok(false);
    }
    let variant_idx = u.int_in_range(0..=(variants.len() - 1))?;
    let variant_name = variants[variant_idx].name.clone();
    state.pop();
    state.push(Type::Bool);
    instrs.push(instr("test_variant", vec![
        Operand::Struct(qs, type_args),
        Operand::Local(variant_name),
    ]));
    Ok(true)
}

fn try_borrow_variant_field(
    want_mut: bool,
    opcode: &str,
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_variants || !state.cfg.enable_references {
        return Ok(false);
    }
    let Some(top) = state.top().cloned() else {
        return Ok(false);
    };
    let Type::Reference(parent_is_mut, parent_inner) = top else {
        return Ok(false);
    };
    if want_mut && !parent_is_mut {
        return Ok(false);
    }
    let Type::Struct(qs, type_args) = *parent_inner else {
        return Ok(false);
    };
    let parent_type_args = type_args;
    let Some(decl) = state.module_ctx.local_struct(&qs).cloned() else {
        return Ok(false);
    };
    if decl.type_params.len() != parent_type_args.len() {
        return Ok(false);
    }
    let StructLayout::Variants(variants) = decl.layout else {
        return Ok(false);
    };
    if variants.is_empty() {
        return Ok(false);
    }
    let variant_idx = u.int_in_range(0..=(variants.len() - 1))?;
    let variant = &variants[variant_idx];
    if variant.fields.is_empty() {
        return Ok(false);
    }
    let field_idx = u.int_in_range(0..=(variant.fields.len() - 1) as u16)?;
    let field = &variant.fields[field_idx as usize];
    let child_inner_ty = substitute(&field.ty, &parent_type_args);

    let parent_ref_id = state.top_ref_id();
    let (parent_root, parent_path) = match parent_ref_id.and_then(|id| state.borrows.get(id)) {
        Some(entry) => (entry.root.clone(), entry.path.clone()),
        None => return Ok(false),
    };
    state.pop();
    let mut child_path = parent_path;
    child_path.push(BorrowStep::VariantField {
        variant: variant_idx as u16,
        field: field_idx,
    });
    let child_mut = Mutability::of(want_mut);
    let child_ty = Type::Reference(want_mut, Box::new(child_inner_ty));
    let child_id = state
        .borrows
        .force_create(parent_root, child_path, child_mut, None);
    state.push_ref(child_ty, child_id);
    instrs.push(Instr {
        opcode: opcode.to_string(),
        operands: vec![Operand::VariantFields {
            owner: qs,
            type_args: parent_type_args,
            fields: vec![(variant.name.clone(), field.name.clone())],
        }],
    });
    Ok(true)
}

// ============================================================================
// Helpers
// ============================================================================

pub(crate) fn ability_ctx<'a>(state: &'a FunctionState<'_, '_>) -> AbilityCtx<'a> {
    AbilityCtx {
        local_module_id: state.module_ctx.module_id.as_ref(),
        structs: &state.module_ctx.structs,
        external_modules: &state.module_ctx.external_modules,
        type_param_constraints: &state.type_param_constraints,
    }
}

pub(crate) fn instr(opcode: &str, operands: Vec<Operand>) -> Instr {
    Instr {
        opcode: opcode.to_string(),
        operands,
    }
}

pub(crate) fn simple_op(opcode: &str) -> Instr {
    instr(opcode, vec![])
}

pub(crate) fn imm_instr(opcode: &str, value: Value) -> Instr {
    instr(opcode, vec![Operand::Imm(value)])
}

pub(crate) fn ld_const_instr(ty: Type, value: Value) -> Instr {
    instr("ld_const", vec![Operand::Type(ty), Operand::Imm(value)])
}

pub(crate) fn name_instr(opcode: &str, name: Identifier) -> Instr {
    instr(opcode, vec![Operand::Local(name)])
}

pub(crate) fn label_instr(opcode: &str, label: BlockId) -> Instr {
    instr(opcode, vec![Operand::Label(label)])
}

pub(crate) fn type_instr(opcode: &str, ty: Type) -> Instr {
    instr(opcode, vec![Operand::Type(ty)])
}

pub(crate) fn struct_instr(opcode: &str, owner: QualifiedStruct, type_args: Vec<Type>) -> Instr {
    instr(opcode, vec![Operand::Struct(owner, type_args)])
}

/// A call of the stdlib function `0x1::<module>::<name><type_args>`.
pub(crate) fn std_call(module: &str, name: &str, type_args: Vec<Type>) -> Instr {
    let module = ModuleId::new(
        AccountAddress::ONE,
        Identifier::new(module).expect("stdlib module names are identifiers"),
    );
    let name = Identifier::new(name).expect("stdlib function names are identifiers");
    instr("call", vec![Operand::Function(
        QualifiedFunction { module, name },
        type_args,
    )])
}
