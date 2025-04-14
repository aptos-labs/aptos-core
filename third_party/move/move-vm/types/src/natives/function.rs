// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Native Function Support
//!
//! All Move native functions have the following signature:
//!
//! `pub fn native_function(
//!     context: &mut impl NativeContext,
//!     ty_args: Vec<Type>,
//!     mut arguments: VecDeque<Value>,
//! ) -> PartialVMResult<NativeResult>;`
//!
//! arguments are passed with first argument at position 0 and so forth.
//! Popping values from `arguments` gives the aguments in reverse order (last first).
//! This module contains the declarations and utilities to implement a native
//! function.

use crate::{loaded_data::runtime_types::Type, values::Value};
pub use move_binary_format::errors::{PartialVMError, PartialVMResult};
pub use move_core_types::{gas_algebra::InternalGas, vm_status::StatusCode};
use move_core_types::{
    gas_algebra::{NumBytes, NumModules},
    identifier::Identifier,
    language_storage::ModuleId,
};
use smallvec::{smallvec, SmallVec};

/// Result of a native function execution requires charges for execution cost.
///
/// An execution that causes an invariant violation would not return a `NativeResult` but
/// return a `PartialVMError` error directly.
/// All native functions must return a `PartialVMResult<NativeResult>` where an `Err` is returned
/// when an error condition is met that should not charge for the execution. A common example
/// is a VM invariant violation which should have been forbidden by the verifier.
/// Errors (typically user errors and aborts) that are logically part of the function execution
/// must be expressed in a `NativeResult` with a cost and a VMStatus.
pub enum NativeResult {
    Success {
        cost: InternalGas,
        num_dependencies: NumModules,
        total_dependency_size: NumBytes,
        ret_vals: SmallVec<[Value; 1]>,
    },
    Abort {
        cost: InternalGas,
        num_dependencies: NumModules,
        total_dependency_size: NumBytes,
        abort_code: u64,
    },
    OutOfGas {
        partial_cost: InternalGas,
        partial_num_dependencies: NumModules,
        partial_total_dependency_size: NumBytes,
    },
    /// Instruct the VM to perform a control flow transfer.
    ///
    /// Note the calling convention here requires the following:
    /// The native function that performs the dispatch should have the same type signature as the dispatch target function except
    /// the native function will have an extra argument in the end to determine which function to jump to.
    ///
    /// Failing to follow this convention will result in errors in paranoid mode.
    CallFunction {
        cost: InternalGas,
        module_name: ModuleId,
        func_name: Identifier,
        ty_args: Vec<Type>,
        args: SmallVec<[Value; 1]>,
    },
    /// Instruct the VM to load up a module into the loader and charge dependency for such operation.
    LoadModule { module_name: ModuleId },
}

impl NativeResult {
    /// Return values of a successful execution.
    pub fn ok(
        cost: InternalGas,
        num_dependencies: NumModules,
        total_dependency_size: NumBytes,
        values: SmallVec<[Value; 1]>,
    ) -> Self {
        NativeResult::Success {
            cost,
            num_dependencies,
            total_dependency_size,
            ret_vals: values,
        }
    }

    /// Failed execution. The failure is a runtime failure in the function and not an invariant
    /// failure of the VM which would raise a `PartialVMError` error directly.
    /// The only thing the funciton can specify is its abort code, as if it had invoked the `Abort`
    /// bytecode instruction
    pub fn err(
        cost: InternalGas,
        num_dependencies: NumModules,
        total_dependency_size: NumBytes,
        abort_code: u64,
    ) -> Self {
        NativeResult::Abort {
            cost,
            num_dependencies,
            total_dependency_size,
            abort_code,
        }
    }

    /// A special variant indicating that the native has determined there is not enough
    /// balance to cover the full cost to get all the work done.
    ///
    /// Along with the ability to get the gas balance from the native context, this offers
    /// natives a way to emulate incremental gas metering, avoiding doing expensive operations
    /// before charging for gas.
    ///
    /// The natives are still required to return a partial cost, which the VM will pass
    /// to the gas meter for proper bookkeeping.
    pub fn out_of_gas(
        partial_cost: InternalGas,
        partial_num_dependencies: NumModules,
        partial_total_dependency_size: NumBytes,
    ) -> Self {
        NativeResult::OutOfGas {
            partial_cost,
            partial_num_dependencies,
            partial_total_dependency_size,
        }
    }

    /// Convert a `PartialVMResult<Value>` into a `PartialVMResult<NativeResult>`
    pub fn map_partial_vm_result_one(
        cost: InternalGas,
        res: PartialVMResult<Value>,
    ) -> PartialVMResult<Self> {
        let result = match res {
            Ok(val) => NativeResult::ok(cost, 0.into(), 0.into(), smallvec![val]),
            Err(err) if err.major_status() == StatusCode::ABORTED => {
                let (_, abort_code, _, _, _, _) = err.all_data();
                NativeResult::err(
                    cost,
                    // TODO(lazy-loading): double check this is correct, seems like this function
                    //   is used by third-party only. Also seems like the same pattern is used in
                    //   other places in third-party natives. For now, 0s are put there.
                    0.into(),
                    0.into(),
                    abort_code.unwrap_or(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR as u64),
                )
            },
            Err(err) => {
                return Err(err);
            },
        };
        Ok(result)
    }
}

/// Return the argument at the top of the stack.
///
/// Arguments are passed to a native as a stack with first arg at the bottom of the stack.
/// Calling this API can help in making the code more readable.
/// It's good practice to pop all arguments in locals of the native function on function entry.
#[macro_export]
macro_rules! pop_arg {
    ($arguments:ident, $t:ty) => {{
        use $crate::natives::function::{NativeResult, PartialVMError, StatusCode};
        match $arguments.pop_back().map(|v| v.value_as::<$t>()) {
            None => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            },
            Some(Err(e)) => return Err(e),
            Some(Ok(v)) => v,
        }
    }};
}
