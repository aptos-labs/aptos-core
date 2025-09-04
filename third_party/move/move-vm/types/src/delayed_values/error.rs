// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;

pub(crate) fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> PartialVMResult<V> {
    value.map_err(code_invariant_error)
}

pub fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PartialVMError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    println!("ERROR: {}", msg);
    PartialVMError::new(StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR)
        .with_message(msg)
}
