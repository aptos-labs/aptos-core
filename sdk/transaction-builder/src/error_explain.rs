// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A module for looking up the human-readable explanation of a Aptos Move
//! transaction abort code.
//!
//! Note that the ~13 KiB error descriptions will be inlined into the final binary.

use move_deps::move_core_types::{
    errmap::{ErrorContext, ErrorMapping},
    language_storage::ModuleId,
};
use once_cell::sync::Lazy;

static RELEASE_ERRMAP: Lazy<ErrorMapping> = Lazy::new(|| {
    let error_map = cached_framework_packages::error_map();
    bcs::from_bytes(&*error_map).expect("Failed to deserialize static error descriptions")
});

/// Given the module ID and the abort code raised from that module, returns the
/// human-readable explanation of that abort if possible.
pub fn get_explanation(module_id: &ModuleId, abort_code: u64) -> Option<ErrorContext> {
    let errmap = &*RELEASE_ERRMAP;
    errmap.get_explanation(module_id, abort_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_deps::move_core_types::{account_address::AccountAddress, ident_str};

    #[test]
    fn test_get_explanation() {
        let module_id = ModuleId::new(AccountAddress::ZERO, ident_str!("TESTTEST").to_owned());
        // We don't care about the result, just that the errmap deserializes without panicking.
        let _ = get_explanation(&module_id, 1234);
    }
}
