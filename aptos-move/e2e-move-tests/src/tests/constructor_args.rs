// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    parser::parse_struct_tag,
    vm_status::{AbortLocation, StatusCode},
};
use move_model::metadata::LanguageVersion;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

const OBJECT_ADDRESS: AccountAddress = AccountAddress::new([
    0x66, 0x2E, 0x50, 0x41, 0x8C, 0xE5, 0xF3, 0x5A, 0x6C, 0xA8, 0xB7, 0x9E, 0x28, 0x7C, 0x94, 0x12,
    0x90, 0x71, 0xAA, 0x3F, 0xBD, 0x2A, 0xB9, 0x51, 0x37, 0xF7, 0xCB, 0xAD, 0x13, 0x6F, 0x09, 0x2B,
]);

fn module_data() -> StructTag {
    parse_struct_tag("0xCAFE::test::ModuleData").unwrap()
}

fn success(h: &mut MoveHarness, tests: Vec<(&str, Vec<Vec<u8>>, &str)>) {
    success_generic(h, vec![], tests)
}

fn success_generic(
    h: &mut MoveHarness,
    ty_args: Vec<TypeTag>,
    tests: Vec<(&str, Vec<Vec<u8>>, &str)>,
) {
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Use language version 2.3 for consistency.
    let options = BuildOptions {
        language_version: Some(LanguageVersion::V2_3),
        ..BuildOptions::move_2()
    };
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("constructor_args.data/pack"),
        options
    ));

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data()));

    for (entry, args, expected_change) in tests {
        assert_success!(h.run_entry_function(
            &acc,
            str::parse(entry).unwrap(),
            ty_args.clone(),
            args,
        ));
        assert_eq!(
            String::from_utf8(
                h.read_resource::<ModuleData>(&OBJECT_ADDRESS, module_data())
                    .unwrap()
                    .state
            )
            .unwrap(),
            expected_change,
        );
    }
}

fn success_generic_view(
    h: &mut MoveHarness,
    ty_args: Vec<TypeTag>,
    tests: Vec<(&str, Vec<Vec<u8>>, &str)>,
) {
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Use language version 2.3 for consistency.
    let options = BuildOptions {
        language_version: Some(LanguageVersion::V2_3),
        ..BuildOptions::move_2()
    };
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("constructor_args.data/pack"),
        options
    ));

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data()));

    for (entry, args, expected) in tests {
        let res = h.execute_view_function(str::parse(entry).unwrap(), ty_args.clone(), args);
        assert!(
            res.values.is_ok(),
            "{}",
            res.values.err().unwrap().to_string()
        );
        let bcs = res.values.unwrap().pop().unwrap();
        let res = bcs::from_bytes::<String>(&bcs).unwrap();
        assert_eq!(res, expected);
    }
}

type Closure = Box<dyn FnOnce(TransactionStatus) -> bool>;

fn fail(tests: Vec<(&str, Vec<Vec<u8>>, Closure)>) {
    fail_generic(vec![], tests)
}

fn fail_generic(ty_args: Vec<TypeTag>, tests: Vec<(&str, Vec<Vec<u8>>, Closure)>) {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Use language version 2.3 for consistency. These tests verify runtime validation
    // of invalid transaction arguments.
    let options = BuildOptions {
        language_version: Some(LanguageVersion::V2_3),
        ..BuildOptions::move_2()
    };
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("constructor_args.data/pack"),
        options
    ));

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data()));

    for (entry, args, check) in tests {
        assert!(check(h.run_entry_function(
            &acc,
            str::parse(entry).unwrap(),
            ty_args.clone(),
            args,
        )));
    }
}

#[test]
fn constructor_args_good() {
    let tests = vec![
        // ensure object exist
        ("0xcafe::test::initialize", vec![], ""),
        // None is a valid value for Option<MyPrecious> even though MyPrecious is private.
        // Some(MyPrecious) would be rejected with INVALID_MAIN_FUNCTION_SIGNATURE at runtime.
        (
            "0xcafe::test::ensure_no_fabrication",
            vec![
                bcs::to_bytes(&Vec::<u64>::new()).unwrap(), // Option<MyPrecious> = None
            ],
            "", // state unchanged
        ),
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),
            ],
            "hi",
        ),
        (
            "0xcafe::test::pass_optional_fixedpoint32",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),     // Object<T>
                bcs::to_bytes(&vec![(1u64 << 32)]).unwrap(), // Option<FixedPoint32>
            ],
            "4294967296",
        ),
        (
            "0xcafe::test::pass_optional_vector_fixedpoint64",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![(1u128 << 64), (2u128 << 64)]]).unwrap(), // Option<vector<FixedPoint64>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "36893488147419103232", // 2 in fixedpoint64
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec!["a"], vec!["b"]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "b", // second element of the vector
        ),
        (
            "0xcafe::test::pass_vector_optional_object",
            vec![
                bcs::to_bytes(&vec![vec![OBJECT_ADDRESS], vec![]]).unwrap(), // vector<Option<Object<T>>>
                bcs::to_bytes(&"pff vectors of optionals").unwrap(),
                bcs::to_bytes(&0u64).unwrap(),
            ],
            "pff vectors of optionals",
        ),
        (
            "0xcafe::test::ensure_vector_vector_u8",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![1u8], vec![2u8]]).unwrap(), // vector<vector<u8>>
            ],
            "vector<vector<u8>>",
        ),
    ];

    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    success(&mut h, tests);
}

#[test]
fn view_constructor_args() {
    let tests = vec![
        // ensure object exist
        ("0xcafe::test::initialize", vec![], ""),
        // make state equal hi
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),
            ],
            "hi",
        ),
    ];

    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    success(&mut h, tests);

    let view = vec![(
        "0xcafe::test::get_state",
        vec![bcs::to_bytes(&OBJECT_ADDRESS).unwrap()],
        "hi",
    )];
    let module_data_type = TypeTag::Struct(Box::new(module_data()));
    success_generic_view(&mut h, vec![module_data_type], view);
}

#[test]
fn constructor_args_option_private_struct_compiles() {
    // Test that with language version 2.4+, the package compiles successfully.
    // Option<MyPrecious> is allowed in entry function parameters:
    //   - None is a valid value and succeeds at runtime (see constructor_args_good).
    //   - Some(MyPrecious) is rejected at runtime with INVALID_MAIN_FUNCTION_SIGNATURE
    //     because MyPrecious has no public constructor (pack function).
    let result = BuiltPackage::build(
        common::test_dir_path("constructor_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    );

    assert!(
        result.is_ok(),
        "Expected compilation to succeed with language version 2.4+, but it failed: {:?}",
        result.err()
    );
}

#[test]
fn constructor_args_bad_runtime() {
    // Test runtime validation with language version 2.3
    let good: &[u8] = "a".as_bytes();
    let bad: &[u8] = &[0x80u8; 1];

    let tests: Vec<(&str, Vec<Vec<u8>>, Closure)> = vec![
        // object doesnt exist
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
                )
            }),
        ),
        // Initialize the object so the next test can reach argument deserialization.
        // Without this, Object<T> construction aborts with EOBJECT_DOES_NOT_EXIST
        // before the invalid UTF-8 bytes in arg[1] are ever validated.
        (
            "0xcafe::test::initialize",
            vec![],
            Box::new(|e| matches!(e, TransactionStatus::Keep(ExecutionStatus::Success))),
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec![good], vec![bad]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            Box::new(|e| {
                // Invalid UTF-8 is caught inside the Move string module at runtime
                // (0x1::string::EINVALID_UTF8 = 1).
                if let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location, code, ..
                }) = e
                {
                    code == 1
                        && matches!(
                            location,
                            AbortLocation::Module(m)
                                if m.address() == &AccountAddress::ONE
                                    && m.name().as_str() == "string"
                        )
                } else {
                    false
                }
            }),
        ),
        (
            "0xcafe::test::ensure_no_fabrication",
            vec![
                bcs::to_bytes(&vec![1u64]).unwrap(), // Option<MyPrecious> with Some value
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                        StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE
                    )))
                )
            }),
        ),
    ];

    fail(tests);
}
