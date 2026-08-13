// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that `std::option::Option`, declared as a public enum, can be used directly
//! from modules outside `std`: constructing `Some`/`None`, testing variants with `is`,
//! matching, and selecting/mutating the variant field.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::ModuleId,
    vm_status::{AbortLocation, StatusCode},
};

const PUBLIC_OPTION_TEST_SOURCE: &str = r#"
/// Exercises `std::option::Option` as a public enum from a module outside `std`.
module 0xCAFE::public_option_test {
    use std::option::{Self, Option};

    /// A local struct to exercise Option over user-defined element types.
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    /// Directly constructed options are interchangeable with the std::option API.
    public entry fun construct_and_compare() {
        let s = Option::Some { e: 42u64 };
        assert!(s == option::some(42u64), 0);

        let n: Option<u64> = Option::None;
        assert!(n == option::none(), 1);
        assert!(s != n, 2);
    }

    /// The `is` operator works on Option outside the defining module.
    public entry fun test_is_operator() {
        let s = option::some(7u8);
        assert!(s is Option::Some, 0);
        assert!(!(s is Option::None), 1);

        let n = option::none<u8>();
        assert!(n is Option::None, 2);
        assert!(!(n is Option::Some), 3);
    }

    fun unwrap_or(o: Option<u64>, default: u64): u64 {
        match (o) {
            Option::Some { e } => e,
            Option::None => default,
        }
    }

    /// `match` over Option, by value and by reference, works outside the defining module.
    public entry fun test_match() {
        assert!(unwrap_or(option::some(10), 99) == 10, 0);
        assert!(unwrap_or(option::none(), 99) == 99, 1);

        let o = option::some(3u64);
        let doubled = match (&o) {
            Option::Some { e } => *e * 2,
            Option::None => 0,
        };
        assert!(doubled == 6, 2);
    }

    /// The variant field can be selected and mutated outside the defining module.
    public entry fun test_field_access() {
        let s = option::some(5u64);
        assert!(s.e == 5, 0);

        s.e = 6;
        assert!(s == option::some(6u64), 1);
    }

    /// Selecting the variant field on a None value must fail at runtime
    /// with STRUCT_VARIANT_MISMATCH.
    public entry fun test_field_read_on_none() {
        let n = option::none<u64>();
        let _x = n.e;
    }

    /// Writing the variant field on a None value must fail at runtime
    /// with STRUCT_VARIANT_MISMATCH.
    public entry fun test_field_write_on_none() {
        let n = option::none<u64>();
        n.e = 5;
    }

    /// Directly constructed options over user-defined types flow through the
    /// regular std::option API.
    public entry fun test_std_interop() {
        let s = Option::Some { e: Point { x: 1, y: 2 } };
        assert!(option::is_some(&s), 0);

        let p = option::extract(&mut s);
        assert!(p.x == 1 && p.y == 2, 1);
        assert!(s is Option::None, 2);

        let n: Option<Point> = Option::None;
        assert!(option::is_none(&n), 3);
        option::fill(&mut n, p);
        assert!(n.e == Point { x: 1, y: 2 }, 4);
    }
}
"#;

#[test]
fn test_public_option_cross_module() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut builder = PackageBuilder::new("PublicOption");
    builder.add_source("public_option_test", PUBLIC_OPTION_TEST_SOURCE);
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    // Language version 2.4+ is required for cross-module access to public enums.
    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    ));

    for entry in [
        "construct_and_compare",
        "test_is_operator",
        "test_match",
        "test_field_access",
        "test_std_interop",
    ] {
        let status = h.run_entry_function(
            &acc,
            format!("0xcafe::public_option_test::{}", entry)
                .parse()
                .unwrap(),
            vec![],
            vec![],
        );
        assert_success!(status, "entry function {} failed", entry);
    }

    // Unhappy path: accessing the `Some` field of a `None` value is caught by the VM's
    // variant check at runtime (STRUCT_VARIANT_MISMATCH). Cross-module field access
    // compiles into a call to a generated accessor inside std::option, so the resulting
    // ExecutionFailure is located in 0x1::option rather than the calling module.
    let option_module = ModuleId::new(AccountAddress::ONE, Identifier::new("option").unwrap());
    for entry in ["test_field_read_on_none", "test_field_write_on_none"] {
        let status = h.run_entry_function(
            &acc,
            format!("0xcafe::public_option_test::{}", entry)
                .parse()
                .unwrap(),
            vec![],
            vec![],
        );
        match status {
            TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
                location: AbortLocation::Module(module),
                ..
            }) => {
                assert_eq!(
                    module, option_module,
                    "entry function {} failed in an unexpected module",
                    entry
                );
            },
            other => panic!(
                "entry function {} should fail with ExecutionFailure in 0x1::option, got {:?}",
                entry, other
            ),
        }

        // The kept transaction status drops the status code, so also run the function
        // directly through the VM to verify STRUCT_VARIANT_MISMATCH is what is raised.
        let err = h
            .exec_function_bypass_visibility(
                AccountAddress::from_hex_literal("0xcafe").unwrap(),
                "public_option_test",
                entry,
                vec![],
                vec![],
            )
            .unwrap_err();
        assert_eq!(
            err.status_code(),
            StatusCode::STRUCT_VARIANT_MISMATCH,
            "entry function {} raised unexpected status: {:?}",
            entry,
            err
        );
    }
}
