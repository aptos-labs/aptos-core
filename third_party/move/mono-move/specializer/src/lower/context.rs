// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Context for lowering stackless exec IR to micro-ops.
//!
//! Builds frame layout information (slot offsets/sizes) needed by the lowerer.
//! All lookups are O(1) via indexed Vecs — no maps.

use crate::{
    lower::lower_function,
    stackless_exec_ir::{FunctionIR, Instr, ModuleIR},
};
use anyhow::{anyhow, bail, Result};
use mono_move_core::{
    align_up_u32,
    interner::{InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, Alignment, FieldLayout, InternedType, Size, Type},
    Code, FieldTypes, FrameLayoutInfo, FrameOffset, Function, MicroOp, MicroOpGasSchedule,
    SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use mono_move_gas::GasInstrumentor;
use move_binary_format::access::ModuleAccess;
use shared_dsa::UnorderedSet;

/// Minimum slot alignment supported by the current micro-op set.
///
/// Micro-ops like `StoreImm8`, `Move8`, `AddU64`, etc. read/write a fixed
/// 8 bytes regardless of the IR-level type's actual size, so any slot whose
/// alignment is less than 8 (`u8`/`u16`/`u32`/`bool`) would be silently
/// overrun by adjacent-slot data. The same constraint also keeps
/// `args_and_locals_size` 8-aligned, which keeps `callee_base = caller's
/// args_and_locals_size + FRAME_METADATA_SIZE` 8-aligned and the metadata
/// `write_u64`s well-defined. Until we have proper small-type micro-ops,
/// the lowering refuses to handle slots with `align < MIN_SLOT_ALIGN`.
const MIN_SLOT_ALIGN: u32 = 8;

fn check_supported_alignment<T>(
    slots: &[T],
    align_of: impl Fn(&T) -> u32,
    context: &str,
) -> Result<()> {
    if let Some(bad_align) = slots.iter().map(align_of).find(|&a| a < MIN_SLOT_ALIGN) {
        bail!(
            "{}: slot align {} < {} not yet supported (u64-aligned types only)",
            context,
            bad_align,
            MIN_SLOT_ALIGN
        );
    }
    Ok(())
}

/// Returns the (size, alignment) of a concrete interned type, or None if the
/// type is not concrete (e.g., contains type parameters or unresolved structs).
pub fn type_size_and_align(ty: InternedType) -> Option<(Size, Alignment)> {
    view_type(ty).size_and_align()
}

/// Byte-level location of a typed value in the current function's frame.
#[derive(Clone, Copy, Debug)]
pub struct SlotInfo {
    pub offset: FrameOffset,
    /// Width of the type currently bound to this slot.
    pub size: u32,
    pub align: u32,
}

/// A frame slot paired with the type of its value.
#[derive(Clone, Copy, Debug)]
pub struct TypedSlot {
    pub slot: SlotInfo,
    pub ty: InternedType,
}

/// Pre-computed layout for one call instruction. Arg and ret slots are
/// caller-frame addresses laid out from `callee_base`.
pub struct CallSiteInfo {
    pub callee_module_id: InternedModuleId,
    pub callee_func_name: InternedIdentifier,
    pub arg_slots: Vec<TypedSlot>,
    pub ret_slots: Vec<TypedSlot>,
}

/// Frame layout for one function.
/// [TODO]: a few raw-`u32` fields remain (sizes/alignments); migrate
/// them to dedicated newtypes for consistency with `FrameOffset`.
pub struct LoweringContext {
    pub home_slots: Vec<SlotInfo>,
    /// End offset of the home-slot region; feeds `callee_base`.
    pub frame_data_size: u32,
    /// In IR order; indexed by `LoweringState::call_site_cursor`.
    pub call_sites: Vec<CallSiteInfo>,
    /// Where `Instr::Ret` writes before the `Return` micro-op. Laid out
    /// from offset 0 so addresses match the caller's `ret_slots`.
    pub return_slots: Vec<SlotInfo>,
    pub num_xfer_positions: u16,
    /// Frame offset of the cycle-breaking scratch slot used by
    /// `parallel_copy::emit_parallel_copy` for `Instr::Ret`. Sized to
    /// hold the widest value in this function's `return_slots` (Ret
    /// copies have matching src/dst types, so that's a safe upper
    /// bound). `0` (with `scratch_size == 0`) when the function has no
    /// returns. The scratch lives at the end of the home region, so
    /// callees see a slightly bumped `callee_base`. Only inspected for
    /// at most one micro-op of any pair, so it doesn't need GC
    /// tracking.
    pub scratch_offset: FrameOffset,
    /// Width of the scratch slot in bytes, or `0` if no scratch is
    /// reserved.
    pub scratch_size: u32,
}

/// Try to build a `LoweringContext` for a monomorphic function.
/// Returns `Ok(None)` if any type is not concrete (e.g. type parameters
/// or unresolved structs); `Err` for unsupported alignments and other
/// unexpected failures.
pub fn try_build_context(
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
) -> Result<Option<LoweringContext>> {
    // 1. Compute home slot layout with natural alignment padding.
    //
    // Slots are laid out linearly in declaration order, padding each to
    // its alignment. This can leave gaps between a small slot followed
    // by a higher-aligned one.
    //
    // TODO: consider a smarter packing (e.g. sort by descending
    // alignment, or bin-pack smaller slots into padding holes) to
    // shrink frame size.
    let Some(home_slots) = layout_slots(0, &func_ir.home_slot_types) else {
        return Ok(None);
    };
    check_supported_alignment(&home_slots, |s| s.align, "home slot")?;
    // `frame_data_size` must be `MIN_SLOT_ALIGN`-aligned so that
    // `callee_base = frame_data_size + FRAME_METADATA_SIZE` is also
    // aligned (the runtime writes saved pc/fp/func_id as `u64`s
    // starting at `frame_data_size`).
    let mut frame_data_size = align_up_u32(
        home_slots.last().map(|s| s.offset.0 + s.size).unwrap_or(0),
        MIN_SLOT_ALIGN,
    );

    // 2. Build `return_slots` from this function's own signature.
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    let own_ret_types = module_ir.module.interned_types_at(own_handle.return_);
    let Some(return_slots) = layout_slots(0, own_ret_types) else {
        return Ok(None);
    };
    check_supported_alignment(&return_slots, |s| s.align, "return slot")?;

    // The return values are written at offsets [0, ret_size) of the function's
    // own frame. They share storage with the args/locals region (the calling
    // convention reuses that space on return), so `args_and_locals_size` must
    // be ≥ ret_size — otherwise the return writes would land in frame metadata.
    // Leaf functions with no params/locals but a non-empty return signature
    // trip this without the bump.
    let ret_end = align_up_u32(
        return_slots
            .last()
            .map(|s| s.offset.0 + s.size)
            .unwrap_or(0),
        MIN_SLOT_ALIGN,
    );
    if ret_end > frame_data_size {
        frame_data_size = ret_end;
    }

    // 3. Reserve a scratch slot at the tail of the home region for
    //    `Ret` cycle-breaking — `return_slots` overlap home, so swaps
    //    like `(b, a)` form copy cycles that `emit_parallel_copy`
    //    routes through this slot. Sized to the widest return slot
    //    (Ret copies are type-matched).
    //
    //    Skipped when fewer than 2 return values: a cycle requires at
    //    least two copies, so single-return (and no-return) functions
    //    can never need scratch.
    //
    //    TODO: tighten further by walking the IR's `Ret` instructions
    //    and detecting whether any copy graph actually contains a
    //    cycle. That would let multi-return functions whose Ret
    //    copies are all identity or otherwise acyclic skip the slot
    //    too, at the cost of ~O(N²) per Ret graph cycle check.
    //    We may also want to consider stricter bounding on number of
    //    return values in the bytecode verifier.
    let max_value_width: u32 = return_slots.iter().map(|s| s.size).max().unwrap_or(0);
    let (scratch_offset, scratch_size) = if return_slots.len() >= 2 && max_value_width > 0 {
        let offset = align_up_u32(frame_data_size, MIN_SLOT_ALIGN);
        let size = align_up_u32(max_value_width, MIN_SLOT_ALIGN);
        frame_data_size = offset + size;
        (FrameOffset(offset), size)
    } else {
        (FrameOffset(0), 0)
    };

    // 4. Walk `Call`/`CallGeneric` instructions and lay out each callee's
    //    arg/ret region from `callee_base`.
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;
    let mut call_sites = Vec::new();
    for instr in func_ir.instrs() {
        let callee_handle = match instr {
            Instr::Call(_, idx, _) => module_ir.module.function_handle_at(*idx),
            Instr::CallGeneric(_, idx, _) => {
                let inst = module_ir.module.function_instantiation_at(*idx);
                module_ir.module.function_handle_at(inst.handle)
            },
            _ => continue,
        };
        let param_types = module_ir.module.interned_types_at(callee_handle.parameters);
        let ret_types = module_ir.module.interned_types_at(callee_handle.return_);
        let Some(arg_slots) = layout_typed_slots_contiguously(callee_base, param_types) else {
            return Ok(None);
        };
        let Some(ret_slots) = layout_typed_slots_contiguously(callee_base, ret_types) else {
            return Ok(None);
        };
        check_supported_alignment(&arg_slots, |s| s.slot.align, "callee arg")?;
        check_supported_alignment(&ret_slots, |s| s.slot.align, "callee ret")?;

        let callee_module_id = module_ir.module.module_id_at(callee_handle.module);
        let callee_func_name = module_ir.module.interned_identifier_at(callee_handle.name);
        call_sites.push(CallSiteInfo {
            callee_module_id,
            callee_func_name,
            arg_slots,
            ret_slots,
        });
    }

    Ok(Some(LoweringContext {
        home_slots,
        frame_data_size,
        call_sites,
        return_slots,
        num_xfer_positions: func_ir.num_xfer_positions,
        scratch_offset,
        scratch_size,
    }))
}

/// Lays out a contiguous sequence of typed slots starting at `base`,
/// padding each to its natural alignment.
///
/// Returns `None` if any type is not concrete.
fn layout_typed_slots_contiguously(base: u32, types: &[InternedType]) -> Option<Vec<TypedSlot>> {
    let mut slots = Vec::with_capacity(types.len());
    let mut offset = base;
    for &ty in types {
        let (size, align) = type_size_and_align(ty)?;
        offset = align_up_u32(offset, align);
        slots.push(TypedSlot {
            slot: SlotInfo {
                offset: FrameOffset(offset),
                size,
                align,
            },
            ty,
        });
        offset += size;
    }
    Some(slots)
}

/// Discards the type tags from [`layout_typed_slots_contiguously`].
/// Currently the only layout strategy used; callers whose correctness
/// doesn't depend on contiguous layout (e.g., home slots, where a
/// future bin-packer could shrink the frame) could be migrated to a
/// non-contiguous strategy without affecting arg/ret callers.
fn layout_slots(base: u32, types: &[InternedType]) -> Option<Vec<SlotInfo>> {
    Some(
        layout_typed_slots_contiguously(base, types)?
            .into_iter()
            .map(|ts| ts.slot)
            .collect(),
    )
}

/// Provides context to specializer so it can obtain external information
/// about types (e.g., their sizes, fields of structs if available) as well
/// as publish new information about types discovered to the context.
pub trait SpecializerContext {
    /// Returns fields of a struct or variants with fields of an enum. If
    /// this information is not available in context, returns [`None`].
    fn get_fields(
        &mut self,
        module_id: &InternedModuleId,
        nominal_name: &InternedIdentifier,
    ) -> Result<Option<FieldTypes>>;

    /// Publishes a computed layout for the nominal type.
    fn set_nominal_layout(
        &self,
        ty: InternedType,
        size: u32,
        align: u32,
        fields: Option<&[FieldLayout]>,
    ) -> Result<()>;
}

/// Attempts to lower a function, and returns an error if lowering failed. The
/// caller must ensure this is not the case by ensuring that all lowering
/// requirements are satisfied (e.g., type sizes known).
pub fn try_lower_function(module_ir: &ModuleIR, func_ir: &FunctionIR) -> Result<Function> {
    let ctx = try_build_context(module_ir, func_ir)?
        .ok_or_else(|| anyhow!("Failed to create lowering context: not all types are concrete"))?;

    let name = module_ir.module.interned_identifier_at(func_ir.name_idx);
    let code = lower_function(func_ir, &ctx)?;
    let code = GasInstrumentor::new(MicroOpGasSchedule).run(code);

    // TODO: `frame_layout` is hardcoded empty (no slots scanned by GC).
    // That is sound only while every fat-pointer slot in this function
    // holds a stack base (filtered by `is_heap_ptr` at GC time). If the
    // lowering ever emits an op that puts a heap pointer in a frame
    // slot, the resulting slot is invisible to GC and a collection
    // during a callee would leave it dangling. Refuse to build such a
    // function until `frame_layout` is derived from the lowering
    // context.
    //
    // Today's lowering doesn't emit any of these ops (the destacker
    // bails first on unsupported Move instructions), so this is a
    // future-trip guard. Categories listed by what semantically puts
    // a heap pointer in a frame slot:
    //   - heap object create / borrow / field read:
    //     `HeapNew`, `HeapBorrow`, `HeapMoveFrom8`, `HeapMoveFrom`
    //   - vector create / borrow / element read / pop:
    //     `VecNew`, `VecBorrow`, `VecLoadElem`, `VecPopBack`
    //   - heap-pointer republish (in-place vec base mutation):
    //     `VecPushBack`
    //   - closures (heap-pointer captures):
    //     `PackClosure`, `CallClosure`
    //
    // Calls (`CallFunc` / `CallDirect` / `CallIndirect`) are not
    // listed here even though a callee returning a heap-typed value
    // would put a heap pointer in the caller's ret region: the
    // hazard depends on the callee's return signature, not on the
    // op itself, and today's lowering bails before emitting a call
    // that returns anything heap-backed.
    for op in &code {
        match op {
            MicroOp::HeapNew { .. }
            | MicroOp::HeapBorrow { .. }
            | MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveFrom { .. }
            | MicroOp::VecNew { .. }
            | MicroOp::VecBorrow { .. }
            | MicroOp::VecLoadElem { .. }
            | MicroOp::VecPopBack { .. }
            | MicroOp::VecPushBack { .. }
            | MicroOp::PackClosure(..)
            | MicroOp::CallClosure(..) => {
                bail!(
                    "function `{}` lowers a heap-pointer-producing op but \
                             frame_layout is not yet derived from the lowering context — \
                             GC would not see the resulting slot",
                    view_name(name),
                );
            },
            _ => {},
        }
    }

    let param_sizes = ctx.home_slots[..func_ir.num_params as usize]
        .iter()
        .map(|s| s.size)
        .collect::<Vec<_>>();
    let param_sizes_sum = param_sizes.iter().map(|s| *s as usize).sum::<usize>();
    let param_and_local_sizes_sum = ctx.frame_data_size as usize;
    let extended_frame_size = ctx
        .call_sites
        .iter()
        .flat_map(|cs| cs.arg_slots.iter().chain(cs.ret_slots.iter()))
        .map(|ts| (ts.slot.offset.0 + ts.slot.size) as usize)
        .max()
        // Leaf function: no callee slots needed beyond metadata.
        .unwrap_or(param_and_local_sizes_sum + FRAME_METADATA_SIZE);

    Ok(Function {
        name,
        code: Code::from_vec(code),
        param_sizes,
        param_sizes_sum,
        param_and_local_sizes_sum,
        extended_frame_size,
        // TODO: hardcoded for now.
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })
}

/// Tries to enforce lowering requirements for all functions in the given
/// module:
///   - calculating type sizes, alignments,
///   - calculating field offsets for structs.
/// Note that the requirements might not be set if there are any generic types
/// or external modules are not available in the context to obtain the field
/// information.
pub fn try_set_lowering_requirements(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
) -> Result<()> {
    let mut visited = UnorderedSet::new();
    for func_ir in module_ir.functions.iter().filter_map(|f| f.as_ref()) {
        try_set_lowering_requirements_for_function_impl(ctx, module_ir, func_ir, &mut visited)?;
    }
    Ok(())
}

/// Tries to enforce lowering requirements for the function in the given
/// module:
///   - calculating type sizes, alignments,
///   - calculating field offsets for structs.
/// Note that the requirements might not be set if there are any generic types
/// or external modules are not available in the context to obtain the field
/// information.
pub fn try_set_lowering_requirements_for_function(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
) -> Result<()> {
    let mut visited = UnorderedSet::new();
    try_set_lowering_requirements_for_function_impl(ctx, module_ir, func_ir, &mut visited)
}

fn try_set_lowering_requirements_for_function_impl(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    visited: &mut UnorderedSet<InternedType>,
) -> Result<()> {
    for &ty in func_ir.home_slot_types.iter() {
        walk_and_size(ctx, ty, visited)?;
    }
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    for &ty in module_ir.module.interned_types_at(own_handle.return_) {
        walk_and_size(ctx, ty, visited)?;
    }
    for instr in func_ir.instrs() {
        let handle_idx = match instr {
            Instr::Call(_, idx, _) => *idx,
            Instr::CallGeneric(_, idx, _) => {
                module_ir.module.function_instantiation_at(*idx).handle
            },
            // TODO: Home slots and callee params/returns are not exhaustive.
            //       Instructions can reference types whose layouts lowering
            //       needs.
            _ => continue,
        };

        let callee_handle = module_ir.module.function_handle_at(handle_idx);
        for &ty in module_ir.module.interned_types_at(callee_handle.parameters) {
            walk_and_size(ctx, ty, visited)?;
        }
        for &ty in module_ir.module.interned_types_at(callee_handle.return_) {
            walk_and_size(ctx, ty, visited)?;
        }
    }

    Ok(())
}

/// Recursive post-order DFS that visits every nominal reachable from the given
/// type. Best-effort: for each visited nominal, computes its layout size,
/// alignment, field offsets when all its fields are sized. Skips nominals for
/// which field information is not available (same treatment as generic type
/// parameters).
///
/// TODO: For fields, we need to check borrow instructions to make sure the
///       offsets are calculated for them.
/// TODO: Make this not recursive.
fn walk_and_size(
    ctx: &mut impl SpecializerContext,
    ty: InternedType,
    visited: &mut UnorderedSet<InternedType>,
) -> Result<()> {
    if !visited.insert(ty) {
        return Ok(());
    }

    match view_type(ty) {
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
        | Type::Address
        | Type::Signer
        | Type::TypeParam { .. }
        | Type::Vector { .. }
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. } => {
            // Sizes for primitives, vectors, function types, and references
            // are known, there is nothing to do. Type parameters have unknown
            // size - just continue.
        },
        Type::Nominal {
            executable_id,
            name,
            ..
        } => {
            // TODO: Walk type-args of the nominal so generic instantiations
            // like `Coin<USDC>` discover USDC as an extra root when its
            // module is outside the allowed scope.
            match ctx.get_fields(executable_id, name)? {
                None => {
                    // The context does not have field information for this
                    // nominal (e.g., the module has not been loaded). Treat
                    // like a generic type parameter: skip.
                },
                Some(FieldTypes::Struct(field_tys)) => {
                    // We have to recurse unconditionally because if size is
                    // set, it does not mean that all modules used have been
                    // resolved. Other thread can set struct's size so this
                    // traversal is needed.
                    for &ft in &field_tys {
                        walk_and_size(ctx, ft, visited)?;
                    }

                    // Best-effort layout computation. If any field is still
                    // not sized, so is the nominal type.
                    let mut offset = 0u32;
                    let mut max_align = 1u32;
                    let mut layout = Vec::with_capacity(field_tys.len());
                    let mut all_sized = true;
                    for &ft in &field_tys {
                        let Some((sz, al)) = view_type(ft).size_and_align() else {
                            all_sized = false;
                            break;
                        };
                        offset = align_up_u32(offset, al);
                        max_align = max_align.max(al);
                        layout.push(FieldLayout::new(offset, ft));
                        offset += sz;
                    }
                    if all_sized {
                        let total = align_up_u32(offset, max_align);
                        ctx.set_nominal_layout(ty, total, max_align, Some(&layout))?;
                    }
                },
                Some(FieldTypes::Enum(_)) => {
                    // Enum size is fixed (heap pointer) regardless of variant
                    // fields. We do not walk variants here because their types
                    // are only needed for pack/unpack/test.
                    ctx.set_nominal_layout(ty, 8, 8, None)?;
                },
            }
        },
    }
    Ok(())
}
