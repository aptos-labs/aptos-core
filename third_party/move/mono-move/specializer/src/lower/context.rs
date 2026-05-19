// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Context for lowering stackless exec IR to micro-ops.
//!
//! Builds frame layout information (slot offsets/sizes) needed by the lowerer.
//! All lookups are O(1) via indexed Vecs — no maps.

use crate::{
    lower::{
        gc_layout::{derive_frame_layout, type_pointer_offsets},
        lower_function,
    },
    stackless_exec_ir::{FunctionIR, Instr, ModuleIR},
};
use anyhow::{bail, Result};
use mono_move_core::{
    align_up_u32,
    interner::{InternedIdentifier, InternedModuleId},
    types::{
        view_type, view_type_list, Alignment, FieldLayout, InternedType, InternedTypeList, Size,
        Type, EMPTY_TYPE_LIST,
    },
    Code, CodeOffset, DescriptorId, FieldTypes, FrameLayoutInfo, FrameOffset, Function, Interner,
    MicroOpGasSchedule, SafePointEntry, SortedSafePointEntries, FRAME_METADATA_SIZE,
};
use mono_move_gas::GasInstrumentor;
use move_binary_format::access::ModuleAccess;
use shared_dsa::{UnorderedMap, UnorderedSet};

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

/// Size in bytes of `ty`. Errors when the type isn't concrete; `label`
/// identifies the value in the error message.
pub fn concrete_type_size(ty: InternedType, label: &str) -> Result<u32> {
    let (size, _) =
        type_size_and_align(ty).ok_or_else(|| anyhow::anyhow!("{} has no concrete size", label))?;
    Ok(size)
}

/// Byte-level location of a typed value in the current function's frame.
#[derive(Clone, Copy)]
pub struct SlotInfo {
    pub offset: FrameOffset,
    /// Width of the type currently bound to this slot.
    pub size: u32,
    pub align: u32,
}

/// A frame slot paired with the type of its value.
#[derive(Clone, Copy)]
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
    /// Empty for non-generic calls.
    pub ty_args: InternedTypeList,
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
    /// `parallel_copy::emit_parallel_copy` for `Instr::Ret`.
    /// Reserved at the end of the home region (sized to fit the widest
    /// return value). `None` when no scratch is needed.
    ///
    /// Invariant: scratch's live range never spans an allocating
    /// micro-op, so does not need GC tracking.
    pub scratch: Option<FrameOffset>,
    /// `vector<T>` -> published `DescriptorId`.
    ///
    /// Invariant: contains an entry for every vector type mentioned in
    /// this function.
    pub vec_descriptors: UnorderedMap<InternedType, DescriptorId>,
}

impl LoweringContext {
    /// `DescriptorId` published for `vec_ty` (the vector type itself,
    /// not its element type), or `None` if no entry exists.
    pub fn vec_descriptor_id(&self, vec_ty: InternedType) -> Option<DescriptorId> {
        self.vec_descriptors.get(&vec_ty).copied()
    }
}

/// Outcome of attempting to build a [`LoweringContext`]: either the
/// context was built, or the function is intentionally skipped with a
/// human-readable reason for display in the snapshot baseline.
///
/// Distinct from the `Err` return: alignment failures and other
/// internal-invariant violations stay on the `Err` path because they
/// indicate a real bug. `Skipped` is reserved for "this function is
/// out of scope for the current lowering, on purpose."
pub enum BuildContextOutcome {
    Built(LoweringContext),
    Skipped(&'static str),
}

/// Outcome of attempting to lower a function: either a fully lowered
/// [`Function`] was produced, or lowering was skipped for an
/// out-of-scope feature (currently nominal types or partial
/// concretization). Mirrors [`BuildContextOutcome`] — `Skipped` is a
/// non-fatal "not yet supported," while internal-invariant violations
/// still travel on the `Err` path.
pub enum LoweringOutcome {
    Built(Function),
    Skipped(&'static str),
}

/// Returns `true` if any of `types` is a [`Type::Nominal`].
fn contains_nominal(types: &[InternedType]) -> bool {
    types
        .iter()
        .any(|&ty| matches!(view_type(ty), Type::Nominal { .. }))
}

/// Try to build a [`LoweringContext`] for a monomorphic function.
///
/// Returns:
///
/// - `Ok(Built(ctx))` on success.
/// - `Ok(Skipped(reason))` if any type can't be handled — the reason
///   is a short label shown in the snapshot baseline (e.g.
///   "not all types are concrete", "nominal type not yet supported").
/// - `Err(_)` for unsupported alignments and other internal-invariant
///   failures.
pub fn try_build_context(
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
    interner: &impl Interner,
    vec_descriptors: UnorderedMap<InternedType, DescriptorId>,
) -> Result<BuildContextOutcome> {
    // 1. Compute home slot layout with natural alignment padding.
    //
    // Slots are laid out linearly in declaration order, padding each to
    // its alignment. This can leave gaps between a small slot followed
    // by a higher-aligned one.
    //
    // TODO: consider a smarter packing (e.g. sort by descending
    // alignment, or bin-pack smaller slots into padding holes) to
    // shrink frame size.
    // TODO: Expose a substitution API that takes and returns non-canonicalized
    // slices of `InternedType`. Today `subst_type_list` operates on
    // `InternedTypeList`, so we have to round-trip `func_ir.home_slot_types`
    // through `type_list_of` to intern it just so substitution accepts it.
    // The intermediate list is only used to feed substitution and then
    // immediately viewed back as a slice via `view_type_list`, so the
    // canonicalization step is pure overhead for this caller.
    let home_list = interner.type_list_of(&func_ir.home_slot_types);
    let home_list = interner.subst_type_list(home_list, ty_args)?;
    let home_types = view_type_list(home_list);
    // Concrete Nominals (post-substitution) are out of scope for the
    // current GC-layout pass.
    if contains_nominal(home_types) {
        return Ok(BuildContextOutcome::Skipped(
            "nominal type not yet supported",
        ));
    }
    let Some(home_slots) = layout_slots(0, home_types) else {
        return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
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
    let own_ret_list =
        interner.type_list_of(module_ir.module.interned_types_at(own_handle.return_));
    let own_ret_list = interner.subst_type_list(own_ret_list, ty_args)?;
    let own_ret_types = view_type_list(own_ret_list);
    if contains_nominal(own_ret_types) {
        return Ok(BuildContextOutcome::Skipped(
            "nominal type not yet supported",
        ));
    }
    let Some(return_slots) = layout_slots(0, own_ret_types) else {
        return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
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
    let scratch = if return_slots.len() >= 2 && max_value_width > 0 {
        let offset = align_up_u32(frame_data_size, MIN_SLOT_ALIGN);
        let size = align_up_u32(max_value_width, MIN_SLOT_ALIGN);
        frame_data_size = offset + size;
        Some(FrameOffset(offset))
    } else {
        None
    };

    // 4. Walk `Call`/`CallGeneric` instructions and lay out each callee's
    //    arg/ret region from `callee_base`.
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;
    let mut call_sites = Vec::new();
    for instr in func_ir.instrs() {
        let (handle_idx, param_list, ret_list, call_ty_args) = match instr {
            Instr::Call(_, idx, _) => {
                let sig = module_ir.module.function_signature_at(*idx);
                (*idx, sig.params, sig.returns, EMPTY_TYPE_LIST)
            },
            Instr::CallGeneric(_, idx, _) => {
                let inst = module_ir.module.function_instantiation_at(*idx);
                let sig = module_ir.module.function_instantiation_signature_at(*idx);
                let params = interner.subst_type_list(sig.params, ty_args)?;
                let returns = interner.subst_type_list(sig.returns, ty_args)?;
                let call_ty_args = interner.subst_type_list(sig.ty_args, ty_args)?;
                (inst.handle, params, returns, call_ty_args)
            },
            _ => continue,
        };

        let param_types = view_type_list(param_list);
        let ret_types = view_type_list(ret_list);
        if contains_nominal(param_types) || contains_nominal(ret_types) {
            return Ok(BuildContextOutcome::Skipped(
                "nominal type not yet supported",
            ));
        }
        let Some(arg_slots) = layout_typed_slots_contiguously(callee_base, param_types) else {
            return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
        };
        let Some(ret_slots) = layout_typed_slots_contiguously(callee_base, ret_types) else {
            return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
        };
        check_supported_alignment(&arg_slots, |s| s.slot.align, "callee arg")?;
        check_supported_alignment(&ret_slots, |s| s.slot.align, "callee ret")?;

        let callee_handle = module_ir.module.function_handle_at(handle_idx);
        let callee_module_id = module_ir.module.module_id_at(callee_handle.module);
        let callee_func_name = module_ir.module.interned_identifier_at(callee_handle.name);
        call_sites.push(CallSiteInfo {
            callee_module_id,
            callee_func_name,
            arg_slots,
            ret_slots,
            ty_args: call_ty_args,
        });
    }

    Ok(BuildContextOutcome::Built(LoweringContext {
        home_slots,
        frame_data_size,
        call_sites,
        return_slots,
        num_xfer_positions: func_ir.num_xfer_positions,
        scratch,
        vec_descriptors,
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

    /// Substitutes type parameters in the given type using type arguments as
    /// the substitution (indexed by indices in type param nodes). Returns an
    /// error if substitution fails.
    ///
    /// # Invariants
    ///
    /// 1. Every type as index `i` in type argument list corresponds to type
    ///    parameter `i` in the generic type.
    /// 2. Size of the type argument list can be greater than the largest type
    ///    parameter `i` in the generic type. It should never be smaller. If
    ///    so, then substitution fails.
    fn subst_type(&self, ty: InternedType, ty_args: InternedTypeList) -> Result<InternedType>;

    /// Publishes a vector descriptor for `elem_ty` (with byte width
    /// `elem_size` and intra-element heap-pointer offsets
    /// `elem_ptr_offsets`), returning the assigned [`DescriptorId`].
    /// Idempotent on `elem_ty`: subsequent calls with the same element
    /// type return the same id without appending.
    fn publish_vec_descriptor(
        &self,
        elem_ty: InternedType,
        elem_size: u32,
        elem_ptr_offsets: &[FrameOffset],
    ) -> Result<DescriptorId>;

    /// Returns the already-published vector-descriptor id for `elem_ty`,
    /// or `None` if no descriptor has been published yet.
    fn vec_descriptor_for(&self, elem_ty: InternedType) -> Option<DescriptorId>;
}

/// Attempts to lower a function.
///
/// `vec_descriptors` must contain an entry for every vector type
/// mentioned in `func_ir` (see [`LoweringContext::vec_descriptors`]).
///
/// Returns:
///
/// - `Ok(LoweringOutcome::Built(f))` on success.
/// - `Ok(LoweringOutcome::Skipped(reason))` when `try_build_context`
///   reports an out-of-scope shape (currently nominal types or
///   partial concretization). The caller decides how to handle this —
///   today, the loader surfaces it as a load-time error while keeping
///   any side-effects (side-loaded dependencies, gas charges) that
///   earlier phases already committed to the read-set.
/// - `Err(_)` for internal-invariant violations and other real bugs.
pub fn try_lower_function(
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
    interner: &impl Interner,
    vec_descriptors: UnorderedMap<InternedType, DescriptorId>,
) -> Result<LoweringOutcome> {
    let ctx = match try_build_context(module_ir, func_ir, ty_args, interner, vec_descriptors)? {
        BuildContextOutcome::Built(c) => c,
        BuildContextOutcome::Skipped(reason) => return Ok(LoweringOutcome::Skipped(reason)),
    };

    let name = module_ir.module.interned_identifier_at(func_ir.name_idx);
    let (code, raw_safe_points) = lower_function(func_ir, &ctx)?;
    // TODO: this remapping of safe-point PCs to the allocating op's own new position
    // will go away once we move gas instrumentation to the stackless exec IR level.
    let (code, pc_map) = GasInstrumentor::new(MicroOpGasSchedule).run_with_pc_map(code);
    let mut safe_points: Vec<SafePointEntry> = raw_safe_points
        .into_iter()
        .map(|entry| SafePointEntry {
            code_offset: CodeOffset(pc_map[entry.code_offset.0 as usize]),
            layout: entry.layout,
        })
        .collect();
    // TODO: drop this sort if we can guarantee the input is already
    // sorted. `pc_map` is monotone and `emit` pushes in code-offset
    // order, so it's structurally a no-op today — kept as a safety
    // net for now.
    safe_points.sort_by_key(|e| e.code_offset.0);

    let param_sizes = ctx.home_slots[..func_ir.num_params as usize]
        .iter()
        .map(|s| s.size)
        .collect::<Vec<_>>();
    let param_and_local_sizes_sum = ctx.frame_data_size as usize;
    let extended_frame_size = ctx
        .call_sites
        .iter()
        .flat_map(|cs| cs.arg_slots.iter().chain(cs.ret_slots.iter()))
        .map(|ts| (ts.slot.offset.0 + ts.slot.size) as usize)
        .max()
        // Leaf function: no callee slots needed beyond metadata.
        .unwrap_or(param_and_local_sizes_sum + FRAME_METADATA_SIZE);

    // Derive `frame_layout` and `zero_frame` from home-slot types.
    // Substitute `ty_args` so generic instantiations see concrete
    // types — `gc_layout` rejects raw `TypeParam`s.
    let home_list = interner.type_list_of(&func_ir.home_slot_types);
    let home_list = interner.subst_type_list(home_list, ty_args)?;
    let derived = derive_frame_layout(&ctx, func_ir, view_type_list(home_list))?;

    Ok(LoweringOutcome::Built(Function {
        name,
        code: Code::from_vec(code),
        param_sizes,
        param_sizes_sum: derived.param_sizes_sum as usize,
        param_and_local_sizes_sum,
        extended_frame_size,
        zero_frame: derived.zero_frame,
        frame_layout: FrameLayoutInfo::new(derived.heap_ptr_offsets),
        safe_point_layouts: SortedSafePointEntries::new(safe_points),
    }))
}

/// Walks every type reachable from each function in `module_ir` and publishes
/// the layout metadata lowering needs:
///   - type sizes, alignments,
///   - field offsets for structs,
///   - vector descriptors (one per unique `vector<T>` element type).
///
/// Returns the `InternedType -> DescriptorId` map for any vector types
/// encountered.
///
/// Note that generic types or types from out-of-scope modules may remain
/// unresolved, in which case the corresponding layouts simply aren't published.
pub fn try_discover_types_for_lowering_in_module(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
) -> Result<UnorderedMap<InternedType, DescriptorId>> {
    let mut visited = UnorderedSet::new();
    let mut vec_descriptors = UnorderedMap::new();
    for func_ir in module_ir.functions.iter().filter_map(|f| f.as_ref()) {
        try_discover_types_for_lowering_in_function_impl(
            ctx,
            module_ir,
            func_ir,
            EMPTY_TYPE_LIST,
            &mut visited,
            &mut vec_descriptors,
        )?;
    }
    Ok(vec_descriptors)
}

/// Per-function variant of [`try_discover_types_for_lowering_in_module`]. Returns
/// the vector-descriptor map discovered for this function's type set.
pub fn try_discover_types_for_lowering_in_function(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
) -> Result<UnorderedMap<InternedType, DescriptorId>> {
    let mut visited = UnorderedSet::new();
    let mut vec_descriptors = UnorderedMap::new();
    try_discover_types_for_lowering_in_function_impl(
        ctx,
        module_ir,
        func_ir,
        ty_args,
        &mut visited,
        &mut vec_descriptors,
    )?;
    Ok(vec_descriptors)
}

fn try_discover_types_for_lowering_in_function_impl(
    ctx: &mut impl SpecializerContext,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
    visited: &mut UnorderedSet<InternedType>,
    vec_descriptors: &mut UnorderedMap<InternedType, DescriptorId>,
) -> Result<()> {
    for &ty in func_ir.home_slot_types.iter() {
        discover_type_metadata(ctx, ty, ty_args, visited, vec_descriptors)?;
    }
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    for &ty in module_ir.module.interned_types_at(own_handle.return_) {
        discover_type_metadata(ctx, ty, ty_args, visited, vec_descriptors)?;
    }
    for instr in func_ir.instrs() {
        let (params, returns) = match instr {
            Instr::Call(_, idx, _) => {
                let sig = module_ir.module.function_signature_at(*idx);
                (sig.params, sig.returns)
            },
            Instr::CallGeneric(_, idx, _) => {
                let sig = module_ir.module.function_instantiation_signature_at(*idx);
                (sig.params, sig.returns)
            },
            // TODO: Home slots and callee params/returns are not exhaustive.
            //       Instructions can reference types whose layouts lowering
            //       needs.
            _ => continue,
        };

        for &ty in view_type_list(params) {
            discover_type_metadata(ctx, ty, ty_args, visited, vec_descriptors)?;
        }
        for &ty in view_type_list(returns) {
            discover_type_metadata(ctx, ty, ty_args, visited, vec_descriptors)?;
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
/// Additionally, for each `Type::Vector` reached, recurses into the element
/// type, then publishes a vector descriptor and records the assigned
/// `DescriptorId` in `vec_descriptors`.
///
/// TODO: For fields, we need to check borrow instructions to make sure the
///       offsets are calculated for them.
/// TODO: Make this not recursive.
fn discover_type_metadata(
    ctx: &mut impl SpecializerContext,
    ty: InternedType,
    ty_args: InternedTypeList,
    visited: &mut UnorderedSet<InternedType>,
    vec_descriptors: &mut UnorderedMap<InternedType, DescriptorId>,
) -> Result<()> {
    let ty = ctx.subst_type(ty, ty_args)?;
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
        | Type::Function { .. } => {
            // Sizes for primitives and function types are known; type
            // parameters have unknown size — nothing to discover.
        },
        Type::ImmutRef { inner } | Type::MutRef { inner } => {
            // Refs are fixed-size, but the referent's types still need
            // discovery. `ty` was already substituted above, so any
            // type params in `inner` are concrete — pass `EMPTY_TYPE_LIST`.
            discover_type_metadata(ctx, *inner, EMPTY_TYPE_LIST, visited, vec_descriptors)?;
        },
        Type::Vector { elem } => {
            discover_type_metadata(ctx, *elem, EMPTY_TYPE_LIST, visited, vec_descriptors)?;
            if let Some(id) = ctx.vec_descriptor_for(*elem) {
                vec_descriptors.insert(ty, id);
            } else if let Some((elem_size, _)) = type_size_and_align(*elem)
                && let Ok(ptr_offsets) = type_pointer_offsets(*elem)
            {
                let ptr_offsets = ptr_offsets.into_iter().map(FrameOffset).collect::<Vec<_>>();
                let id = ctx.publish_vec_descriptor(*elem, elem_size, &ptr_offsets)?;
                vec_descriptors.insert(ty, id);
            }
        },
        Type::Nominal {
            module_id,
            name,
            ty_args: nominal_ty_args,
            ..
        } => {
            match ctx.get_fields(module_id, name)? {
                None => {
                    // The context does not have field information for this
                    // nominal (e.g., the module has not been loaded). Treat
                    // like a generic type parameter: skip.
                },
                Some(FieldTypes::Struct(fields)) => {
                    let fields = fields
                        .iter()
                        .map(|f| ctx.subst_type(*f, *nominal_ty_args))
                        .collect::<Result<Vec<_>>>()?;

                    // We have to recurse unconditionally because if size is
                    // set, it does not mean that all modules used have been
                    // resolved. Other thread can set struct's size so this
                    // traversal is needed. Recursing on the substituted
                    // fields also surfaces transitively-referenced nominals
                    // (e.g., `Coin<USDC>` field - `USDC` as a root).
                    for &ft in &fields {
                        // At this point we have already substituted fields, so
                        // all types inside must be instantiated. Even if we
                        // encounter some nominal, its type arguments would be
                        // substituted as well.
                        discover_type_metadata(ctx, ft, EMPTY_TYPE_LIST, visited, vec_descriptors)?;
                    }

                    // Best-effort layout computation. If any field is still
                    // not sized, so is the nominal type.
                    let mut offset = 0u32;
                    let mut max_align = 1u32;
                    let mut layout = Vec::with_capacity(fields.len());
                    let mut all_sized = true;
                    for &ft in &fields {
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
