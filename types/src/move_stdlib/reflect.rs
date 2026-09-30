// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Error codes and the forbidden-target list of the `std::reflect` natives.

use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Code of `std::reflect::ReflectionError::InvalidIdentifier`. The remaining
/// codes are the `FunctionResolutionError` discriminants.
pub const INVALID_IDENTIFIER: u16 = 0;

/// Functions reflection refuses to resolve, as `(module, function)` pairs at
/// the framework address `0x1`. A function is forbidden when the bytecode
/// verifier enforces its call-site rules, which a dynamically resolved
/// function value cannot uphold.
pub const FORBIDDEN_FRAMEWORK_FUNCTIONS: &[(&str, &str)] =
    &[("event", "emit"), ("init", "internal_maybe_initialize")];

/// Whether reflection refuses to resolve `address::module_name::func_name`.
pub fn is_forbidden_to_reflect(
    address: &AccountAddress,
    module_name: &IdentStr,
    func_name: &IdentStr,
) -> bool {
    address == &AccountAddress::ONE
        && FORBIDDEN_FRAMEWORK_FUNCTIONS
            .iter()
            .any(|&(module, function)| {
                module_name.as_str() == module && func_name.as_str() == function
            })
}
