// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    deserializer::DeserializerConfig,
    errors::BinaryLoaderResult,
    file_format::{basic_test_module, empty_script, CompiledModule, CompiledScript},
    file_format_common::*,
};
use move_core_types::vm_status::StatusCode;
use std::fmt::Debug;

fn malformed_simple_versioned_test(version: u32) {
    // bad uleb (more than allowed for table count)
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(150); // table count (high bit 1)
    binary.push(150); // table count (high bit 1)
    binary.push(1);
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad uleb").major_status(),
        StatusCode::MALFORMED
    );

    // bad uleb (too big)
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(150); // table count (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(150); // table count again (high bit 1)
    binary.push(0); // table count again
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad uleb").major_status(),
        StatusCode::MALFORMED
    );

    // no tables
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(0); // table count
    let res = CompiledModule::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected no table count").major_status(),
        StatusCode::MALFORMED
    );

    // missing tables
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(10); // table count
    let res = CompiledModule::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected no table header").major_status(),
        StatusCode::MALFORMED
    );

    // missing table content
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(1); // table count
    binary.push(1); // table type
    binary.push(0); // table offset
    binary.push(10); // table length
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected no table content").major_status(),
        StatusCode::MALFORMED
    );

    // bad table header (bad offset)
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(1); // table count
    binary.push(1); // table type
    binary.push(100); // bad table offset
    binary.push(10); // table length
    let res = CompiledModule::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad table offset").major_status(),
        StatusCode::BAD_HEADER_TABLE
    );

    // bad table header (bad offset)
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(2); // table count
    binary.push(1); // table type
    binary.push(0); // table offset
    binary.push(10); // table length
    binary.push(2); // table type
    binary.push(100); // bad table offset
    binary.push(10); // table length
    binary.resize(binary.len() + 5000, 0);
    let res = CompiledModule::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad table offset").major_status(),
        StatusCode::BAD_HEADER_TABLE
    );

    // incomplete table
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(1); // table count
    binary.push(1); // table type
    binary.push(0); // table offset
    binary.push(10); // table length
    binary.resize(binary.len() + 5, 0);
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad table content").major_status(),
        StatusCode::MALFORMED
    );

    // unknown table
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(1); // table count
    binary.push(100); // table type
    binary.push(0); // table offset
    binary.push(10); // table length
    binary.resize(binary.len() + 10, 0);
    let res = CompiledModule::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected unknown table").major_status(),
        StatusCode::UNKNOWN_TABLE_TYPE
    );

    // duplicate table
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(3); // table count
    binary.push(1); // table type
    binary.push(0); // table offset
    binary.push(10); // table length
    binary.push(2); // table type
    binary.push(10); // table offset
    binary.push(10); // table length
    binary.push(1); // table type
    binary.push(20); // table offset
    binary.push(10); // table length
    binary.resize(binary.len() + 5000, 0);
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected table offset overflow")
            .major_status(),
        StatusCode::DUPLICATE_TABLE
    );

    // bad table in script
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend(version.to_le_bytes()); // version
    binary.push(1); // table count
    binary.push(0xD); // table type - FieldHandle not good for script
    binary.push(0); // table offset
    binary.push(10); // table length
    binary.resize(binary.len() + 5000, 0);
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected table offset overflow")
            .major_status(),
        StatusCode::MALFORMED
    );
}

#[test]
#[allow(clippy::same_item_push)]
fn malformed_simple() {
    // empty binary
    let binary = vec![];
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected malformed binary").major_status(),
        StatusCode::BAD_MAGIC
    );

    // under-sized binary
    let binary = vec![0u8, 0u8, 0u8];
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected malformed binary").major_status(),
        StatusCode::BAD_MAGIC
    );

    // bad magic
    let binary = vec![0u8; 4];
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected bad magic").major_status(),
        StatusCode::BAD_MAGIC
    );

    // only magic
    let binary = BinaryConstants::MOVE_MAGIC.to_vec();
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected malformed binary").major_status(),
        StatusCode::MALFORMED
    );

    // bad version
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend((VERSION_MAX.checked_add(1).unwrap()).to_le_bytes()); // version
    binary.push(10); // table count
    binary.push(0); // rest of binary
    let res = CompiledScript::deserialize(&binary);
    assert_eq!(
        res.expect_err("Expected unknown version").major_status(),
        StatusCode::UNKNOWN_VERSION
    );

    // versioned tests
    for version in VERSION_1..VERSION_MAX {
        malformed_simple_versioned_test(version);
    }
}

#[test]
fn max_version_lower_than_hardcoded() {
    let mut binary = BinaryConstants::MOVE_MAGIC.to_vec();
    binary.extend((VERSION_MAX).to_le_bytes()); // version
    binary.push(10); // table count
    binary.push(0); // rest of binary

    let config = DeserializerConfig::new(
        VERSION_MAX.checked_sub(1).unwrap(),
        LEGACY_IDENTIFIER_SIZE_MAX,
    );
    let res = CompiledScript::deserialize_with_config(&binary, &config);
    assert_eq!(
        res.expect_err("Expected unknown version").major_status(),
        StatusCode::UNKNOWN_VERSION
    );
}

// Ensure that we can deserialize a script from disk
static EMPTY_SCRIPT: &[u8] = include_bytes!("empty_script.mv");

#[test]
fn deserialize_file() {
    CompiledScript::deserialize(EMPTY_SCRIPT).expect("script should deserialize properly");
}

// An invalid script that is missing a signature should not deserialize successfully.
static INVALID_SCRIPT_NO_SIGNATURE: &[u8] = include_bytes!("invalid_script_no_signature.mv");

#[test]
fn deserialize_invalid_script_no_signature() {
    assert_eq!(
        CompiledScript::deserialize(INVALID_SCRIPT_NO_SIGNATURE)
            .unwrap_err()
            .major_status(),
        StatusCode::INDEX_OUT_OF_BOUNDS
    );
}

// A binary with long identifiers should not deserialize successfully.
static TOO_LONG_IDENTIFIER: &[u8] = include_bytes!("tool_long_identifier.mv");

#[test]
fn deserialize_too_long_identifiers() {
    // The deserialization with the legacy limit succeeds.
    let legacy_config = DeserializerConfig::new(VERSION_MAX, LEGACY_IDENTIFIER_SIZE_MAX);
    assert!(CompiledModule::deserialize_with_config(TOO_LONG_IDENTIFIER, &legacy_config).is_ok());

    // The deserialization with the reduced limit should fail.
    let config = DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX);
    assert_eq!(
        CompiledModule::deserialize_with_config(TOO_LONG_IDENTIFIER, &config)
            .unwrap_err()
            .major_status(),
        StatusCode::MALFORMED
    );
}

/// Checks that `binary`, the serialization of `value`, deserializes to `value` in both modes, and
/// that with trailing bytes appended it only does so in lenient mode.
fn check_trailing_bytes<T: PartialEq + Debug>(
    value: &T,
    binary: &[u8],
    deserialize: fn(&[u8], &DeserializerConfig) -> BinaryLoaderResult<T>,
) {
    let lenient = DeserializerConfig::default();
    let strict = DeserializerConfig {
        reject_trailing_bytes: true,
        ..lenient.clone()
    };

    for config in [&lenient, &strict] {
        assert_eq!(&deserialize(binary, config).unwrap(), value);
    }

    for trailing_bytes in [&[0u8][..], b"payload"] {
        let binary = [binary, trailing_bytes].concat();
        assert_eq!(&deserialize(&binary, &lenient).unwrap(), value);
        assert_eq!(
            deserialize(&binary, &strict).unwrap_err().major_status(),
            StatusCode::MALFORMED
        );
    }
}

#[test]
fn deserialize_module_with_trailing_bytes() {
    let module = basic_test_module();
    let mut binary = vec![];
    module
        .serialize_for_version(Some(module.version), &mut binary)
        .unwrap();
    check_trailing_bytes(&module, &binary, CompiledModule::deserialize_with_config);
}

#[test]
fn deserialize_script_with_trailing_bytes() {
    let script = empty_script();
    let mut binary = vec![];
    script
        .serialize_for_version(Some(script.version), &mut binary)
        .unwrap();
    check_trailing_bytes(&script, &binary, CompiledScript::deserialize_with_config);
}
