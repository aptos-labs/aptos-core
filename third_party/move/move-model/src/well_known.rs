// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Names of well-known functions or attributes.
//!
//! This currently only contains those declarations used somewhere, not all well-known
//! declarations. It can be extended on the go.

/// Function identifying the name of an attribute which declares an
/// item to be part of test.
pub fn is_test_only_attribute_name(s: &str) -> bool {
    s == "test" || s == "test_only"
}

/// Function identifying the name of an attribute which declares an
/// item to be a test.
pub fn is_test_attribute_name(s: &str) -> bool {
    s == "test"
}

/// Function identifying the name of an attribute which declares an
/// item to be part of verification only.
pub fn is_verify_only_attribute_name(s: &str) -> bool {
    s == "verify_only"
}

pub const VECTOR_MODULE: &str = "vector";
pub const VECTOR_BORROW_MUT: &str = "vector::borrow_mut";
pub const EVENT_EMIT_EVENT: &str = "event::emit_event";
pub const BORROW_NAME: &str = "borrow";
pub const BORROW_MUT_NAME: &str = "borrow_mut";
/// Functions in the std::vector module that are implemented as bytecode instructions.
pub const VECTOR_FUNCS_WITH_BYTECODE_INSTRS: &[&str] = &[
    "empty",
    "length",
    "borrow",
    "borrow_mut",
    "push_back",
    "pop_back",
    "destroy_empty",
    "swap",
];

pub const CMP_MODULE: &str = "cmp";

pub const TYPE_NAME_MOVE: &str = "type_info::type_name";
pub const TYPE_NAME_SPEC: &str = "type_info::$type_name";
pub const TYPE_INFO_MOVE: &str = "type_info::type_of";
pub const TYPE_INFO_SPEC: &str = "type_info::$type_of";
pub const TYPE_SPEC_IS_STRUCT: &str = "type_info::spec_is_struct";

/// NOTE: `type_info::type_name` and `type_name::get` are very similar.
/// The main difference (from a prover's perspective) include:
/// - formatting of an address (part of the struct name), and
/// - whether it is in `stdlib` or `extlib`.
pub const TYPE_NAME_GET_MOVE: &str = "type_name::get";
pub const TYPE_NAME_GET_SPEC: &str = "type_name::$get";

/// The well-known name of the first parameter of a method.
pub const RECEIVER_PARAM_NAME: &str = "self";

/// The well-known abort codes used by the compiler. These conform
/// to the error category standard as defined in
/// `../move-stdlib/sources/error.move` in the standard library. The lowest
/// three bytes represent the error category (one byte) and the reason (two bytes).
/// All compiler generated abort codes use category
/// `std::error::INTERNAL` (`0xB`). The upper five bytes
/// are populated with the lowest bytes of the sha256
/// of the string "Move 2 Abort Code".
const fn make_abort_code(reason: u16) -> u64 {
    let magic = 0xCA26CBD9BE; // sha256("Move 2 Abort code")
    (magic << 24) | (0xB << 16) | (reason as u64)
}

// Used when user omits an abort code in an `assert!`.
pub const UNSPECIFIED_ABORT_CODE: u64 = make_abort_code(0);

// Used when a runtime value falls through a match.
pub const INCOMPLETE_MATCH_ABORT_CODE: u64 = make_abort_code(1);

// Well known attributes
pub const PERSISTENT_ATTRIBUTE: &str = "persistent";
pub const MODULE_LOCK_ATTRIBUTE: &str = "module_lock";
