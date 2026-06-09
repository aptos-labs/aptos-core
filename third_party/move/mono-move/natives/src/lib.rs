// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function implementations shipped with MonoMove. Each native is a
//! plain function generic over any [`NativeContext`] impl.
//!
//! [`NativeContext`]: mono_move_core::native::NativeContext

// Re-exported so the `natives!` macro can name it via `$crate::NativeFunction`
// without callers having to add `mono-move-core` to their imports.
use mono_move_core::native::NativeContextFamily;
pub use mono_move_core::native::NativeFunction;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

pub mod signer;
pub mod test_natives;

pub use signer::make_all_signer_natives;
pub use test_natives::{make_all_test_natives, native_u64_add, native_u64_identity};

/// All natives shipped with the production MonoMove VM. Additional native
/// modules are concatenated here as they are implemented.
pub fn make_all_production_natives<F: NativeContextFamily>(
) -> Vec<(AccountAddress, Identifier, Identifier, NativeFunction<F>)> {
    make_all_signer_natives::<F>()
}

/// Parses a fully-qualified function name (e.g. "0x1::natives::u64_add")
/// into its component parts. Panics on malformed input.
//
// TODO: replace with a proper parser. See if one already exists in
// move-core-types.
pub(crate) fn parse_qualified_native_name(qname: &str) -> (AccountAddress, Identifier, Identifier) {
    let mut parts = qname.split("::");
    let addr_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing address", qname));
    let module_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing module", qname));
    let function_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing function", qname));
    assert!(
        parts.next().is_none(),
        "malformed native name `{}`: too many `::`-separated parts",
        qname,
    );
    let addr = AccountAddress::from_hex_literal(addr_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid address: {}", qname, e));
    let module = Identifier::new(module_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid module: {}", qname, e));
    let function = Identifier::new(function_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid function: {}", qname, e));
    (addr, module, function)
}

/// Builds a list of natives from human-readable qualified names and Rust functions.
///
/// Example:
/// ```ignore
/// let natives = natives![
///     ("0x1::test_natives::u64_add", native_u64_add),
///     ("0x1::test_natives::u64_identity", native_u64_identity),
/// ];
/// ```
macro_rules! natives {
    [ $( ( $qname:expr , $func:expr ) ),* $(,)? ] => {
        ::std::vec![
            $({
                let (addr, module, function) =
                    $crate::parse_qualified_native_name($qname);
                let func: $crate::NativeFunction<_> =
                    ::std::boxed::Box::new(|ctx| $func(ctx));
                (addr, module, function, func)
            }),*
        ]
    };
}
pub(crate) use natives;
