// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Helpers for working with the Move `Result` type.

use move_vm_types::{
    natives::function::{PartialVMError, PartialVMResult},
    values::{Struct, StructRef, Value},
};

const OK_VARIANT: u16 = 0;
const ERR_VARIANT: u16 = 1;

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
            "invalid result value",
        )),
    }
}

pub fn unwrap_err_result(value: Value) -> PartialVMResult<Value> {
    let (tag, mut vals) = value.value_as::<Struct>()?.unpack_with_tag()?;
    let val = vals.next();
    match val {
        Some(x) if tag == ERR_VARIANT => Ok(x),
        _ => Err(PartialVMError::new_invariant_violation(
            "invalid result value",
        )),
    }
}
