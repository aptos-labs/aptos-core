// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Synthetic natives used by the differential harness. Expected to go
//! away once real natives are wired up.

use crate::{monomorphic_natives, polymorphic_natives, NativeEntry};
use mono_move_core::native::{NativeContext, NativeContextFamily, NativeStatus, VMInternalError};

pub fn native_u64_add<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: u64 matches the Move-level `u64` type of args 0/1 and return 0.
    let a: u64 = unsafe { ctx.arg(0) }?;
    let b: u64 = unsafe { ctx.arg(1) }?;
    let sum = match a.checked_add(b) {
        Some(s) => s,
        None => {
            return Ok(NativeStatus::Abort {
                code: 1,
                message: None,
            })
        },
    };
    unsafe { ctx.set_return(0, sum) }?;
    Ok(NativeStatus::Success)
}

pub fn native_u64_identity<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: u64 matches the Move-level `u64` type of arg 0 and return 0.
    let x: u64 = unsafe { ctx.arg(0) }?;
    unsafe { ctx.set_return(0, x) }?;
    Ok(NativeStatus::Success)
}

pub fn make_all_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = monomorphic_natives![
        ("0x1::test_natives::u64_add", native_u64_add),
        ("0x1::test_natives::u64_identity", native_u64_identity),
    ];
    // The plain move-stdlib `bcs` module declares only `to_bytes`, so the
    // differential harness reaches `bcs::serialized_size` through this test
    // module instead (the V1 side re-registers the legacy impl here too).
    natives.extend(polymorphic_natives![(
        "0x1::test_natives::serialized_size",
        crate::bcs::native_serialized_size
    )]);
    natives
}
