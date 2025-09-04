// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_gas_schedule::{
    gas_feature_versions::RELEASE_V1_23,
    gas_params::natives::velor_framework::RANDOMNESS_FETCH_AND_INC_COUNTER,
};
use velor_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use better_any::{Tid, TidAble};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

#[derive(Tid, Default)]
pub struct RandomnessContext {
    // A txn-local 8-byte counter that increments each time a random 32-byte
    // blob is requested.
    txn_local_state: Vec<u8>,
    // True if the current transaction's payload was a public(friend) or
    // private entry function, which also has `#[randomness]` annotation.
    unbiasable: bool,
}

impl RandomnessContext {
    pub fn new() -> Self {
        Self {
            txn_local_state: vec![0; 8],
            unbiasable: false,
        }
    }

    pub fn increment(&mut self) {
        for byte in self.txn_local_state.iter_mut() {
            if *byte < 255 {
                *byte += 1;
                break;
            } else {
                *byte = 0;
            }
        }
    }

    pub fn mark_unbiasable(&mut self) {
        self.unbiasable = true;
    }

    pub fn is_unbiasable(&self) -> bool {
        self.unbiasable
    }
}

pub fn fetch_and_increment_txn_counter(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if context.gas_feature_version() >= RELEASE_V1_23 {
        context.charge(RANDOMNESS_FETCH_AND_INC_COUNTER)?;
    }

    let ctx = context.extensions_mut().get_mut::<RandomnessContext>();
    if !ctx.is_unbiasable() {
        return Err(SafeNativeError::Abort {
            abort_code: E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT,
        });
    }

    let ret = ctx.txn_local_state.to_vec();
    ctx.increment();
    Ok(smallvec![Value::vector_u8(ret)])
}

pub fn is_unbiasable(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // Because we need to run a special transaction prologue to pre-charge maximum
    // amount of gas, we require all callers to have an annotation that the entry
    // function call is unbiasable. This property is only checked at runtime here.
    let is_unbiasable = context
        .extensions()
        .get::<RandomnessContext>()
        .is_unbiasable();

    // TODO: charge gas?
    Ok(smallvec![Value::bool(is_unbiasable)])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = vec![
        (
            "fetch_and_increment_txn_counter",
            fetch_and_increment_txn_counter as RawSafeNative,
        ),
        ("is_unbiasable", is_unbiasable),
    ];

    builder.make_named_natives(natives)
}
