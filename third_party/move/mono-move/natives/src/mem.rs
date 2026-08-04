// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native for the `mem` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref},
    VMResult,
};

/// `0x1::mem::swap<T>(left: &mut T, right: &mut T)`
///
/// Exchanges the values behind the two references.
pub fn native_swap<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let size = ctx.value_size(ctx.ty_arg(0)?)? as usize;
    // SAFETY: arg 0 / arg 1 are `&mut T`, each holding `size` valid bytes.
    unsafe {
        let left: Ref<Opaque> = ctx.arg(0)?;
        let right: Ref<Opaque> = ctx.arg(1)?;
        // TODO(correctness): `swap_nonoverlapping` would be UB if the two referents
        // ever alias. Move borrow rules forbid that, but it's unclear whether
        // Block-STM speculative execution can transiently produce aliasing
        // references — so swap via a temporary, which is sound either way until
        // we've confirmed disjointness holds under speculation.
        let mut tmp = vec![0u8; size];
        std::ptr::copy_nonoverlapping(left.ptr(), tmp.as_mut_ptr(), size);
        std::ptr::copy(right.ptr(), left.ptr(), size);
        std::ptr::copy_nonoverlapping(tmp.as_ptr(), right.ptr(), size);
    }
    Ok(NativeStatus::Success)
}

/// Natives for the `mem` module.
pub fn make_all_mem_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::mem::swap", native_swap)]
}
