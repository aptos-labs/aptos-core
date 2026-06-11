// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `vector` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref, VMInternalError, Vector,
    },
    types::view_type,
};

/// Given positions/lengths fall outside the vector boundaries.
const EINDEX_OUT_OF_BOUNDS: u64 = 1;

/// `0x1::vector::move_range<T>(from: &mut vector<T>, removal_position: u64, length: u64, to: &mut vector<T>, insert_position: u64)`
///
/// Removes `length` elements from `from` starting at `removal_position` and
/// inserts them into `to` at `insert_position`.
pub fn native_move_range<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // TODO: charge gas.
    let (size, _) = view_type(ctx.ty_arg(0)?).size_and_align().ok_or_else(|| {
        VMInternalError::InvariantViolation("vector::move_range: element type has no size".into())
    })?;
    let elem_size = size as usize;

    // SAFETY: args 0/3 are `&mut vector<T>`, args 1/2/4 are `u64`, and
    // `elem_size` is the byte size of `T`.
    unsafe {
        let from: Ref<Vector<Opaque>> = ctx.arg(0)?;
        let removal_position = ctx.arg::<u64>(1)? as usize;
        let length = ctx.arg::<u64>(2)? as usize;
        let to: Ref<Vector<Opaque>> = ctx.arg(3)?;
        let insert_position = ctx.arg::<u64>(4)? as usize;

        // Bounds are checked here so the abort code matches the legacy native.
        let from_len = from.borrow().len() as usize;
        let to_len = to.borrow().len() as usize;
        if removal_position
            .checked_add(length)
            .is_none_or(|end| end > from_len)
            || insert_position > to_len
        {
            return Ok(NativeStatus::Abort {
                code: EINDEX_OUT_OF_BOUNDS,
                message: None,
            });
        }

        // Moving nothing is a no-op (and avoids allocating an empty `to`).
        if length == 0 {
            return Ok(NativeStatus::Success);
        }

        to.move_range(
            &from,
            removal_position,
            length,
            insert_position,
            elem_size,
            ctx,
        )?;
    }
    Ok(NativeStatus::Success)
}

/// Natives for the `vector` module.
pub fn make_all_vector_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::vector::move_range", native_move_range)]
}
