// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Context for lowering stackless exec IR to micro-ops.
//!
//! Builds frame layout information (slot offsets/sizes) needed by the lowerer.
//! All lookups are O(1) via indexed Vecs — no maps.

use crate::stackless_exec_ir::{FunctionIR, Instr, ModuleIR};
use anyhow::Result;
use mono_move_core::{
    types::{align_up, view_type, Alignment, InternedType, Size},
    FRAME_METADATA_SIZE,
};
use move_binary_format::access::ModuleAccess;

/// Returns the (size, alignment) of a concrete interned type, or None if the
/// type is not concrete (e.g., contains type parameters or unresolved structs).
pub fn type_size_and_align(ty: InternedType) -> Option<(Size, Alignment)> {
    view_type(ty).size_and_align()
}

#[derive(Clone, Copy, Debug)]
pub struct SlotInfo {
    pub offset: u32,
    pub size: u32,
    /// Byte alignment required by this slot's type.
    pub align: u32,
}

pub struct CallSiteInfo {
    pub callee_func_id: u32,
    pub arg_write_slots: Vec<SlotInfo>,
    pub ret_read_slots: Vec<SlotInfo>,
    pub param_types: Vec<InternedType>,
    pub ret_types: Vec<InternedType>,
}

pub struct LoweringContext {
    pub home_slots: Vec<SlotInfo>,
    pub frame_data_size: u32,
    pub call_sites: Vec<CallSiteInfo>,
    pub return_slots: Vec<SlotInfo>,
    /// Maximum number of Xfer slots needed across all call sites in this function.
    pub num_xfer_slots: u16,
}

/// Try to build a LoweringContext for a monomorphic function.
/// Returns `Ok(None)` if any type is not concrete (e.g. type parameters, structs).
/// Returns `Err` for unexpected failures.
pub fn try_build_context(
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
) -> Result<Option<LoweringContext>> {
    // Use an inner function that returns Option to keep `?` ergonomic for
    // non-concrete type checks, then wrap the result.
    let inner = try_build_context_inner(module_ir, func_ir);
    match inner {
        Some(result) => result.map(Some),
        None => Ok(None),
    }
}

/// Returns `None` if any type is not concrete.
/// Returns `Some(Ok(ctx))` on success, `Some(Err(..))` on unexpected failure.
fn try_build_context_inner(
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
) -> Option<Result<LoweringContext>> {
    // 1. Compute home slot layout with proper alignment.
    //
    // Slots are laid out linearly in declaration order, padding each to its
    // natural alignment. This can leave gaps between a small slot followed
    // by a higher-aligned one.
    //
    // TODO: consider a smarter packing (e.g. sort by descending alignment,
    // or bin-pack smaller slots into padding holes) to shrink frame size.
    let home_slots = layout_slots(0, &func_ir.home_slot_types)?;
    let frame_data_size = home_slots.last().map(|s| s.offset + s.size).unwrap_or(0);

    // 2. Build return_slots from this function's own signature.
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    let own_ret_types = module_ir.module.interned_types_at(own_handle.return_);
    let return_slots = layout_slots(0, own_ret_types)?;

    // 3. Walk Call/CallGeneric instructions, looking up each callee's sig
    //    in the module-level tables.
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;
    let mut call_sites = Vec::new();

    for instr in func_ir.instrs() {
        let handle_idx = match instr {
            Instr::Call(_, idx, _) => *idx,
            Instr::CallGeneric(_, idx, _) => {
                module_ir.module.function_instantiation_at(*idx).handle
            },
            _ => continue,
        };

        let callee_handle = module_ir.module.function_handle_at(handle_idx);
        let param_types = module_ir.module.interned_types_at(callee_handle.parameters);
        let ret_types = module_ir.module.interned_types_at(callee_handle.return_);

        let callee_func_id = handle_idx.0 as u32;

        // Param and return areas share `callee_base` (callee's frame start).
        let arg_write_slots = layout_slots(callee_base, param_types)?;
        let ret_read_slots = layout_slots(callee_base, ret_types)?;

        call_sites.push(CallSiteInfo {
            callee_func_id,
            arg_write_slots,
            ret_read_slots,
            param_types: param_types.to_vec(),
            ret_types: ret_types.to_vec(),
        });
    }

    Some(Ok(LoweringContext {
        home_slots,
        frame_data_size,
        call_sites,
        return_slots,
        num_xfer_slots: func_ir.num_xfer_slots,
    }))
}

/// Lays out a contiguous sequence of slots starting at `base`, padding each
/// to its natural alignment. Returns `None` if any type is not concrete.
fn layout_slots(base: u32, types: &[InternedType]) -> Option<Vec<SlotInfo>> {
    let mut slots = Vec::with_capacity(types.len());
    let mut offset = base;
    for ty in types {
        let (size, alignment) = type_size_and_align(*ty)?;
        offset = align_up(offset, alignment);
        slots.push(SlotInfo {
            offset,
            size,
            align: alignment,
        });
        offset += size;
    }
    Some(slots)
}
