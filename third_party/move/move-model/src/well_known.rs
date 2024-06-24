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

/// The well-known abort codes used by the compiler. These are marked
/// by lowest 6 bytes of a sha256 of the string "Move 2 Abort Code",
/// appended with two bytes for the error type.
/// TODO: add a check at runtime that user is not clashing with reserved
/// codes?
pub const WELL_KNOWN_ABORT_CODE_BASE: u64 = 0xD8CA26CBD9BE << 16;
pub const INCOMPLETE_MATCH_ABORT_CODE: u64 = WELL_KNOWN_ABORT_CODE_BASE | 0x1;
