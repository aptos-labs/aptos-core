// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-driven derivation of GC frame layout from monomorphic slot types.

use super::context::LoweringContext;
use crate::stackless_exec_ir::FunctionIR;
use anyhow::{bail, Result};
use mono_move_core::{
    types::{view_type, InternedType, Type},
    FrameOffset,
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
) -> Result<DerivedFrameLayout> {
    let mut heap_ptr_offsets = vec![];
    // TODO: consider whether `LoweringContext::home_slots` should carry
    // each slot's type directly, so we wouldn't need to zip with a
    // separate `home_slot_types` slice here.
    for (slot, &ty) in ctx.home_slots.iter().zip(home_slot_types.iter()) {
        for rel in type_pointer_offsets(ty)? {
            heap_ptr_offsets.push(FrameOffset(slot.offset.0 + rel));
        }
    }
    // TODO: revisit whether sort and dedup is necessary here or if invariants
    // established beforehand guarantee them already.
    heap_ptr_offsets.sort_by_key(|o| o.0);
    heap_ptr_offsets.dedup();

    // Byte size of the parameter region: past the last parameter.
    let param_region_size: u32 = ctx
        .home_slots
        .iter()
        .take(func_ir.num_params as usize)
        .last()
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
/// `bail!`s there must return `false` here.
pub fn gc_layout_supports(ty: InternedType) -> bool {
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
        Type::Nominal { .. } => {
            // Mirrors the bails in `type_pointer_offsets`'s Nominal arm:
            // unpopulated layout, enum (no field_layouts), or unsupported field.
            let Some(layout) = view_type(ty).layout() else {
                return false;
            };
            let Some(fields) = layout.field_layouts() else {
                return false;
            };
            fields.iter().all(|f| gc_layout_supports(f.ty()))
        },
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
pub fn type_pointer_offsets(ty: InternedType) -> Result<Vec<u32>> {
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

        // Inline structs: walk each field's pointer offsets and shift
        // by the field's byte offset within the struct.
        // TODO: Enums.
        //
        // TODO: rewrite without recursion or add a depth/visited bound;
        // a malformed or racing `NominalLayout` publisher could otherwise
        // produce a cyclic layout that blows the stack here.
        Type::Nominal { .. } => {
            let layout = view_type(ty)
                .layout()
                .ok_or_else(|| anyhow::anyhow!("nominal type has no layout populated"))?;
            let Some(fields) = layout.field_layouts() else {
                bail!("enum type in frame slot not yet supported by gc_layout");
            };
            let mut out = vec![];
            for field in fields {
                for rel in type_pointer_offsets(field.ty())? {
                    let abs = field.offset.checked_add(rel).ok_or_else(|| {
                        anyhow::anyhow!(
                            "gc_layout: field.offset {} + inner offset {} overflows u32",
                            field.offset,
                            rel,
                        )
                    })?;
                    out.push(abs);
                }
            }
            out
        },

        Type::TypeParam { .. } => {
            bail!("type parameter reached gc_layout — try_build_context should have skipped");
        },
    };
    Ok(offsets)
}
