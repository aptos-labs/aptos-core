// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMError;
use move_core_types::{identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::SmallVec;

/// Wraps [PartialVMError] to ensure it cannot be constructed via public constructor when we create
/// a [LimitExceededError].
pub struct MeteringError(PartialVMError);

impl MeteringError {
    pub fn unpack(self) -> PartialVMError {
        self.0
    }
}

/// Specifies different ways of exceeding the limit.
pub enum LimitExceededError {
    /// Represents legacy out of gas status. Mapped to [StatusCode::OUT_OF_GAS]. Does not represent
    /// anything else, e.g., reaching memory limits, loading too many dependencies.
    LegacyOutOfGas,
    /// Error due to metering. The inner value contains the VM error which can be later returned to
    /// interpreter.
    LimitExceeded(MeteringError),
}

impl LimitExceededError {
    pub fn from_err(err: PartialVMError) -> SafeNativeError {
        match err.major_status() {
            StatusCode::OUT_OF_GAS
            | StatusCode::EXECUTION_LIMIT_REACHED
            | StatusCode::DEPENDENCY_LIMIT_REACHED
            | StatusCode::MEMORY_LIMIT_EXCEEDED
            | StatusCode::TOO_MANY_TYPE_NODES
            | StatusCode::VM_MAX_VALUE_DEPTH_REACHED => SafeNativeError::LimitExceeded(
                LimitExceededError::LimitExceeded(MeteringError(err)),
            ),
            // Treat all other code as invariant violations and leave it for the VM to propagate
            // these further. Note that we do not remap the errors. For example, if there is a
            // speculative error returned (signaling Block-STM to stop executing this transaction),
            // we better not remap it.
            // TODO(Gas): Have a single method to convert partial VM error to safe native error.
            _ => SafeNativeError::InvariantViolation(err),
        }
    }
}

/// Saner representation of a native function error.
#[allow(unused)]
pub enum SafeNativeError {
    /// Indicating that the native function has aborted due to some (user) errors.
    ///
    /// Equivalent to aborting in a regular Move function, so the same error convention should
    /// be followed.
    Abort { abort_code: u64 },

    /// Indicating that the native function has exceeded execution limits.
    ///
    /// If metering in native context is not enabled, this will cause the VM to deduct all the
    /// remaining balance and abort the transaction, so use it carefully! Normally this should only
    /// be triggered by `SafeNativeContext::charge()` and one should not return this variant
    /// manually without a good reason.
    ///
    /// If metering in native context is enabled, then simply returns the error code that specifies
    /// the limit that was exceeded.
    LimitExceeded(LimitExceededError),

    /// Indicating that the native function ran into some internal errors that shall not normally
    /// be triggerable by user inputs.
    InvariantViolation(PartialVMError),

    /// Indicating the native function will result in a switch in control flow.
    ///
    /// Please refer to the implementation in aptos_framework::natives::dispatchable_fungible_asset::native_dispatch
    /// for reference implementation and avoid having an alternative implementation.
    ///
    /// It is important to make sure the args are in the exact same order as passed in from the native argument input
    /// as the MoveVM relies on this ordering to perform paranoid mode stack transition.
    FunctionDispatch {
        module_name: ModuleId,
        func_name: Identifier,
        ty_args: Vec<Type>,
        args: SmallVec<[Value; 1]>,
    },

    /// Load up a module and charge the module accordingly.
    ///
    /// It is critical to invoke this function before calling FunctionDispatch to make sure the
    /// module loading is charged properly, otherwise it would be a potential gas issue.
    ///
    /// Note: not used once metering in native context is enabled.
    LoadModule { module_name: ModuleId },
}

// Allows us to keep using the `?` operator on function calls that return `PartialVMResult` inside safe natives.
//
// TODO(Gas): This automatic conversion is VERY PROBLEMATIC as it makes it extremely easy to
//            accidentally propagate a non-invariant violation, which is a violation of the
//            contract.
//
//            We are actively seeking to remove this implementation.
//            Please help us stop the bleed by not using this conversion.
impl From<PartialVMError> for SafeNativeError {
    fn from(e: PartialVMError) -> Self {
        SafeNativeError::InvariantViolation(e)
    }
}

pub type SafeNativeResult<T> = Result<T, SafeNativeError>;
