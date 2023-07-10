// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMError;

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
