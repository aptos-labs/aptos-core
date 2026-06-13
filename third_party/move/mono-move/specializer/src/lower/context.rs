// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Context for lowering stackless exec IR to micro-ops.
//!
//! Builds frame layout information (slot offsets/sizes) needed by the lowerer.
//! All lookups are O(1) via indexed Vecs — no maps.

use crate::{
    lower::{
        gc_layout::{
            derive_frame_layout, gc_layout_supports, shifted_field_pointer_offsets,
            type_pointer_offsets,
        },
        translate::{lower_function, LoweredFunction},
    },
    stackless_exec_ir::{
        instr_utils::{nominal_type_in_instr, resource_type_in_instr, NominalKind},
        FunctionIR, Instr, ModuleIR,
    },
};
use anyhow::{bail, Result};
use mono_move_core::{
    align_up_u32, checked_align_up_u32,
    interner::{InternedFunctionRef, InternedIdentifier, InternedModuleId},
    native::{NativeIdx, NativeResolver},
    next_captured_value_offset,
    types::{
        strip_ref, view_type, view_type_list, Alignment, FieldLayout, InternedType,
        InternedTypeList, Size, Type, EMPTY_TYPE_LIST,
    },
    value_layout::REF_LAYOUT_ID,
    Code, DescriptorId, FieldTypes, FieldValueLayout, FrameLayoutInfo, FrameOffset, Function,
    Interner, LayoutFlags, LayoutId, PreparedModule, SizedSlot, SortedSafePointEntries,
    ValueLayout, FRAME_METADATA_SIZE, MAX_ALIGN,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionHandleIndex, VariantFieldHandleIndex},
};
use move_core_types::function::ClosureMask;
use shared_dsa::{UnorderedMap, UnorderedSet};

/// Alignment the frame's data region (params + locals + scratch) is rounded to.
const FRAME_ALIGN: u32 = MAX_ALIGN as u32;

/// Reserves `size` bytes for a scratch/box slot at the current end of the frame
/// data region (rounded up to [`FRAME_ALIGN`]), advances `frame_data_size` past
/// it, and returns the slot's offset.
fn reserve_slot(frame_data_size: &mut u32, size: u32) -> FrameOffset {
    let offset = align_up_u32(*frame_data_size, FRAME_ALIGN);
    *frame_data_size = offset + size;
    FrameOffset(offset)
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

/// Byte width of the pointee of reference type `ref_ty`. Errors when `ref_ty`
/// is not a reference, or when its pointee isn't concrete.
pub fn ref_pointee_size(ref_ty: InternedType) -> Result<u32> {
    concrete_type_size(strip_ref(ref_ty)?, "ref pointee type")
}

/// A frame slot paired with the type of its value.
#[derive(Clone, Copy)]
pub struct TypedSlot {
    pub slot: SizedSlot,
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
    /// `Some(idx)` when the callee resolves to a registered native.
    pub native_idx: Option<NativeIdx>,
}

/// Pre-resolved data for one `PackClosure`, consumed in IR order.
pub struct ClosurePackInfo {
    /// Symbolic target identity, resolved lazily at call time.
    pub func_ref: InternedFunctionRef,
    /// GC trace descriptor for the captured-data object; `None` for a
    /// non-capturing closure (no captured-data object is allocated).
    pub captured_data_descriptor_id: Option<DescriptorId>,
    /// Byte width of the captured-data values region (`0` if non-capturing).
    pub values_size: u32,
}

/// Captured-data layout for one capturing `PackClosure`: the GC trace
/// descriptor (the reserved `Trivial` slot when pointer-free) and the byte
/// width of the values region, which the allocation needs.
#[derive(Clone, Copy)]
pub struct ClosureCapturedInfo {
    pub descriptor_id: DescriptorId,
    pub values_size: u32,
}

/// Per-`PackClosure` outcome of the discovery pass, in IR order. Distinguishes
/// the three cases the build pass must act on, so it needn't re-derive them
/// from the mask.
#[derive(Clone, Copy)]
pub enum CapturedDataLayout {
    /// Closure captures nothing; no captured-data object is allocated.
    NonCapturing,
    /// Concrete capturing closure with its resolved captured-data layout.
    Capturing(ClosureCapturedInfo),
    /// A captured type isn't concrete / GC-walkable yet; lowering must `Skip`.
    NotDerivable,
}

/// One variant field's location within the enum data region. Offset is
/// data-region-relative (0-based after the tag).
#[derive(Copy, Clone)]
pub struct VariantFieldLayout {
    pub offset: u32,
    pub size: u32,
    pub ty: InternedType,
}

/// Lowering layout for one concrete enum type: its published descriptor and,
/// per variant, the field layout within the data region.
pub struct EnumLayout {
    pub descriptor_id: DescriptorId,
    pub variants: Vec<Vec<VariantFieldLayout>>,
}

/// Resolved plan for a variant-field access. Move semantics require the
/// access to abort unless the runtime variant is one the handle names;
/// variants are byte-packed, so the field's byte offset can differ per
/// variant and must be selected by the runtime tag.
pub(crate) struct VariantFieldAccess {
    /// Field byte size, uniform across the handle's variants (same field type).
    pub field_size: u32,
    /// Data-region-relative field offset per variant tag; `None` where that
    /// variant does not declare the field (accessing it must abort). Indexed by
    /// variant tag; length is the enum's variant count.
    pub offsets: Vec<Option<u32>>,
    /// `Some(off)` when every variant declares the field at the same offset:
    /// the static fast path (no tag dispatch, membership check, or scratch
    /// reference needed).
    pub uniform_offset: Option<u32>,
}

/// Resolve a `VariantFieldHandleIndex` against the owning enum's derived layout.
/// Move groups variants in a handle by field index, not byte layout, so a shared
/// field can sit at different byte offsets across the handle's variants; the
/// access selects the offset by the runtime tag (and aborts for variants that
/// lack the field).
pub(crate) fn resolve_variant_field_access(
    module: &PreparedModule,
    enum_layouts: &UnorderedMap<InternedType, EnumLayout>,
    vfh: VariantFieldHandleIndex,
) -> Result<VariantFieldAccess> {
    let handle = module.variant_field_handle_at(vfh);
    let enum_ty = module.interned_nominal_def_type_at(handle.struct_index);
    let field_pos = handle.field as usize;
    let layout = enum_layouts
        .get(&enum_ty)
        .ok_or_else(|| anyhow::anyhow!("variant field: no derived layout for enum"))?;
    let mut offsets = vec![None; layout.variants.len()];
    let mut field_size = None;
    for &variant in &handle.variants {
        let field = layout
            .variants
            .get(variant as usize)
            .and_then(|fields| fields.get(field_pos))
            .ok_or_else(|| anyhow::anyhow!("variant field index out of range"))?;
        offsets[variant as usize] = Some(field.offset);
        // The field type is identical across the handle's variants
        // (verifier-enforced), so its size is uniform.
        field_size = Some(field.size);
    }
    let field_size =
        field_size.ok_or_else(|| anyhow::anyhow!("variant field handle has no variants"))?;
    // Fast path: field present in every variant at the same offset -> no runtime
    // tag dispatch or membership check needed. A `None` hole (a variant that
    // lacks the field) correctly rejects the fast path.
    let uniform_offset = offsets.first().copied().flatten().filter(|&first| {
        offsets
            .iter()
            .all(|maybe_offset| *maybe_offset == Some(first))
    });
    Ok(VariantFieldAccess {
        field_size,
        offsets,
        uniform_offset,
    })
}

/// Descriptors discovered for a function's type/closure set, threaded from the
/// discovery pass into lowering.
#[derive(Default)]
pub struct LoweringDescriptors {
    /// Type -> published descriptor id: a `vector<T>` for vector descriptors,
    /// or a resource struct type for `move_to`/`move_from` descriptors.
    ///
    /// TODO: rename to a type-generic name now that it also holds struct
    /// descriptors, and extend to enum descriptors.
    pub vec: UnorderedMap<InternedType, DescriptorId>,
    /// Concrete enum type -> its descriptor + per-variant field layout.
    pub enum_layouts: UnorderedMap<InternedType, EnumLayout>,
    /// Captured-data layout per `PackClosure`, in IR order; consumed
    /// positionally by the build pass.
    pub closure_captured: Vec<CapturedDataLayout>,
}

/// Frame layout for one function.
/// [TODO]: a few raw-`u32` fields remain (sizes/alignments); migrate
/// them to dedicated newtypes for consistency with `FrameOffset`.
pub struct LoweringContext<'a> {
    /// Module the function lives in; gives lowering access to the
    /// constant pool and other module-level metadata.
    pub module: &'a PreparedModule,
    pub home_slots: Vec<SizedSlot>,
    /// End offset of the home-slot region; feeds `callee_base`.
    pub frame_data_size: u32,
    /// In IR order; indexed by `LoweringState::call_site_cursor`.
    pub call_sites: Vec<CallSiteInfo>,
    /// Where `Instr::Ret` writes before the `Return` micro-op. Laid out
    /// from offset 0 so addresses match the caller's `ret_slots`.
    pub return_slots: Vec<SizedSlot>,
    pub num_xfer_positions: u16,
    /// TODO: we should consider unifying the various scratch slots below,
    /// even though they are used for different purposes, only one is ever
    /// live at a time, and they have the same GC invariant.
    /// Frame offset of the cycle-breaking scratch slot used by
    /// `parallel_copy::emit_parallel_copy` for `Instr::Ret`.
    /// Reserved at the end of the home region (sized to fit the widest
    /// return value). `None` when no scratch is needed.
    ///
    /// Invariant: scratch's live range never spans an allocating
    /// micro-op, so does not need GC tracking.
    pub scratch: Option<FrameOffset>,
    /// Frame offset of the 8-byte slot used for lowering of [`Instr::MoveTo`]
    /// and [`Instr::MoveFrom`] ([`None`] if no such instructions are present
    /// in the function). This slot holds the intermediate heap pointer (the
    /// "box pointer"): [`Instr::MoveTo`] allocates the resource object here
    /// before publishing it, and [`Instr::MoveFrom`] receives the moved-out
    /// pointer here before unboxing it.
    ///
    /// **Invariant:** this pointer is never a GC root. Its live range never
    /// spans a collection — the allocating instructions that writes it (for
    /// example [`mono_move_core::MicroOp::HeapNew`] for [`Instr::MoveTo`] and
    /// deep copy for [`Instr::MoveFrom`]) collects *before* the write, so it
    /// does not need GC tracking.
    pub resource_box_slot: Option<FrameOffset>,
    /// 16-byte scratch fat-pointer slot for by-reference variant-field
    /// read/write (`ReadVariantField`/`WriteVariantField`): the `enum_borrow`
    /// (a heap deref) writes the field reference here, then `ReadRef`/
    /// `WriteRef` consumes it. `None` when the function has no such ops.
    ///
    /// Invariant: like `scratch`, its live range (borrow → deref) never spans
    /// an allocating micro-op, so it needs no GC tracking.
    pub variant_field_scratch: Option<FrameOffset>,
    /// 8-byte scratch heap-pointer slot for enum `PackVariant`/`UnpackVariant`
    /// when the slot-allocator aliases the enum-pointer slot with a field slot.
    /// `PackVariant` stages the freshly allocated pointer here and publishes it
    /// to `dst` only after the field stores; `UnpackVariant` copies the pointer
    /// here and loads every field from it. Both keep the slot allocator's
    /// read-before-write contract intact. `None` when the function packs/unpacks
    /// no enums.
    ///
    /// Invariant: like `scratch`, its live range never spans an allocating
    /// micro-op (`EnumNew` is the only allocator and writes the pointer here
    /// after allocating), so it needs no GC tracking.
    pub enum_ptr_scratch: Option<FrameOffset>,
    /// Maps a type to the [`DescriptorId`] published for it: a `vector<T>` for
    /// vector descriptors, or the resource struct type for `move_to`/`move_from`
    /// descriptors.
    ///
    /// Invariant: contains an entry for every vector or resource type used in
    /// this function.
    pub descriptors: UnorderedMap<InternedType, DescriptorId>,
    /// TODO: consider reconciling with the descriptors map above.
    /// Concrete enum type -> its descriptor + per-variant field layout.
    ///
    /// Invariant: contains an entry for every enum type whose concrete
    /// variant ops this function lowers (enforced in `try_build_context`).
    pub enum_layouts: UnorderedMap<InternedType, EnumLayout>,
    /// Per-`PackClosure` resolved data, in IR order.
    pub closure_pack_sites: Vec<ClosurePackInfo>,
    /// Per-`CallClosure` return-slot layout (caller-frame addresses laid out
    /// from `callee_base`).
    pub closure_call_sites: Vec<Vec<TypedSlot>>,
}

impl LoweringContext<'_> {
    /// `DescriptorId` published for `ty`, or `None` if no entry exists. The key
    /// is the type itself: a `vector<T>` for vector descriptors, or the resource
    /// struct type for `move_to`/`move_from` descriptors.
    pub fn descriptor_id(&self, ty: InternedType) -> Option<DescriptorId> {
        self.descriptors.get(&ty).copied()
    }

    /// Layout published for the concrete enum type `enum_ty`, or `None` if
    /// no entry exists.
    pub fn enum_layout(&self, enum_ty: InternedType) -> Option<&EnumLayout> {
        self.enum_layouts.get(&enum_ty)
    }
}

/// Outcome of attempting to build a [`LoweringContext`]: either the
/// context was built, or the function is intentionally skipped with a
/// human-readable reason for display in the snapshot baseline.
///
/// Distinct from the `Err` return: internal-invariant violations stay on
/// the `Err` path because they indicate a real bug. `Skipped` is reserved
/// for "this function is out of scope for the current lowering, on purpose".
pub enum BuildContextOutcome<'a> {
    Built(LoweringContext<'a>),
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

/// Interned `(module_id, func_name)` identity of a function handle.
fn callee_identity(
    module: &PreparedModule,
    handle_idx: FunctionHandleIndex,
) -> (InternedModuleId, InternedIdentifier) {
    let handle = module.function_handle_at(handle_idx);
    (
        module.module_id_at(handle.module),
        module.interned_identifier_at(handle.name),
    )
}

/// Try to build a [`LoweringContext`] for a monomorphic function.
///
/// Returns:
///
/// - `Ok(Built(ctx))` on success.
/// - `Ok(Skipped(reason))` if any type can't be handled — the reason
///   is a short label shown in the snapshot baseline.
/// - `Err(_)` for internal-invariant failures (real bugs).
pub fn try_build_context<'a>(
    module_ir: &'a ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
    interner: &impl Interner,
    descriptors: LoweringDescriptors,
    natives: &dyn NativeResolver,
) -> Result<BuildContextOutcome<'a>> {
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
    let Some(home_slots) = layout_slots(0, home_types) else {
        return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
    };
    // Catches sized nominals (e.g. enums) whose GC layout isn't walkable.
    if home_types.iter().any(|&ty| !gc_layout_supports(ty)) {
        return Ok(BuildContextOutcome::Skipped(
            "nominal type not yet supported by gc_layout",
        ));
    }
    // `frame_data_size` must be `FRAME_ALIGN`-aligned so that
    // `callee_base = frame_data_size + FRAME_METADATA_SIZE` is also
    // aligned (the runtime writes saved pc/fp/func_id as `u64`s
    // starting at `frame_data_size`).
    let mut frame_data_size = align_up_u32(
        home_slots.last().map(|s| s.offset.0 + s.size).unwrap_or(0),
        FRAME_ALIGN,
    );

    // 2. Build `return_slots` from this function's own signature.
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    let own_ret_list =
        interner.type_list_of(module_ir.module.interned_types_at(own_handle.return_));
    let own_ret_list = interner.subst_type_list(own_ret_list, ty_args)?;
    let own_ret_types = view_type_list(own_ret_list);
    let Some(return_slots) = layout_slots(0, own_ret_types) else {
        return Ok(BuildContextOutcome::Skipped("not all types are concrete"));
    };
    if own_ret_types.iter().any(|&ty| !gc_layout_supports(ty)) {
        return Ok(BuildContextOutcome::Skipped(
            "nominal type not yet supported by gc_layout",
        ));
    }

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
        FRAME_ALIGN,
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
    let scratch = (return_slots.len() >= 2 && max_value_width > 0).then(|| {
        reserve_slot(
            &mut frame_data_size,
            align_up_u32(max_value_width, FRAME_ALIGN),
        )
    });

    // 4. Reserve the 8-byte box-pointer slot for `move_to`/`move_from`
    //    lowering, when the body uses either. It holds the intermediate heap
    //    pointer between the alloc/move and the box/unbox copy. Must not be
    //    GC-tracked.
    let needs_resource_box_slot = func_ir
        .instrs()
        .any(|instr| matches!(instr, Instr::MoveTo(..) | Instr::MoveFrom(..)));
    let resource_box_slot = needs_resource_box_slot.then(|| reserve_slot(&mut frame_data_size, 8));

    // 5. Single IR pass over the function's enum ops: (1) verify each one's enum
    //    layout was derivable during discovery, and (2) detect which scratch
    //    slots the lowering needs.
    let mut needs_variant_field_scratch = false;
    let mut needs_enum_ptr_scratch = false;
    for instr in func_ir.instrs() {
        let Some((enum_ty, NominalKind::Enum)) = nominal_type_in_instr(&module_ir.module, instr)
        else {
            continue;
        };
        if !descriptors.enum_layouts.contains_key(&enum_ty) {
            // Skip the whole function when any enum op references an enum whose
            // layout was not derivable.
            return Ok(BuildContextOutcome::Skipped("enum layout not derivable"));
        }
        // A by-reference variant-field read/write may need the 16-byte scratch
        // fat-pointer slot: only the divergent-offset path uses it (the
        // tag-dispatched borrow writes the field reference there for the
        // following `ReadRef`/`WriteRef`, both non-allocating, so it never spans
        // a safe point), but reserve conservatively for any such op rather than
        // re-resolving each access here just to learn whether it is uniform.
        if matches!(
            instr,
            Instr::ReadVariantField(..) | Instr::WriteVariantField(..)
        ) {
            needs_variant_field_scratch = true;
        }
        // Pack/unpack may need an 8-byte scratch to keep the enum pointer out of
        // an aliased field slot (resolved per-instruction in lowering; the exact
        // alias depends on final slot offsets, which aren't known yet here).
        if matches!(instr, Instr::PackVariant(..) | Instr::UnpackVariant(..)) {
            needs_enum_ptr_scratch = true;
        }
    }
    let variant_field_scratch =
        needs_variant_field_scratch.then(|| reserve_slot(&mut frame_data_size, 16));
    let enum_ptr_scratch = needs_enum_ptr_scratch.then(|| reserve_slot(&mut frame_data_size, 8));

    // TODO: we need to revisit the complexity and performance of this function
    // after support for generic monomorphization is in place.
    // 6. Lay out every callee-frame region in a single IR-order pass: regular
    //    calls (`Call`/`CallGeneric`) and closures (`PackClosure`/`CallClosure`)
    //    are disjoint instruction kinds writing disjoint outputs, so one walk
    //    serves both.
    let callee_base = frame_data_size + FRAME_METADATA_SIZE as u32;
    let mut call_sites = Vec::new();
    let mut closure_pack_sites = Vec::new();
    let mut closure_call_sites: Vec<Vec<TypedSlot>> = Vec::new();
    let mut closure_pack_idx = 0usize;
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
            Instr::PackClosure(_, fhi, _, _) => {
                if !module_ir
                    .module
                    .function_handle_at(*fhi)
                    .type_parameters
                    .is_empty()
                {
                    return Ok(BuildContextOutcome::Skipped(
                        "generic closure target not yet lowered",
                    ));
                }
                let (callee_module_id, callee_func_name) = callee_identity(&module_ir.module, *fhi);
                // TODO: support native closure targets. `CallClosure` resolves
                // via `load_function`, which has no IR for natives, so skip them.
                if natives
                    .resolve(callee_module_id, callee_func_name, EMPTY_TYPE_LIST)
                    .is_some()
                {
                    return Ok(BuildContextOutcome::Skipped(
                        "native closure target not yet lowered",
                    ));
                }
                let func_ref =
                    interner.function_ref_of(callee_module_id, callee_func_name, EMPTY_TYPE_LIST);
                // Captured-data layout resolved positionally.
                let layout = descriptors.closure_captured[closure_pack_idx];
                closure_pack_idx += 1;
                let (captured_data_descriptor_id, values_size) = match layout {
                    CapturedDataLayout::NonCapturing => (None, 0),
                    CapturedDataLayout::Capturing(info) => {
                        (Some(info.descriptor_id), info.values_size)
                    },
                    CapturedDataLayout::NotDerivable => {
                        return Ok(BuildContextOutcome::Skipped(
                            "captured-data layout not derivable",
                        ));
                    },
                };
                closure_pack_sites.push(ClosurePackInfo {
                    func_ref,
                    captured_data_descriptor_id,
                    values_size,
                });
                continue;
            },
            Instr::CallClosure(_, sig_types, _) => {
                let first = view_type_list(*sig_types)
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("CallClosure signature is empty"))?;
                let Type::Function { results, .. } = view_type(first) else {
                    anyhow::bail!("CallClosure signature must start with a Function type");
                };
                let ret_list = interner.subst_type_list(*results, ty_args)?;
                let ret_slots = match layout_callee_region(callee_base, view_type_list(ret_list)) {
                    CalleeRegion::Ready(slots) => slots,
                    CalleeRegion::Skip(reason) => return Ok(BuildContextOutcome::Skipped(reason)),
                };
                closure_call_sites.push(ret_slots);
                continue;
            },
            _ => continue,
        };

        let arg_slots = match layout_callee_region(callee_base, view_type_list(param_list)) {
            CalleeRegion::Ready(slots) => slots,
            CalleeRegion::Skip(reason) => return Ok(BuildContextOutcome::Skipped(reason)),
        };
        let ret_slots = match layout_callee_region(callee_base, view_type_list(ret_list)) {
            CalleeRegion::Ready(slots) => slots,
            CalleeRegion::Skip(reason) => return Ok(BuildContextOutcome::Skipped(reason)),
        };
        let (callee_module_id, callee_func_name) = callee_identity(&module_ir.module, handle_idx);
        // TODO: The native registry is trusted unconditionally here.
        //
        // Consider cross-checking against the callee module's `is_native` flag
        // against the callee module's `is_native` flag so a registered impl cannot
        // shadow a Move-body function with the same qualified name.
        let native_idx = natives.resolve(callee_module_id, callee_func_name, call_ty_args);
        call_sites.push(CallSiteInfo {
            callee_module_id,
            callee_func_name,
            arg_slots,
            ret_slots,
            ty_args: call_ty_args,
            native_idx,
        });
    }

    // Each `PackClosure` consumes one layout in order, so the cursor must reach
    // the end of the discovered set.
    debug_assert_eq!(
        closure_pack_idx,
        descriptors.closure_captured.len(),
        "PackClosure count diverged from discovered captured-data layouts"
    );

    Ok(BuildContextOutcome::Built(LoweringContext {
        module: &module_ir.module,
        home_slots,
        frame_data_size,
        call_sites,
        return_slots,
        num_xfer_positions: func_ir.num_xfer_positions,
        scratch,
        resource_box_slot,
        variant_field_scratch,
        enum_ptr_scratch,
        descriptors: descriptors.vec,
        enum_layouts: descriptors.enum_layouts,
        closure_pack_sites,
        closure_call_sites,
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
            slot: SizedSlot {
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
fn layout_slots(base: u32, types: &[InternedType]) -> Option<Vec<SizedSlot>> {
    Some(
        layout_typed_slots_contiguously(base, types)?
            .into_iter()
            .map(|ts| ts.slot)
            .collect(),
    )
}

/// Outcome of laying out a callee-frame region: the typed slots, or the
/// out-of-scope reason lowering must skip with.
enum CalleeRegion {
    Ready(Vec<TypedSlot>),
    Skip(&'static str),
}

/// Lays out a callee-frame region (args or returns) at `base` and checks it is
/// lowerable: every type must be concrete and GC-walkable.
fn layout_callee_region(base: u32, types: &[InternedType]) -> CalleeRegion {
    let Some(slots) = layout_typed_slots_contiguously(base, types) else {
        return CalleeRegion::Skip("not all types are concrete");
    };
    if types.iter().any(|&ty| !gc_layout_supports(ty)) {
        return CalleeRegion::Skip("nominal type not yet supported by gc_layout");
    }
    CalleeRegion::Ready(slots)
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

    /// Publishes an enum descriptor for `enum_ty` (byte width `size` and
    /// per-variant heap-pointer offsets `variant_pointer_offsets`, each
    /// relative to the data region after the 8-byte tag), returning the
    /// assigned [`DescriptorId`]. Idempotent on `enum_ty`.
    fn publish_enum_descriptor(
        &self,
        enum_ty: InternedType,
        size: u32,
        variant_pointer_offsets: Vec<Vec<u32>>,
    ) -> Result<DescriptorId>;

    /// GC trace descriptor for a captured-data object with values-region size
    /// `values_size` and intra-values heap-pointer offsets `pointer_offsets`.
    /// Pointer-free captures return `TRIVIAL_DESCRIPTOR_ID`; pointer-bearing
    /// ones reuse-or-create a descriptor keyed on the pointer-offset shape.
    fn publish_captured_data_descriptor(
        &self,
        values_size: u32,
        pointer_offsets: &[FrameOffset],
    ) -> Result<DescriptorId>;

    /// Returns the layout id for `ty`, or `None` if no layout has been
    /// published yet. Primitives/references/functions resolve to a reserved id.
    fn layout_id_for(&self, ty: InternedType) -> Option<LayoutId>;

    /// Returns the published layout for `id`, or `None` if unknown.
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout>;

    /// Publishes `layout` for `ty` and returns its assigned id. Idempotent.
    fn publish_layout(&self, ty: InternedType, layout: ValueLayout) -> LayoutId;

    /// Publishes a struct descriptor for `struct_ty` (with payload size `size`
    /// and intra-struct heap-pointer offsets `ptr_offsets`), returning the
    /// assigned [`DescriptorId`]. Used for resources spilled to the heap by
    /// `move_to`. Idempotent on `struct_ty`.
    fn publish_struct_descriptor(
        &self,
        struct_ty: InternedType,
        size: u32,
        ptr_offsets: &[FrameOffset],
    ) -> Result<DescriptorId>;
}

/// Interned type list of a closure target's captured parameters (the mask-set
/// subset of its params, in ascending index order), substituted by `ty_args`.
/// Returns `None` for a non-capturing closure (`mask` empty).
///
/// Used by the discovery pass to derive the captured-data layout (values size
/// and heap-pointer offsets).
fn captured_types_of(
    interner: &impl Interner,
    module_ir: &ModuleIR,
    function_handle_idx: FunctionHandleIndex,
    mask: ClosureMask,
    ty_args: InternedTypeList,
) -> Result<Option<InternedTypeList>> {
    if mask.captured_count() == 0 {
        return Ok(None);
    }
    let handle = module_ir.module.function_handle_at(function_handle_idx);
    let params = interner.type_list_of(module_ir.module.interned_types_at(handle.parameters));
    let params = interner.subst_type_list(params, ty_args)?;
    let captured: Vec<InternedType> = view_type_list(params)
        .iter()
        .enumerate()
        .filter_map(|(i, &ty)| mask.is_captured(i).then_some(ty))
        .collect();
    Ok(Some(interner.type_list_of(&captured)))
}

/// Attempts to lower a function.
///
/// `descriptors` must contain an entry for every vector type, every resource
/// type, and every capturing closure target mentioned in `func_ir` (produced
/// by the discovery pass; see [`LoweringDescriptors`]).
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
    descriptors: LoweringDescriptors,
    natives: &dyn NativeResolver,
) -> Result<LoweringOutcome> {
    let ctx = match try_build_context(module_ir, func_ir, ty_args, interner, descriptors, natives)?
    {
        BuildContextOutcome::Built(c) => c,
        BuildContextOutcome::Skipped(reason) => return Ok(LoweringOutcome::Skipped(reason)),
    };

    let name = module_ir.module.interned_identifier_at(func_ir.name_idx);
    let LoweredFunction {
        code,
        entry_gas,
        mut safe_points,
    } = lower_function(func_ir, &ctx)?;
    // TODO: drop this sort if we can guarantee the input is already
    // sorted. `pc_map` is monotone and `emit` pushes in code-offset
    // order, so it's structurally a no-op today — kept as a safety
    // net for now.
    safe_points.sort_by_key(|e| e.code_offset.0);

    // Per-parameter (offset, size, align), in declaration order.
    let param_slots = ctx.home_slots[..func_ir.num_params as usize].to_vec();
    let param_and_local_sizes_sum = ctx.frame_data_size as usize;
    let extended_frame_size = ctx
        .call_sites
        .iter()
        .flat_map(|cs| cs.arg_slots.iter().chain(cs.ret_slots.iter()))
        // Closure calls reserve only a caller-frame return region (the runtime
        // stages args into the callee frame); include it so reads stay in bounds.
        .chain(ctx.closure_call_sites.iter().flatten())
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
        module_id: module_ir.module.id(),
        code: Code::from_vec(code),
        entry_gas,
        param_slots,
        param_region_size: derived.param_region_size as usize,
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
    interner: &impl Interner,
    module_ir: &ModuleIR,
) -> Result<LoweringDescriptors> {
    let mut visited = UnorderedSet::new();
    let mut descriptors = LoweringDescriptors::default();
    for func_ir in module_ir.functions.iter().filter_map(|f| f.as_ref()) {
        try_discover_types_for_lowering_in_function_impl(
            ctx,
            interner,
            module_ir,
            func_ir,
            EMPTY_TYPE_LIST,
            &mut visited,
            &mut descriptors,
        )?;
    }
    Ok(descriptors)
}

/// Per-function variant of [`try_discover_types_for_lowering_in_module`]. Returns
/// the descriptor maps discovered for this function's type/closure set.
pub fn try_discover_types_for_lowering_in_function(
    ctx: &mut impl SpecializerContext,
    interner: &impl Interner,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
) -> Result<LoweringDescriptors> {
    let mut visited = UnorderedSet::new();
    let mut descriptors = LoweringDescriptors::default();
    try_discover_types_for_lowering_in_function_impl(
        ctx,
        interner,
        module_ir,
        func_ir,
        ty_args,
        &mut visited,
        &mut descriptors,
    )?;
    Ok(descriptors)
}

fn try_discover_types_for_lowering_in_function_impl(
    ctx: &mut impl SpecializerContext,
    interner: &impl Interner,
    module_ir: &ModuleIR,
    func_ir: &FunctionIR,
    ty_args: InternedTypeList,
    visited: &mut UnorderedSet<InternedType>,
    descriptors: &mut LoweringDescriptors,
) -> Result<()> {
    for &ty in func_ir.home_slot_types.iter() {
        discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
    }
    let own_handle = module_ir.module.function_handle_at(func_ir.handle_idx);
    for &ty in module_ir.module.interned_types_at(own_handle.return_) {
        discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
    }
    for instr in func_ir.instrs() {
        // Calls: walk param + return signature lists.
        //
        // TODO: `CallClosure` signatures are not walked, and
        // `discover_type_metadata` treats `Type::Function` as terminal, so a
        // type reached only through a closure signature's args/results misses
        // its descriptor and skips lowering. Recurse into `Type::Function` and
        // feed closure-call signatures here.
        let (params, returns) = match instr {
            Instr::Call(_, idx, _) => {
                let sig = module_ir.module.function_signature_at(*idx);
                (Some(sig.params), Some(sig.returns))
            },
            Instr::CallGeneric(_, idx, _) => {
                let sig = module_ir.module.function_instantiation_signature_at(*idx);
                (Some(sig.params), Some(sig.returns))
            },
            _ => (None, None),
        };
        if let Some(params) = params {
            for &ty in view_type_list(params) {
                discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
            }
        }
        if let Some(returns) = returns {
            for &ty in view_type_list(returns) {
                discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
            }
        }

        // Catch nominal types an instruction references directly but
        // that aren't reached by the home/call walks above.
        if let Some((ty, _kind)) = nominal_type_in_instr(&module_ir.module, instr) {
            discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
        }

        // `PackClosure`: resolve the captured-data layout and record it
        // positionally, in IR order, for the build pass.
        if let Instr::PackClosure(_, fhi, mask, _) = instr {
            let layout =
                discover_captured_data_descriptor(ctx, interner, module_ir, *fhi, *mask, ty_args)?;
            descriptors.closure_captured.push(layout);
        }

        // Resources accessed by global-storage ops are laid out as heap
        // objects, so `move_to` can spill the inline value. Discover the
        // resource's layout, then publish a struct descriptor keyed on the
        // concrete resource type.
        //
        // TODO: a still-generic resource (in the module-level walk) is skipped
        // here, mirroring vector descriptors. Handle it once generic
        // global-storage ops are lowered.
        if let Some(resource_ty) = resource_type_in_instr(instr) {
            discover_type_metadata(ctx, resource_ty, ty_args, visited, descriptors)?;
            let resource_ty = ctx.subst_type(resource_ty, ty_args)?;
            if let Some((size, _)) = type_size_and_align(resource_ty)
                && let Ok(ptr_offsets) = type_pointer_offsets(resource_ty)
            {
                let ptr_offsets = ptr_offsets.into_iter().map(FrameOffset).collect::<Vec<_>>();
                let id = ctx.publish_struct_descriptor(resource_ty, size, &ptr_offsets)?;
                descriptors.vec.insert(resource_ty, id);
            }
        }

        // The walks above don't reach a constant's own type. A vector
        // constant needs its (possibly nested) vector descriptors published
        // so `StoreImmVec` can resolve them at runtime, so discover the
        // constant's type here.
        if let Instr::LdConst(_, idx) = instr {
            let ty = module_ir.module.interned_constant_type_at(*idx);
            discover_type_metadata(ctx, ty, ty_args, visited, descriptors)?;
        }
    }

    Ok(())
}

/// Resolves the captured-data layout for one `PackClosure`: which of
/// [`CapturedDataLayout`]'s cases it is and, when capturing, its GC trace
/// descriptor and values-region size.
///
/// Captured values are laid out at their natural alignment (see
/// [`next_captured_value_offset`]) so heap pointers inside captures stay
/// 8-aligned for the GC; `values_size` and the descriptor's `pointer_offsets`
/// reflect that padded layout.
fn discover_captured_data_descriptor(
    ctx: &mut impl SpecializerContext,
    interner: &impl Interner,
    module_ir: &ModuleIR,
    fhi: FunctionHandleIndex,
    mask: ClosureMask,
    ty_args: InternedTypeList,
) -> Result<CapturedDataLayout> {
    let Some(captured_list) = captured_types_of(interner, module_ir, fhi, mask, ty_args)? else {
        return Ok(CapturedDataLayout::NonCapturing);
    };
    let mut cursor = 0usize;
    let mut pointer_offsets = Vec::new();
    for &ty in view_type_list(captured_list) {
        let Some((size, align)) = type_size_and_align(ty) else {
            return Ok(CapturedDataLayout::NotDerivable);
        };
        if !gc_layout_supports(ty) {
            return Ok(CapturedDataLayout::NotDerivable);
        }
        let (offset, next) = next_captured_value_offset(cursor, size as usize, align as usize);
        for rel in type_pointer_offsets(ty)? {
            pointer_offsets.push(FrameOffset(offset as u32 + rel));
        }
        cursor = next;
    }
    let values_size = cursor as u32;
    let descriptor_id = ctx.publish_captured_data_descriptor(values_size, &pointer_offsets)?;
    Ok(CapturedDataLayout::Capturing(ClosureCapturedInfo {
        descriptor_id,
        values_size,
    }))
}

/// Accumulate the inline byte layout of `field_types` (in declaration order):
/// (each field's variant field layout, the region's total size, and its max
/// alignment).
/// Returns `None` if any field type lacks a concrete size or the running
/// offset/total overflows `u32`.
fn layout_inline_fields(
    field_types: &[InternedType],
) -> Option<(Vec<VariantFieldLayout>, u32, u32)> {
    let mut offset = 0u32;
    let mut max_align = 1u32;
    let mut fields = Vec::with_capacity(field_types.len());
    for &ty in field_types {
        let (size, align) = view_type(ty).size_and_align()?;
        offset = checked_align_up_u32(offset, align)?;
        max_align = max_align.max(align);
        fields.push(VariantFieldLayout { offset, size, ty });
        offset = offset.checked_add(size)?;
    }
    let total = checked_align_up_u32(offset, max_align)?;
    Some((fields, total, max_align))
}

/// Recursive post-order DFS that visits every nominal reachable from the given
/// type and, as a side effect, publishes its GC vector descriptors,
/// `NominalLayout`s, and `ValueLayout`s. Returns the type's [`LayoutId`] when one
/// could be built, or `None` when it is deferred.
///
/// Additionally, for each `Type::Vector` reached, recurses into the element
/// type, then publishes a vector descriptor and records the assigned
/// `DescriptorId` in `descriptors`.
///
/// TODO: For fields, we need to check borrow instructions to make sure the
///       offsets are calculated for them.
/// TODO: Make this not recursive.
fn discover_type_metadata(
    ctx: &mut impl SpecializerContext,
    ty: InternedType,
    ty_args: InternedTypeList,
    visited: &mut UnorderedSet<InternedType>,
    descriptors: &mut LoweringDescriptors,
) -> Result<Option<LayoutId>> {
    let ty = ctx.subst_type(ty, ty_args)?;
    if !visited.insert(ty) {
        return Ok(ctx.layout_id_for(ty));
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
            // Primitives have known layouts; function value layout depends
            // on actual data, and type parameters have no layout. Nothing to
            // discover.
            Ok(ctx.layout_id_for(ty))
        },
        Type::ImmutRef { inner } | Type::MutRef { inner } => {
            // Refs are fixed-size and have known layout, but the referent's
            // types still need discovery. `ty` was already substituted above,
            // so any  type params in `inner` are concrete, not type arguments
            // to pass here.
            discover_type_metadata(ctx, *inner, EMPTY_TYPE_LIST, visited, descriptors)?;
            Ok(Some(REF_LAYOUT_ID))
        },
        Type::Vector { elem } => {
            let elem_id =
                discover_type_metadata(ctx, *elem, EMPTY_TYPE_LIST, visited, descriptors)?;
            // Get or publish the GC descriptor for the element.
            let descriptor_id = if let Some(id) = ctx.vec_descriptor_for(*elem) {
                Some(id)
            } else if let Some((elem_size, _)) = type_size_and_align(*elem)
                && let Ok(ptr_offsets) = type_pointer_offsets(*elem)
            {
                let ptr_offsets = ptr_offsets.into_iter().map(FrameOffset).collect::<Vec<_>>();
                Some(ctx.publish_vec_descriptor(*elem, elem_size, &ptr_offsets)?)
            } else {
                None
            };
            if let Some(id) = descriptor_id {
                descriptors.vec.insert(ty, id);
            }
            // Publish the vector layout only when both the element layout and
            // the descriptor are available (the same condition the descriptor
            // uses), so `descriptor_id` is always valid on the layout.
            match (elem_id, descriptor_id) {
                (Some(elem_id), Some(descriptor_id)) => {
                    let layout = ValueLayout::vector(elem_id, descriptor_id);
                    Ok(Some(ctx.publish_layout(ty, layout)))
                },
                _ => Ok(None),
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
                    // like a generic type parameter: defer.
                    Ok(None)
                },
                Some(FieldTypes::Struct(fields)) => {
                    let fields = fields
                        .iter()
                        .map(|f| ctx.subst_type(*f, *nominal_ty_args))
                        .collect::<Result<Vec<_>>>()?;

                    // Recurse on every field unconditionally (a deferred or
                    // unsized field must not stop discovery of later fields'
                    // descriptors), collecting each field's layout id.
                    let mut field_ids = Vec::with_capacity(fields.len());
                    for &ft in &fields {
                        field_ids.push(discover_type_metadata(
                            ctx,
                            ft,
                            EMPTY_TYPE_LIST,
                            visited,
                            descriptors,
                        )?);
                    }

                    // Best-effort layout computation. If any field is still
                    // not sized (or has no published layout), so is the
                    // nominal type: defer. `None` means a field is not sized.
                    let Some((field_layouts, total, max_align)) = layout_inline_fields(&fields)
                    else {
                        return Ok(None);
                    };
                    // TODO: remove legacy size/layout.
                    let mut nominal_fields = Vec::with_capacity(fields.len());
                    let mut layout_fields = Vec::with_capacity(fields.len());
                    let mut fixed_bcs_total: u64 = 0;
                    let mut data_dependent = false;

                    for (field, &fid) in field_layouts.iter().zip(&field_ids) {
                        // A field can be sized yet still lack a published layout:
                        // `vector<T>` always reports an 8-byte pointer, but its
                        // layout is deferred until the element layout and vector
                        // descriptor exist. Defer the nominal too, rather than
                        // treating this as an invariant violation.
                        let Some(id) = fid else {
                            return Ok(None);
                        };
                        let Some(child) = ctx.layout(id) else {
                            bail!("published layout id does not resolve to a layout");
                        };
                        nominal_fields.push(FieldLayout::new(field.offset, field.ty));
                        layout_fields.push(FieldValueLayout {
                            offset: field.offset,
                            id,
                        });
                        match child.fixed_bcs_size {
                            Some(bcs_sz) => {
                                fixed_bcs_total = fixed_bcs_total.saturating_add(bcs_sz as u64)
                            },
                            None => data_dependent = true,
                        }
                    }
                    ctx.set_nominal_layout(ty, total, max_align, Some(&nominal_fields))?;

                    let fixed_bcs_size = if data_dependent || fixed_bcs_total > u32::MAX as u64 {
                        None
                    } else {
                        Some(fixed_bcs_total as u32)
                    };
                    // The struct has no pointers and no padding exactly when
                    // its packed BCS size equals its in-memory size: a pointer
                    // field makes the BCS size data-dependent (`None`), and any
                    // alignment padding makes it strictly smaller than `total`.
                    let mut flags = LayoutFlags::empty();
                    if fixed_bcs_size == Some(total) {
                        flags |= LayoutFlags::NO_POINTERS_NO_PADDING;
                    }
                    let value_layout = ValueLayout::struct_layout(
                        total,
                        max_align,
                        fixed_bcs_size,
                        flags,
                        layout_fields.into_boxed_slice(),
                    );
                    Ok(Some(ctx.publish_layout(ty, value_layout)))
                },
                Some(FieldTypes::Enum(variants)) => {
                    // An enum is an 8-byte heap pointer at the type level.
                    ctx.set_nominal_layout(ty, 8, 8, None)?;

                    let mut variant_layouts: Vec<Vec<VariantFieldLayout>> =
                        Vec::with_capacity(variants.len());
                    let mut variant_ptr_offsets: Vec<Vec<u32>> = Vec::with_capacity(variants.len());
                    let mut max_data_size = 0u32;
                    let mut all_sized = true;

                    'variants: for variant_fields in &variants {
                        // Substitute each field type by the enum's own type
                        // args (mirrors the Struct arm), then recurse so nested
                        // vec/struct/enum descriptors get published.
                        let fields = variant_fields
                            .iter()
                            .map(|field_ty| ctx.subst_type(*field_ty, *nominal_ty_args))
                            .collect::<Result<Vec<_>>>()?;
                        for &field_ty in &fields {
                            discover_type_metadata(
                                ctx,
                                field_ty,
                                EMPTY_TYPE_LIST,
                                visited,
                                descriptors,
                            )?;
                        }

                        // Per-variant layout: offsets are data-region-relative
                        // (0-based after the tag).
                        let Some((variant_layout, variant_size, _)) = layout_inline_fields(&fields)
                        else {
                            all_sized = false;
                            break 'variants;
                        };
                        // GC pointer offsets: each field's inner pointer offsets
                        // shifted by the field's offset within the data region.
                        let Ok(ptr_offsets) = shifted_field_pointer_offsets(
                            variant_layout.iter().map(|field| (field.offset, field.ty)),
                        ) else {
                            all_sized = false;
                            break 'variants;
                        };
                        max_data_size = max_data_size.max(variant_size);
                        variant_layouts.push(variant_layout);
                        variant_ptr_offsets.push(ptr_offsets);
                    }

                    // Best-effort: only publish the enum descriptor + field
                    // layout when every variant field is sized + GC-walkable and
                    // the total size fits `u32`. Otherwise the 8-byte nominal
                    // layout above remains and `try_build_context` skips the
                    // function.
                    if all_sized
                        && let Some(size) = 8u32
                            .checked_add(max_data_size)
                            .and_then(|data_end| checked_align_up_u32(data_end, 8))
                    {
                        let descriptor_id =
                            ctx.publish_enum_descriptor(ty, size, variant_ptr_offsets)?;
                        descriptors.enum_layouts.insert(ty, EnumLayout {
                            descriptor_id,
                            variants: variant_layouts,
                        });
                    }

                    // TODO: consider reconciling `EnumLayout`, `ValueLayout::open_enum`, and the enum descriptor.
                    // The enum's `ValueLayout` is the open-enum shape regardless
                    // of whether the variant field layout was derivable; the
                    // value-traversal walk resolves variants lazily.
                    let value_layout = ValueLayout::open_enum(ty, *nominal_ty_args);
                    Ok(Some(ctx.publish_layout(ty, value_layout)))
                },
            }
        },
    }
}
