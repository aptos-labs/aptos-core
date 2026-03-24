// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Bundled Move standard library sources for browser-native compilation.
//!
//! These sources are embedded at compile time so the WASM compiler can resolve
//! `std::*` imports without filesystem access.

use move_compiler_v2::sources::SourceMap;
use move_core_types::account_address::AccountAddress;

/// All move-stdlib source files, embedded at compile time.
const STDLIB_SOURCES: &[(&str, &str)] = &[
    ("signer.move", include_str!("../../framework/move-stdlib/sources/signer.move")),
    ("mem.move", include_str!("../../framework/move-stdlib/sources/mem.move")),
    ("vector.move", include_str!("../../framework/move-stdlib/sources/vector.move")),
    ("error.move", include_str!("../../framework/move-stdlib/sources/error.move")),
    ("option.move", include_str!("../../framework/move-stdlib/sources/option.move")),
    ("string.move", include_str!("../../framework/move-stdlib/sources/string.move")),
    ("bcs.move", include_str!("../../framework/move-stdlib/sources/bcs.move")),
    ("hash.move", include_str!("../../framework/move-stdlib/sources/hash.move")),
    ("bit_vector.move", include_str!("../../framework/move-stdlib/sources/bit_vector.move")),
    ("fixed_point32.move", include_str!("../../framework/move-stdlib/sources/fixed_point32.move")),
    ("acl.move", include_str!("../../framework/move-stdlib/sources/acl.move")),
    ("cmp.move", include_str!("../../framework/move-stdlib/sources/cmp.move")),
    ("result.move", include_str!("../../framework/move-stdlib/sources/result.move")),
    ("reflect.move", include_str!("../../framework/move-stdlib/sources/reflect.move")),
    ("configs/features.move", include_str!("../../framework/move-stdlib/sources/configs/features.move")),
];

/// Build a SourceMap containing all bundled move-stdlib sources.
pub fn stdlib_source_map() -> SourceMap {
    let mut deps = SourceMap::new();
    for (filename, content) in STDLIB_SOURCES {
        deps.add_file(*filename, *content);
    }
    deps
}

/// Well-known Aptos named address mappings.
///
/// These are the standard addresses used by the Aptos framework:
/// - `std` → 0x1 (Move standard library)
/// - `aptos_std` → 0x1 (Aptos standard library)
/// - `aptos_framework` → 0x1 (Aptos framework)
/// - `aptos_token_objects` → 0x4 (Token objects)
pub fn well_known_addresses() -> Vec<(String, AccountAddress)> {
    vec![
        ("std".to_string(), AccountAddress::ONE),
        ("aptos_std".to_string(), AccountAddress::ONE),
        ("aptos_framework".to_string(), AccountAddress::ONE),
        ("aptos_token_objects".to_string(), AccountAddress::from_hex_literal("0x4").unwrap()),
    ]
}
