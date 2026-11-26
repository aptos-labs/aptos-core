// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Error codes, name parsing, and the forbidden-target list of the `std::reflect` natives.

use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
};

/// Code of `std::reflect::ReflectionError::InvalidIdentifier`. The remaining
/// codes are the `FunctionResolutionError` discriminants.
pub const INVALID_IDENTIFIER: u16 = 0;

/// Parses a reflected module or function name. Returns [None] if `bytes` is longer than
/// `max_len` or is not a valid identifier. The length is checked before the bytes are read,
/// and nothing is allocated unless the name is valid.
pub fn parse_name(bytes: &[u8], max_len: u64) -> Option<Identifier> {
    if bytes.len() as u64 > max_len {
        return None;
    }
    let name = std::str::from_utf8(bytes).ok()?;
    Identifier::is_valid(name).then(|| Identifier::new_unchecked(name))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name_enforces_max_len() {
        let at_limit = vec![b'a'; 255];
        assert_eq!(
            parse_name(&at_limit, 255).map(Identifier::into_bytes),
            Some(at_limit.clone())
        );

        let over_limit = vec![b'a'; 256];
        assert_eq!(parse_name(&over_limit, 255), None);
        assert!(parse_name(&over_limit, u64::MAX).is_some());
    }

    #[test]
    fn parse_name_matches_identifier_from_utf8_without_limit() {
        let inputs: [&[u8]; 10] = [
            b"",
            b"a",
            b"foo_bar$1",
            b"_",
            b"_x",
            b"$x",
            b"1a",
            b"a b",
            b"<SELF>",
            &[0xFF, 0xFE],
        ];
        for bytes in inputs {
            assert_eq!(
                parse_name(bytes, u64::MAX),
                Identifier::from_utf8(bytes.to_vec()).ok(),
                "input: {bytes:?}"
            );
        }
    }
}
