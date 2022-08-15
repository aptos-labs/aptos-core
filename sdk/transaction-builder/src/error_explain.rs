// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A module for looking up the human-readable explanation of a Aptos Move
//! transaction abort code.
//!
//! Note that the ~13 KiB error descriptions will be inlined into the final binary.

use move_deps::move_core_types::errmap::ErrorDescription;
use move_deps::move_core_types::language_storage::ModuleId;

/// Given the module ID and the abort code raised from that module, returns the
/// human-readable explanation of that abort if possible.
pub fn get_explanation(module_id: &ModuleId, abort_code: u64) -> Option<ErrorDescription> {
    framework::head_release_bundle()
        .error_mapping()
        .get_explanation(module_id, abort_code & 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_deps::move_core_types::{account_address::AccountAddress, ident_str};

    #[test]
    fn test_get_explanation() {
        let module_id = ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned());
        // If this breaks, well then someone adjusted the error codes...
        let no_category = get_explanation(&module_id, 4).unwrap();
        let category = get_explanation(&module_id, 65540).unwrap();
        // bcs because ErrorDescription doesn't support `==`
        assert_eq!(
            bcs::to_bytes(&no_category).unwrap(),
            bcs::to_bytes(&category).unwrap()
        );
    }
}
