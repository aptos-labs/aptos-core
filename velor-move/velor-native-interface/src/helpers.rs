// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// Like `pop_arg!` but for safe natives that return `SafeNativeResult<T>`. Will return a
/// `SafeNativeError::InvariantViolation(UNKNOWN_INVARIANT_VIOLATION_ERROR)` when there aren't
/// enough arguments on the stack.
#[macro_export]
macro_rules! safely_pop_arg {
    ($args:ident, $t:ty) => {{
        use $crate::reexports::move_vm_types::natives::function::{PartialVMError, StatusCode};
        match $args.pop_back() {
            Some(val) => match val.value_as::<$t>() {
                Ok(v) => v,
                Err(e) => return Err($crate::SafeNativeError::InvariantViolation(e)),
            },
            None => {
                return Err($crate::SafeNativeError::InvariantViolation(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                ))
            },
        }
    }};
}

/// Returns a field value of the specified type from a struct at a given index.
/// If the field access is out of bounds, or type is incorrect, a
/// `SafeNativeError::InvariantViolation` is returned.
#[macro_export]
macro_rules! safely_get_struct_field_as {
    ($value:expr, $idx:expr, $t:ty) => {{
        // Note: we remap errors to safe errors in order to avoid implicit
        //       conversions via `Into`.
        $value
            .borrow_field($idx)
            .map_err($crate::SafeNativeError::InvariantViolation)?
            .value_as::<Reference>()
            .map_err($crate::SafeNativeError::InvariantViolation)?
            .read_ref()
            .map_err($crate::SafeNativeError::InvariantViolation)?
            .value_as::<$t>()
            .map_err($crate::SafeNativeError::InvariantViolation)?
    }};
}

/// Like `assert_eq!` but for safe natives that return `SafeNativeResult<T>`. Instead of panicking,
/// will return a `SafeNativeError::InvariantViolation(UNKNOWN_INVARIANT_VIOLATION_ERROR)`.
#[macro_export]
macro_rules! safely_assert_eq {
    ($left:expr, $right:expr $(,)?) => {{
        use $crate::reexports::move_vm_types::natives::function::{PartialVMError, StatusCode};
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    return Err($crate::SafeNativeError::InvariantViolation(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                    ));
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
        use $crate::reexports::move_vm_types::natives::function::{PartialVMError, StatusCode};
        match $ty_args.pop() {
            Some(ty) => ty,
            None => {
                return Err($crate::SafeNativeError::InvariantViolation(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
                ))
            },
        }
    }};
}

/// Like `pop_vec_arg` but for safe natives that return `SafeNativeResult<T>`.
/// (Duplicates code from above, unfortunately.)
#[macro_export]
macro_rules! safely_pop_vec_arg {
    ($arguments:ident, $t:ty) => {{
        // Replicating the code from pop_arg! here
        use $crate::reexports::move_vm_types::natives::function::{PartialVMError, StatusCode};
        let value_vec = match $arguments.pop_back().map(|v| v.value_as::<Vec<Value>>()) {
            None => {
                return Err($crate::SafeNativeError::InvariantViolation(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                ))
            }
            Some(Err(e)) => return Err($crate::SafeNativeError::InvariantViolation(e)),
            Some(Ok(v)) => v,
        };

        // Pop each Value from the popped Vec<Value>, cast it as a Vec<u8>, and push it to a Vec<Vec<u8>>
        let mut vec_vec = vec![];
        for value in value_vec {
            let vec = match value.value_as::<$t>() {
                Err(e) => return Err($crate::SafeNativeError::InvariantViolation(e)),
                Ok(v) => v,
            };
            vec_vec.push(vec);
        }

        vec_vec
    }};
}
