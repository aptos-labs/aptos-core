// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains names of built-in functions and types.

use move_symbol_pool::Symbol;
use once_cell::sync::Lazy;
use std::collections::BTreeSet;

static BUILTIN_FUNCTION_NAMES: Lazy<BTreeSet<Symbol>> = Lazy::new(|| {
    [
        "move_to",
        "move_from",
        "borrow_global",
        "borrow_global_mut",
        "exists",
        "freeze",
        "assert",
    ]
    .iter()
    .map(|n| Symbol::from(*n))
    .collect()
});

/// Built in type names.
pub const ADDRESS: &str = "address";
pub const BOOL: &str = "bool";
pub const SIGNER: &str = "signer";
pub const U_128: &str = "u128";
pub const U_16: &str = "u16";
pub const U_256: &str = "u256";
pub const U_32: &str = "u32";
pub const U_64: &str = "u64";
pub const U_8: &str = "u8";
pub const VECTOR: &str = "vector";

static BUILTIN_TYPE_NAMES: Lazy<BTreeSet<Symbol>> = Lazy::new(|| {
    [
        ADDRESS, SIGNER, U_8, U_16, U_32, U_64, U_128, U_256, BOOL, VECTOR,
    ]
    .iter()
    .map(|n| Symbol::from(*n))
    .collect()
});

/// Built in function names.
pub fn all_function_names() -> &'static BTreeSet<Symbol> {
    &BUILTIN_FUNCTION_NAMES
}

/// All the built-in type names.
pub fn all_type_names() -> &'static BTreeSet<Symbol> {
    &BUILTIN_TYPE_NAMES
}
