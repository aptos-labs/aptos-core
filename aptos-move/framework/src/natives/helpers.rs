// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::on_chain_config::{Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use smallvec::SmallVec;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Used to pop a `Vec<Vec<u8>>` or `Vec<Struct>` argument off the stack in unsafe natives that return `PartialVMResult<T>`.
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

/// Like `pop_vec_arg` but for safe natives that return `SafeNativeResult<T>`.
/// (Duplicates code from above, unfortunately.)
#[macro_export]
macro_rules! safely_pop_vec_arg {
    ($arguments:ident, $t:ty) => {{
        // Replicating the code from pop_arg! here
        use move_vm_types::natives::function::{PartialVMError, StatusCode};
        let value_vec = match $arguments.pop_back().map(|v| v.value_as::<Vec<Value>>()) {
            None => {
                return Err($crate::natives::helpers::SafeNativeError::InvariantViolation(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                ))
            }
            Some(Err(e)) => return Err($crate::natives::helpers::SafeNativeError::InvariantViolation(e)),
            Some(Ok(v)) => v,
        };

        // Pop each Value from the popped Vec<Value>, cast it as a Vec<u8>, and push it to a Vec<Vec<u8>>
        let mut vec_vec = vec![];
        for value in value_vec {
            let vec = match value.value_as::<$t>() {
                Err(e) => return Err($crate::natives::helpers::SafeNativeError::InvariantViolation(e)),
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

/// Like `pop_arg!` but for safe natives that return `SafeNativeResult<T>`. Will return a
/// `SafeNativeError::InvariantViolation(UNKNOWN_INVARIANT_VIOLATION_ERROR)` when there aren't
/// enough arguments on the stack.
#[macro_export]
macro_rules! safely_pop_arg {
    ($args:ident, $t:ty) => {{
        use move_vm_types::natives::function::{PartialVMError, StatusCode};
        match $args.pop_back() {
            Some(val) => match val.value_as::<$t>() {
                Ok(v) => v,
                Err(e) => {
                    return Err($crate::natives::helpers::SafeNativeError::InvariantViolation(e))
                },
            },
            None => {
                return Err(
                    $crate::natives::helpers::SafeNativeError::InvariantViolation(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                    ),
                )
            },
        }
    }};
}

/// Like `assert_eq!` but for safe natives that return `SafeNativeResult<T>`. Instead of panicking,
/// will return a `SafeNativeError::InvariantViolation(UNKNOWN_INVARIANT_VIOLATION_ERROR)`.
#[macro_export]
macro_rules! safely_assert_eq {
    ($left:expr, $right:expr $(,)?) => {{
        use move_vm_types::natives::function::{PartialVMError, StatusCode};
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    return Err(
                        $crate::natives::helpers::SafeNativeError::InvariantViolation(
                            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                        ),
                    );
                }
            },
        }
    }};
}

/// Pops a `Type` argument off the type argument stack inside a safe native. Returns a
/// `SafeNativeError::InvariantViolation(UNKNOWN_INVARIANT_VIOLATION_ERROR)` in case there are not
/// enough arguments on the stack.
///
/// NOTE: Expects as its argument an object that has a `fn pop(&self) -> Option<_>` method (e.g., a `Vec<_>`)
#[macro_export]
macro_rules! safely_pop_type_arg {
    ($ty_args:ident) => {{
        use move_vm_types::natives::function::{PartialVMError, StatusCode};
        match $ty_args.pop() {
            Some(ty) => ty,
            None => {
                return Err(
                    $crate::natives::helpers::SafeNativeError::InvariantViolation(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                    ),
                )
            },
        }
    }};
}

#[allow(unused)]
pub struct SafeNativeContext<'a, 'b, 'c, 'd> {
    timed_features: &'c TimedFeatures,
    features: Arc<Features>,
    inner: &'c mut NativeContext<'a, 'b, 'd>,

    gas_budget: InternalGas,
    gas_used: InternalGas,
}

impl<'a, 'b, 'c, 'd> Deref for SafeNativeContext<'a, 'b, 'c, 'd> {
    type Target = NativeContext<'a, 'b, 'd>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, 'b, 'c, 'd> DerefMut for SafeNativeContext<'a, 'b, 'c, 'd> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'a, 'b, 'c, 'd> SafeNativeContext<'a, 'b, 'c, 'd> {
    /// Always remember: first charge gas, then execute!
    ///
    /// In other words, this function **MUST** always be called **BEFORE** executing **any**
    /// gas-metered operation or library call within a native function.
    #[must_use = "must always propagate the error returned by this function to the native function that called it using the ? operator"]
    pub fn charge(&mut self, amount: InternalGas) -> SafeNativeResult<()> {
        self.gas_used += amount;

        if self.gas_used > self.gas_budget {
            Err(SafeNativeError::OutOfGas)
        } else {
            Ok(())
        }
    }

    pub fn get_feature_flags(&self) -> &Features {
        self.features.deref()
    }

    pub fn timed_feature_enabled(&self, flag: TimedFeatureFlag) -> bool {
        self.timed_features.is_enabled(flag)
    }
}

#[allow(unused)]
pub enum SafeNativeError {
    Abort { abort_code: u64 },
    OutOfGas,
    InvariantViolation(PartialVMError),
}

/// Allows us to keep using the `?` operator on function calls that return `PartialVMResult` inside safe natives.
impl From<PartialVMError> for SafeNativeError {
    fn from(e: PartialVMError) -> Self {
        SafeNativeError::InvariantViolation(e)
    }
}

pub type SafeNativeResult<T> = Result<T, SafeNativeError>;

pub fn make_safe_native<G>(
    gas_params: G,
    timed_features: TimedFeatures,
    features: Arc<Features>,
    func: impl Fn(
            &G,
            &mut SafeNativeContext,
            Vec<Type>,
            VecDeque<Value>,
        ) -> SafeNativeResult<SmallVec<[Value; 1]>>
        + Sync
        + Send
        + 'static,
) -> NativeFunction
where
    G: Send + Sync + 'static,
{
    let closure = move |context: &mut NativeContext, ty_args, args| {
        use SafeNativeError::*;

        let gas_budget = context.gas_balance();

        let mut context = SafeNativeContext {
            timed_features: &timed_features,
            features: features.clone(),
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
                InvariantViolation(err) => Err(err),
            },
        }
    };

    Arc::new(closure)
}

/// For all $n > 0$, returns $\floor{\log_2{n}}$, contained within a `Some`.
/// For $n = 0$, returns `None`.
pub fn log2_floor(n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }

    // NOTE: n > 0, so n.leading_zeros() cannot equal usize::BITS. Therefore, we will never cast -1 to a usize.
    Some(((usize::BITS - n.leading_zeros()) - 1) as usize)
}

#[test]
fn test_log2_floor() {
    assert_eq!(log2_floor(usize::MIN), None);
    assert_eq!(log2_floor(0), None);
    assert_eq!(log2_floor(1), Some(0));
    assert_eq!(log2_floor(2), Some(1));
    assert_eq!(log2_floor(3), Some(1));
    assert_eq!(log2_floor(4), Some(2));
    assert_eq!(log2_floor(5), Some(2));
    assert_eq!(log2_floor(6), Some(2));
    assert_eq!(log2_floor(7), Some(2));
    assert_eq!(log2_floor(8), Some(3));
    assert_eq!(log2_floor(9), Some(3));
    assert_eq!(log2_floor(10), Some(3));
    assert_eq!(log2_floor(11), Some(3));
    assert_eq!(log2_floor(12), Some(3));
    assert_eq!(log2_floor(13), Some(3));
    assert_eq!(log2_floor(14), Some(3));
    assert_eq!(log2_floor(15), Some(3));
    assert_eq!(log2_floor(16), Some(4));
    assert_eq!(log2_floor(usize::MAX), Some((usize::BITS - 1) as usize));
}

/// For all $n > 0$, returns $\ceil{\log_2{n}}$, contained within a `Some`.
/// For $n = 0$, returns `None`.
pub fn log2_ceil(n: usize) -> Option<usize> {
    match n {
        0 => None,
        1 => Some(0),
        _ => Some(log2_floor(n - 1).unwrap() + 1),
    }
}

#[test]
fn test_log2_ceil() {
    assert_eq!(log2_ceil(usize::MIN), None);
    assert_eq!(log2_ceil(0), None);
    assert_eq!(log2_ceil(1), Some(0));
    assert_eq!(log2_ceil(2), Some(1));
    assert_eq!(log2_ceil(3), Some(2));
    assert_eq!(log2_ceil(4), Some(2));
    assert_eq!(log2_ceil(5), Some(3));
    assert_eq!(log2_ceil(6), Some(3));
    assert_eq!(log2_ceil(7), Some(3));
    assert_eq!(log2_ceil(8), Some(3));
    assert_eq!(log2_ceil(9), Some(4));
    assert_eq!(log2_ceil(10), Some(4));
    assert_eq!(log2_ceil(11), Some(4));
    assert_eq!(log2_ceil(12), Some(4));
    assert_eq!(log2_ceil(13), Some(4));
    assert_eq!(log2_ceil(14), Some(4));
    assert_eq!(log2_ceil(15), Some(4));
    assert_eq!(log2_ceil(16), Some(4));
    assert_eq!(log2_ceil(usize::MAX), Some(usize::BITS as usize));
}
