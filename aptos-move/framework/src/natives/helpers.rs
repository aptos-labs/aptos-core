// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::{TimedFeatureFlag, TimedFeatures},
    vm_status::StatusCode,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use smallvec::SmallVec;
use std::{collections::VecDeque, sync::Arc};

/// Used to pop a Vec<Vec<u8>> argument off the stack.
#[macro_export]
macro_rules! pop_vec_arg {
    ($arguments:ident, $t:ty) => {{
        // Replicating the code from pop_arg! here
        use move_vm_types::natives::function::{PartialVMError, StatusCode};
        let value_vec = match $arguments.pop_back().map(|v| v.value_as::<Vec<Value>>()) {
            None => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            }
            Some(Err(e)) => return Err(e),
            Some(Ok(v)) => v,
        };

        // Pop each Value from the popped Vec<Value>, cast it as a Vec<u8>, and push it to a Vec<Vec<u8>>
        let mut vec_vec = vec![];
        for value in value_vec {
            let vec = match value.value_as::<$t>() {
                Err(e) => return Err(e),
                Ok(v) => v,
            };
            vec_vec.push(vec);
        }

        vec_vec
    }};
}

pub fn make_module_natives(
    natives: impl IntoIterator<Item = (impl Into<String>, NativeFunction)>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    natives
        .into_iter()
        .map(|(func_name, func)| (func_name.into(), func))
}

#[allow(unused)]
/// Wraps a test-only native function inside an Arc<UnboxedNativeFunction>.
pub fn make_test_only_native_from_func(
    func: fn(&mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>,
) -> NativeFunction {
    Arc::new(func)
}

/// Used to pass gas parameters into native functions.
pub fn make_native_from_func<G>(
    gas_params: G,
    func: fn(&G, &mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>,
) -> NativeFunction
where
    G: Send + Sync + 'static,
{
    Arc::new(move |context, ty_args, args| func(&gas_params, context, ty_args, args))
}

#[macro_export]
macro_rules! pop_arg_safe {
    ($args:ident, $t:ty) => {
        match $args.pop_back() {
            Some(val) => match val.value_as::<$t>() {
                Ok(v) => v,
                Err(_e) => {
                    return Err($crate::natives::helpers::SafeNativeError::InvariantViolation)
                },
            },
            None => return Err($crate::natives::helpers::SafeNativeError::InvariantViolation),
        }
    };
}

#[allow(unused)]
pub struct SafeNativeContext<'a, 'b, 'c> {
    timed_features: &'c TimedFeatures,
    inner: &'c mut NativeContext<'a, 'b>,

    gas_budget: InternalGas,
    gas_used: InternalGas,
}

impl<'a, 'b, 'c> SafeNativeContext<'a, 'b, 'c> {
    pub fn charge(&mut self, amount: InternalGas) -> SafeNativeResult<()> {
        self.gas_used += amount;

        if self.gas_used > self.gas_budget
            && self
                .timed_features
                .is_enabled(TimedFeatureFlag::NativesAbortEarlyIfOutOfGas)
        {
            Err(SafeNativeError::OutOfGas)
        } else {
            Ok(())
        }
    }
}

#[allow(unused)]
pub enum SafeNativeError {
    Abort { abort_code: u64 },
    OutOfGas,
    InvariantViolation,
}

pub type SafeNativeResult<T> = Result<T, SafeNativeError>;

pub fn make_safe_native<G>(
    gas_params: G,
    timed_features: TimedFeatures,
    func: fn(
        &G,
        &mut SafeNativeContext,
        Vec<Type>,
        VecDeque<Value>,
    ) -> SafeNativeResult<SmallVec<[Value; 1]>>,
) -> NativeFunction
where
    G: Send + Sync + 'static,
{
    let closure = move |context: &mut NativeContext, ty_args, args| {
        use SafeNativeError::*;

        let gas_budget = context.gas_balance();

        let mut context = SafeNativeContext {
            timed_features: &timed_features,
            inner: context,

            gas_budget,
            gas_used: 0.into(),
        };

        let res = func(&gas_params, &mut context, ty_args, args);

        match res {
            Ok(ret_vals) => Ok(NativeResult::ok(context.gas_used, ret_vals)),
            Err(err) => match err {
                Abort { abort_code } => Ok(NativeResult::err(context.gas_used, abort_code)),
                OutOfGas => Ok(NativeResult::out_of_gas(context.gas_used)),
                InvariantViolation => Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                )),
            },
        }
    };

    Arc::new(closure)
}
