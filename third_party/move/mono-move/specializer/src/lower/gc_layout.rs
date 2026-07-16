// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-driven derivation of GC frame layout from monomorphic slot types.

use super::context::LoweringContext;
use crate::{
    error::{LoweringError, LoweringResult},
    stackless_exec_ir::FunctionIR,
};
use mono_move_core::{
    types::{view_type, InternedType, Type},
    FrameOffset, LayoutId, LayoutKind, LayoutProvider,
};

/// GC-relevant frame metadata derived for a single function.
pub struct DerivedFrameLayout {
    /// Sorted, deduplicated frame byte-offsets of heap-pointer slots.
    pub heap_ptr_offsets: Vec<FrameOffset>,
    /// `true` iff at least one offset belongs to a non-parameter home slot.
    pub zero_frame: bool,
    /// Byte size of the parameter region.
    pub param_region_size: u32,
}

/// Derive the GC frame layout for `func_ir`. `home_slot_types` is
/// taken separately so the caller can pass a substituted view.
pub fn derive_frame_layout(
    ctx: &LoweringContext<'_>,
    func_ir: &FunctionIR,
    home_slot_types: &[InternedType],
) -> LoweringResult<DerivedFrameLayout> {
    let mut heap_ptr_offsets = vec![];
    // TODO(cleanup): consider whether `LoweringContext::home_slots` should carry
    // each slot's type directly, so we wouldn't need to zip with a
    // separate `home_slot_types` slice here.
    for (slot, &ty) in ctx.home_slots.iter().zip(home_slot_types.iter()) {
        for rel in type_pointer_offsets(ctx.layouts, ty)? {
            heap_ptr_offsets.push(FrameOffset(slot.offset.0 + rel));
        }
    }
    // TODO(cleanup): revisit whether sort and dedup is necessary here or if invariants
    // established beforehand guarantee them already.
    heap_ptr_offsets.sort_by_key(|o| o.0);
    heap_ptr_offsets.dedup();

    // Byte size of the parameter region: past the last parameter.
    let param_region_size: u32 = ctx
        .home_slots
        .iter()
        .take(func_ir.num_params as usize)
        .next_back()
        .map(|s| s.offset.0 + s.size)
        .unwrap_or(0);
    // Is any pointer slot beyond the parameter region?
    let zero_frame = heap_ptr_offsets
        .last()
        .is_some_and(|off| off.0 >= param_region_size);

    Ok(DerivedFrameLayout {
        heap_ptr_offsets,
        zero_frame,
        param_region_size,
    })
}

/// Returns `true` iff `type_pointer_offsets` would accept `ty` without
/// erroring. Keep in sync with `type_pointer_offsets`: every case that
/// errors there must return `false` here.
pub fn gc_layout_supports(layouts: &dyn LayoutProvider, ty: InternedType) -> bool {
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
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Vector { .. }
        | Type::Function { .. } => true,
        // A nominal is walkable once its value layout is published.
        Type::Nominal { .. } => layouts.layout_by_ty(ty).is_some(),
        Type::TypeParam { .. } => false,
    }
}

/// Byte offsets of the pointer slots inside a value of type `ty`,
/// relative to the value's start.
///
/// Returns an error if the provided type is not yet supported or
/// contains non-instantiated type parameters. Callers that want to
/// decide *whether* to lower a function should use `gc_layout_supports`
/// for a graceful `Skipped` outcome rather than reaching this `Err`.
pub fn type_pointer_offsets(
    layouts: &dyn LayoutProvider,
    ty: InternedType,
) -> LoweringResult<Vec<u32>> {
    let offsets = match view_type(ty) {
        // Scalars: no pointer offsets.
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
        | Type::Signer => vec![],

        // 16-byte fat pointer: base half at offset 0, scalar offset
        // half at +8. Only the base is a pointer slot.
        Type::ImmutRef { .. } | Type::MutRef { .. } => vec![0],

        // 8-byte heap pointers.
        Type::Vector { .. } | Type::Function { .. } => vec![0],

        // Structs/enums: walk the published value layout. A struct recurses
        // through its fields' child layouts; an enum is an 8-byte heap pointer.
        //
        // TODO(metering): rewrite without recursion or add a depth/visited bound.
        Type::Nominal { .. } => {
            let id = layouts
                .layout_id(ty)
                .ok_or(LoweringError::LayoutNotPopulated)?;
            layout_pointer_offsets(layouts, id)?
        },

        Type::TypeParam { .. } => {
            return Err(LoweringError::TypeParamReachedGcLayout);
        },
    };
    Ok(offsets)
}

/// Heap-pointer byte offsets within a value of the given published layout,
/// relative to the value's start. Pointer positions are a property of the
/// layout: a reference, vector, function, or (frozen) enum slot holds a pointer
/// at relative offset 0; a struct recurses through its fields' child layouts;
/// scalars hold none.
///
/// TODO(metering): rewrite without recursion or add a depth/visited bound.
fn layout_pointer_offsets(layouts: &dyn LayoutProvider, id: LayoutId) -> LoweringResult<Vec<u32>> {
    let layout = layouts
        .layout(id)
        .ok_or(LoweringError::LayoutIdUnresolved)?;
    let offsets = match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => vec![],
        LayoutKind::Ref
        | LayoutKind::Vector { .. }
        | LayoutKind::FrozenEnum { .. }
        | LayoutKind::Function => vec![0],
        LayoutKind::Struct { fields } => {
            let mut out = vec![];
            for field in fields.iter() {
                for rel in layout_pointer_offsets(layouts, field.id)? {
                    let abs = field.offset.checked_add(rel).ok_or(
                        LoweringError::GcFieldOffsetOverflow {
                            field_offset: field.offset,
                            inner_offset: rel,
                        },
                    )?;
                    out.push(abs);
                }
            }
            out
        },
    };
    Ok(offsets)
}

/// Heap-pointer byte offsets for a sequence of `(field_offset, field_type)`
/// pairs: each field's own pointer offsets shifted by the field's offset within
/// the enclosing region.
/// Errors on a non-GC-walkable field type or a `u32` offset overflow.
pub fn shifted_field_pointer_offsets(
    layouts: &dyn LayoutProvider,
    fields: impl IntoIterator<Item = (u32, InternedType)>,
) -> LoweringResult<Vec<u32>> {
    let mut out = vec![];
    for (field_offset, field_ty) in fields {
        for rel in type_pointer_offsets(layouts, field_ty)? {
            let abs =
                field_offset
                    .checked_add(rel)
                    .ok_or(LoweringError::GcFieldOffsetOverflow {
                        field_offset,
                        inner_offset: rel,
                    })?;
            out.push(abs);
        }
    }
    Ok(out)
}
