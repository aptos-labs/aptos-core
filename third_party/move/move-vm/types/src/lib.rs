// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

/// Returns the hash (SHA-3-256) of the bytes.
pub fn sha3_256(bytes: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};

    let mut sha3_256 = Sha3_256::new();
    sha3_256.update(bytes);
    sha3_256.finalize().into()
}

#[macro_export]
macro_rules! debug_write {
    ($($toks: tt)*) => {
        write!($($toks)*).map_err(|_|
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("failed to write to buffer".to_string())
        )
    };
}

#[macro_export]
macro_rules! debug_writeln {
    ($($toks: tt)*) => {
        writeln!($($toks)*).map_err(|_|
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("failed to write to buffer".to_string())
        )
    };
}

pub mod code;
pub mod delayed_values;
pub mod gas;
pub mod loaded_data;
pub mod module_traversal;
pub mod natives;
pub mod resolver;
pub mod value_serde;
pub mod value_traversal;
pub mod values;
pub mod views;

#[cfg(test)]
mod unit_tests;
