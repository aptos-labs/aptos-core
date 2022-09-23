// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{assert_success, assert_vm_status, MoveHarness};
use move_deps::move_core_types::{parser::parse_struct_tag, vm_status::StatusCode};
use serde::{Deserialize, Serialize};

mod common;

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[test]
fn string_args() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    // Now send hi transaction, after that resource should exist and carry value
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi").unwrap(),
        vec![],
        vec![bcs::to_bytes("hi there!".as_bytes()).unwrap()],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data)
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
}

#[test]
fn string_args_vec() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    let s_vec = vec![
        "hi there!".as_bytes(),
        "hello".as_bytes(),
        "world".as_bytes(),
    ];
    // Now send hi_vec transaction, after that resource should exist and carry value
    let mut i = 0u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi_vec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data.clone())
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
    // Now send hi_vec transaction, after that resource should exist and carry value
    i = 1u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi_vec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data.clone())
                .unwrap()
                .state
        )
        .unwrap(),
        "hello"
    );
    // Now send hi_vec transaction, after that resource should exist and carry value
    i = 2u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi_vec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data)
                .unwrap()
                .state
        )
        .unwrap(),
        "world"
    );
}

#[test]
fn string_args_vec_vec() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    let s_vec = vec![
        vec![
            "hi there!".as_bytes(),
            "hello".as_bytes(),
            "world".as_bytes(),
        ],
        vec![
            "hello".as_bytes(),
            "world".as_bytes(),
            "hi there!".as_bytes(),
        ],
        vec![
            "world".as_bytes(),
            "hi there!".as_bytes(),
            "hello".as_bytes(),
        ],
    ];
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let j = 0u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap()
        ],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data.clone())
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 1u64;
    let j = 2u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap()
        ],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data.clone())
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 2u64;
    let j = 1u64;
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap()
        ],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data)
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
}

#[test]
fn string_args_bad_1() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    // Now send hi transaction, after that resource should exist and carry value
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi").unwrap(),
        vec![],
        vec![bcs::to_bytes(&[0xf0, 0x28, 0x8c, 0xbc]).unwrap()],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_2() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    // Now send hi transaction, after that resource should exist and carry value
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi").unwrap(),
        vec![],
        vec![bcs::to_bytes(&[0xc3, 0x28]).unwrap()],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_vec1() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    let bad = vec![0xc3, 0x28];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    // Now send hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi_vec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_vec2() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    let bad = vec![0xff, 0xfe];
    let s_vec = vec!["hello".as_bytes(), "world".as_bytes(), &bad[..]];
    // Now send hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi_vec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_vec_vec_1() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    let bad = vec![0x40, 0xfe];
    let s_vec = vec![
        vec![&bad[..], "hello".as_bytes(), "world".as_bytes()],
        vec![
            "hello".as_bytes(),
            "world".as_bytes(),
            "hi there!".as_bytes(),
        ],
        vec![
            "world".as_bytes(),
            "hi there!".as_bytes(),
            "hello".as_bytes(),
        ],
    ];
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let j = 0u64;
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap(),
        ],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_vec_vec_2() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    let bad = vec![0xf0, 0x28, 0x8c, 0x28];
    let s_vec = vec![
        vec![
            "hi there!".as_bytes(),
            "hello".as_bytes(),
            "world".as_bytes(),
        ],
        vec!["hello".as_bytes(), &bad[..], "hi there!".as_bytes()],
        vec![
            "world".as_bytes(),
            "hi there!".as_bytes(),
            "hello".as_bytes(),
        ],
    ];
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let j = 0u64;
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap(),
        ],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}

#[test]
fn string_args_bad_vec_vec_3() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    let bad = vec![0x40, 0xff];
    let s_vec = vec![
        vec![
            "hi there!".as_bytes(),
            "hello".as_bytes(),
            "world".as_bytes(),
        ],
        vec![
            "hello".as_bytes(),
            "world".as_bytes(),
            "hi there!".as_bytes(),
        ],
        vec!["world".as_bytes(), "hi there!".as_bytes(), &bad[..]],
    ];
    // Now send more_hi_vec transaction, after that resource should exist and carry value
    let i = 0u64;
    let j = 0u64;
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::more_hi_vec").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&s_vec).unwrap(),
            bcs::to_bytes(&i).unwrap(),
            bcs::to_bytes(&j).unwrap(),
        ],
    );
    assert_vm_status!(status, StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
}
