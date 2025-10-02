// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;
use smallvec::{smallvec, SmallVec};
use aptos_native_interface::{SafeNativeContext, SafeNativeResult};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::values::Value;
use crate::natives::transaction_context::NativeTransactionContext;

fn native_process_positions_for_status_native(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    //let transaction_context = context.extensions().get::<NativeTransactionContext>();
    println!("native_process_positions_for_status_native ...........");
    Ok(smallvec![Value::u32(5)])
}


#[test]
fn dummy_test() {
    println!("dummy_test ...........");
}
