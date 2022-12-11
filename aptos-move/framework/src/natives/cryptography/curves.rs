// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::util::{make_native_from_func, make_test_only_native_from_func};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::values::Struct;
use move_vm_types::values::Value;
use smallvec::smallvec;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

fn scalar_one_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bytes = vec![0_u8, 1_u8];
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::struct_(Struct::pack(vec![Value::vector_u8(bytes),]))],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![(
        "scalar_one",
        make_native_from_func(gas_params.clone(), scalar_one_internal),
    )]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
