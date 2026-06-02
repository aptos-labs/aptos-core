// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Synthetic natives used by the differential harness. Expected to go
//! away once real natives are wired up.

use crate::{natives, NativeFunction};
use mono_move_core::native::{NativeContext, NativeContextFamily, NativeResult, VMInternalError};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

pub fn native_u64_add<C: NativeContext>(ctx: &mut C) -> Result<NativeResult, VMInternalError> {
    let a: u64 = ctx.arg(0)?;
    let b: u64 = ctx.arg(1)?;
    let sum = match a.checked_add(b) {
        Some(s) => s,
        None => {
            return Ok(NativeResult::Abort {
                code: 1,
                message: None,
            })
        },
    };
    ctx.set_return(0, sum)?;
    Ok(NativeResult::Success)
}

pub fn native_u64_identity<C: NativeContext>(ctx: &mut C) -> Result<NativeResult, VMInternalError> {
    let x: u64 = ctx.arg(0)?;
    ctx.set_return(0, x)?;
    Ok(NativeResult::Success)
}

pub fn make_all_test_natives<F: NativeContextFamily>(
) -> Vec<(AccountAddress, Identifier, Identifier, NativeFunction<F>)> {
    natives![
        ("0x1::test_natives::u64_add", native_u64_add),
        ("0x1::test_natives::u64_identity", native_u64_identity),
    ]
}
