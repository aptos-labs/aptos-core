// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;

pub const ABORT_MESSAGE_SIZE_LIMIT: usize = 1024;

/// Returns `Err` if `bytes_len` exceeds [`ABORT_MESSAGE_SIZE_LIMIT`].
pub fn check_abort_message_limit(bytes_len: usize) -> Result<(), PartialVMError> {
    if bytes_len > ABORT_MESSAGE_SIZE_LIMIT {
        return Err(
            PartialVMError::new(StatusCode::ABORT_MESSAGE_LIMIT_EXCEEDED).with_message(format!(
                "Expected at most {} bytes, got {} bytes",
                ABORT_MESSAGE_SIZE_LIMIT, bytes_len
            )),
        );
    }
    Ok(())
}
