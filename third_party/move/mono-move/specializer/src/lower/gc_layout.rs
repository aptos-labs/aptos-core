// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-driven derivation of GC frame layout from monomorphic slot types.
//!
//! Each frame slot's type determines whether — and at which byte
//! offset(s) within the slot — it holds a heap pointer the GC must
//! scan. This module exposes [`append_pointer_offsets`], which encodes
//! that mapping per [`mono_move_core::types::Type`] variant.

use super::context::LoweringContext;
use anyhow::{bail, Result};
use mono_move_core::{
    types::{view_type, InternedType, Type},
    FrameOffset,
};

/// `frame_layout` plus the derived `zero_frame` flag for a function.
pub struct DerivedFrameLayout {
    /// Sorted, deduplicated frame byte-offsets of pointer slots.
    pub frame_layout: Vec<FrameOffset>,
    /// `true` iff at least one offset belongs to a non-parameter home
    /// slot — the runtime must zero `param_sizes_sum..extended_frame_size`
    /// at frame creation so pointer locals start null.
    pub zero_frame: bool,
}

/// Derive `frame_layout` and `zero_frame` for a function from its home
/// slot types.
pub fn derive_frame_layout(
    ctx: &LoweringContext,
    home_slot_types: &[InternedType],
    num_params: u16,
) -> Result<DerivedFrameLayout> {
    let mut frame_layout: Vec<FrameOffset> = Vec::new();
    for (slot, &ty) in ctx.home_slots.iter().zip(home_slot_types.iter()) {
        append_pointer_offsets(ty, slot.offset.0, &mut frame_layout)?;
    }
    frame_layout.sort_by_key(|o| o.0);
    frame_layout.dedup();

    let param_sizes_sum: u32 = ctx
        .home_slots
        .iter()
        .take(num_params as usize)
        .map(|s| s.size)
        .sum();
    let zero_frame = frame_layout.iter().any(|off| off.0 >= param_sizes_sum);

    Ok(DerivedFrameLayout {
        frame_layout,
        zero_frame,
    })
}

/// For a value of type `ty` laid out at frame offset `base`, append
/// the frame-relative byte offsets of the pointer slots within the
/// value that the GC must scan.
///
/// Returns `Err` if `ty` is a variant this module doesn't yet handle.
pub fn append_pointer_offsets(
    ty: InternedType,
    base: u32,
    out: &mut Vec<FrameOffset>,
) -> Result<()> {
    match view_type(ty) {
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
        | Type::Signer => {},

        // 16-byte fat pointer: base half at `base`, scalar offset half
        // at `base + 8`. Only the base is a pointer slot.
        Type::ImmutRef { .. } | Type::MutRef { .. } => {
            out.push(FrameOffset(base));
        },

        // 8-byte heap pointer.
        Type::Vector { .. } | Type::Function { .. } => {
            out.push(FrameOffset(base));
        },

        // TODO: drill into `NominalLayout::field_layouts()` to
        // surface internal pointer offsets, treating fieldless
        // (enum) layouts as a single 8-byte heap-pointer slot.
        Type::Nominal { .. } => {
            bail!("nominal type in frame slot not yet supported by gc_layout");
        },

        Type::TypeParam { .. } => {
            bail!("type parameter reached gc_layout — try_build_context should have skipped");
        },
    }
    Ok(())
}
