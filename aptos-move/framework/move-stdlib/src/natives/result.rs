// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers for working with the Move `Result` type.

use aptos_types::move_stdlib::result::{ERR_TAG as ERR_VARIANT, OK_TAG as OK_VARIANT};
use move_vm_types::{
    natives::function::{PartialVMError, PartialVMResult},
    values::{Struct, StructRef, Value},
};

pub fn ok_result(val: Value) -> Value {
    Value::struct_(Struct::pack_variant(OK_VARIANT, vec![val]))
}

pub fn err_result(err: Value) -> Value {
    Value::struct_(Struct::pack_variant(ERR_VARIANT, vec![err]))
}

pub fn is_ok_result_ref(value: Value) -> PartialVMResult<bool> {
    value
        .value_as::<StructRef>()?
        .test_variant(OK_VARIANT)?
        .value_as::<bool>()
}

pub fn unwrap_result(value: Value) -> PartialVMResult<Value> {
    let (tag, mut vals) = value.value_as::<Struct>()?.unpack_with_tag()?;
    let val = vals.next();
    match val {
        Some(x) if tag == OK_VARIANT => Ok(x),
        _ => Err(PartialVMError::new_invariant_violation(
            "invalid result value: expected Ok(_)",
        )),
    }
}

pub fn unwrap_err_result(value: Value) -> PartialVMResult<Value> {
    let (tag, mut vals) = value.value_as::<Struct>()?.unpack_with_tag()?;
    let val = vals.next();
    match val {
        Some(x) if tag == ERR_VARIANT => Ok(x),
        _ => Err(PartialVMError::new_invariant_violation(
            "invalid result value: expected Err(_)",
        )),
    }
}
