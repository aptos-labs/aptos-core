// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `vector` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref},
    VMResult,
};

/// Positions/length fall outside the vectors.
const EINDEX_OUT_OF_BOUNDS: u64 = 1;

/// `0x1::vector::move_range<T>(from: &mut vector<T>, removal_position: u64, length: u64, to: &mut vector<T>, insert_position: u64)`
//
// TODO(metering): charge gas
pub fn native_move_range<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let elem_ty = ctx.ty_arg(0)?;
    // SAFETY: the ABI is `(&mut vector<T>, u64, u64, &mut vector<T>, u64)`.
    let in_bounds = unsafe {
        let from: Ref<Opaque> = ctx.arg(0)?;
        let removal_position: u64 = ctx.arg(1)?;
        let length: u64 = ctx.arg(2)?;
        let to: Ref<Opaque> = ctx.arg(3)?;
        let insert_position: u64 = ctx.arg(4)?;
        ctx.vector_move_range(
            &from,
            removal_position,
            length,
            &to,
            insert_position,
            elem_ty,
        )?
    };
    if in_bounds {
        Ok(NativeStatus::Success)
    } else {
        Ok(NativeStatus::Abort {
            code: EINDEX_OUT_OF_BOUNDS,
            message: None,
        })
    }
}

/// Natives for the `vector` module.
pub fn make_all_vector_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::vector::move_range", native_move_range)]
}
