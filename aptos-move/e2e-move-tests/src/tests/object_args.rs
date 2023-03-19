// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    parser::parse_struct_tag,
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

fn success(tests: Vec<(&str, Vec<(Vec<Vec<u8>>, &str)>)>) {
    success_generic(vec![], tests)
}

fn success_generic(ty_args: Vec<TypeTag>, tests: Vec<(&str, Vec<(Vec<Vec<u8>>, &str)>)>) {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("object_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    for (entry, in_out) in tests {
        for (args, expected_change) in in_out {
            assert_success!(h.run_entry_function(
                &acc,
                str::parse(entry).unwrap(),
                ty_args.clone(),
                args,
            ));
            assert_eq!(
                String::from_utf8(
                    h.read_resource::<ModuleData>(acc.address(), module_data.clone())
                        .unwrap()
                        .state
                )
                .unwrap(),
                expected_change,
            );
        }
    }
}

fn fail(tests: Vec<(&str, Vec<Vec<u8>>, StatusCode)>) {
    fail_generic(vec![], tests)
}

fn fail_generic(ty_args: Vec<TypeTag>, tests: Vec<(&str, Vec<Vec<u8>>, StatusCode)>) {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("object_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    for (entry, args, _err) in tests {
        // Now send hi transaction, after that resource should exist and carry value
        let status = h.run_entry_function(&acc, str::parse(entry).unwrap(), ty_args.clone(), args);
        assert_vm_status!(status, _err);
    }
}

#[test]
fn object_args_good() {
    let mut tests = vec![];

    // just strings
    let args = vec![bcs::to_bytes("hi there!".as_bytes()).unwrap()];
    let expected_change = "hi there!";

    tests.push(("0xcafe::test::hi", vec![(args, expected_change)]));

    // vector of strings
    let mut in_out = vec![];

    let s_vec = vec![
        "hi there! hello".as_bytes(),
        "hello".as_bytes(),
        "world, hello world".as_bytes(),
    ];
    let i = 0u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "hi there! hello";
    in_out.push((args, expected_change));

    let i = 1u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "hello";
    in_out.push((args, expected_change));

    let i = 2u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "world, hello world";
    in_out.push((args, expected_change));

    tests.push(("0xcafe::test::str_vec", in_out));

    // vector of vector of strings
    let mut in_out = vec![];

    let s_vec = vec![
        vec![
            "hi there! hello".as_bytes(),
            "hello".as_bytes(),
            "world, hello world".as_bytes(),
        ],
        vec![
            "hello".as_bytes(),
            "world, hello world".as_bytes(),
            "hi there! hello".as_bytes(),
        ],
        vec![
            "world, hello world".as_bytes(),
            "hi there! hello".as_bytes(),
            "hello".as_bytes(),
        ],
    ];
    let i = 0u64;
    let j = 0u64;
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "hi there! hello";
    in_out.push((args, expected_change));

    let i = 1u64;
    let j = 1u64;
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "world, hello world";
    in_out.push((args, expected_change));

    let i = 2u64;
    let j = 2u64;
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "hello";
    in_out.push((args, expected_change));

    let s_vec = vec![vec!["hello".as_bytes(); 50]; 200];
    let bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    let i = 0u64;
    let j = 0u64;
    let args = vec![
        bcs_vec,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "hello";
    in_out.push((args, expected_change));

    // vectors or strings with size taking more than 1 byte in uleb128 compression
    let hello = "hello".repeat(60);
    let string_arg = big_string_vec(10, 10, hello.as_str());
    let i = 8u64;
    let j = 7u64;
    let args = vec![
        string_arg,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = format!("{}{}{}", hello, i, j);
    in_out.push((args, expected_change.as_str()));

    let hello = "hello".repeat(6);
    let string_arg = big_string_vec(300, 2, hello.as_str());
    let i = 8u64;
    let j = 0u64;
    let args = vec![
        string_arg,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = format!("{}{}{}", hello, i, j);
    in_out.push((args, expected_change.as_str()));

    let hello = "hello".repeat(6);
    let string_arg = big_string_vec(2, 300, hello.as_str());
    let i = 0u64;
    let j = 7u64;
    let args = vec![
        string_arg,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = format!("{}{}{}", hello, i, j);
    in_out.push((args, expected_change.as_str()));

    tests.push(("0xcafe::test::str_vec_vec", in_out));

    // multi vector
    let long_addr = AccountAddress::from_hex_literal(
        "0xffabcdeffff555777876542123456789ca843279e3427144cead5e4d59ffffff",
    )
    .unwrap();
    let a_vec = vec![vec![&long_addr; 2], vec![&long_addr; 2]];
    let s_vec = vec![
        vec![
            "hi there! hello".as_bytes(),
            "hello".as_bytes(),
            "world, hello world".as_bytes(),
        ],
        vec![
            "hello".as_bytes(),
            "world, hello world".as_bytes(),
            "hi there! hello".as_bytes(),
        ],
        vec![
            "world, hello world".as_bytes(),
            "hi there! hello".as_bytes(),
            "hello".as_bytes(),
        ],
    ];
    let u64_vec_max = vec![std::u64::MAX, std::u64::MAX, std::u64::MAX];
    let u64_long = vec![0xABCDEFu64; 100];
    let i = 0u64;
    let j = 0u64;
    let args = vec![
        bcs::to_bytes(&a_vec).unwrap(),
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&u64_vec_max).unwrap(),
        bcs::to_bytes(&u64_long).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "hi there! hello";

    tests.push(("0xcafe::test::multi_vec", vec![(args, expected_change)]));

    success(tests);
}

#[test]
fn object_args_bad() {
    let mut tests = vec![];

    // simple strings
    let args = vec![bcs::to_bytes(&vec![0xF0u8, 0x28u8, 0x8Cu8, 0xBCu8]).unwrap()];
    tests.push((
        "0xcafe::test::hi",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let args = vec![bcs::to_bytes(&vec![0xC3u8, 0x28u8]).unwrap()];
    tests.push((
        "0xcafe::test::hi",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    // vector of strings
    let bad = vec![0xC3u8, 0x28u8];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let i = 0u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push((
        "0xcafe::test::str_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let bad = vec![0xC3u8, 0x28u8];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push((
        "0xcafe::test::str_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    // vector of vector of strings
    let i = 0u64;
    let j = 0u64;

    let bad = vec![0x40u8, 0xFEu8];
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
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push((
        "0xcafe::test::str_vec_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let bad = vec![0xF0u8, 0x28u8, 0x8Cu8, 0x28u8];
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
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push((
        "0xcafe::test::str_vec_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let bad = vec![0x60u8, 0xFFu8];
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
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push((
        "0xcafe::test::str_vec_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    fail(tests);
}
