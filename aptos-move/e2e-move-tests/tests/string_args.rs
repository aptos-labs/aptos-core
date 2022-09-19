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

fn success(tests: Vec<(&str, Vec<(Vec<Vec<u8>>, &str)>)>) {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    for (entry, in_out) in tests {
        for (args, expected_change) in in_out {
            assert_success!(h.run_entry_function(&acc, str::parse(entry).unwrap(), vec![], args));
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
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    for (entry, args, _err) in tests {
        // Now send hi transaction, after that resource should exist and carry value
        let status = h.run_entry_function(&acc, str::parse(entry).unwrap(), vec![], args);
        assert_vm_status!(status, _err);
    }
}

// Generates a vector of a vector of strings. Used to produce big size arguments
// that require more than 1 byte lenght when compressed in uleb128
fn big_string_vec(first_dim: u64, second_dim: u64, base: &str) -> Vec<u8> {
    let mut outer = vec![];
    for i in 0..first_dim {
        let mut inner = vec![];
        for j in 0..second_dim {
            inner.push(format!("{}{}{}", base, i, j));
        }
        outer.push(inner);
    }
    bcs::to_bytes(&outer).unwrap()
}

#[test]
fn string_args() {
    let mut tests = vec![];

    // just strings
    let args = vec![bcs::to_bytes("hi there!".as_bytes()).unwrap()];
    let expected_change = "hi there!";

    tests.push(("0xcafe::test::hi", vec![(args, expected_change)]));

    // vector of strings
    let mut in_out = vec![];

    let s_vec = vec![
        "hi there!".as_bytes(),
        "hello".as_bytes(),
        "world".as_bytes(),
    ];
    let i = 0u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "hi there!";
    in_out.push((args, expected_change));

    let i = 1u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "hello";
    in_out.push((args, expected_change));

    let i = 2u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    let expected_change = "world";
    in_out.push((args, expected_change));

    tests.push(("0xcafe::test::hi_vec", in_out));

    // vector of vector of strings
    let mut in_out = vec![];

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
    let i = 0u64;
    let j = 0u64;
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "hi there!";
    in_out.push((args, expected_change));

    let i = 1u64;
    let j = 1u64;
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    let expected_change = "world";
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

    tests.push(("0xcafe::test::more_hi_vec", in_out));

    // vectors or strings with size taking more than 1 byte in uleb128 compression
    let mut in_out = vec![];

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

    tests.push(("0xcafe::test::more_hi_vec", in_out));

    success(tests);
}

#[test]
fn string_args_bad_utf8() {
    let mut tests = vec![];

    // simple strings
    let args = vec![bcs::to_bytes(&[0xf0, 0x28, 0x8c, 0xbc]).unwrap()];
    tests.push((
        "0xcafe::test::hi",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let args = vec![bcs::to_bytes(&[0xc3, 0x28]).unwrap()];
    tests.push((
        "0xcafe::test::hi",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    // vector of strings
    let bad = vec![0xc3, 0x28];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let i = 0u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push((
        "0xcafe::test::hi_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let bad = vec![0xc3, 0x28];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push((
        "0xcafe::test::hi_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    // vector of vector of strings
    let i = 0u64;
    let j = 0u64;

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
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push((
        "0xcafe::test::more_hi_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

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
    let args = vec![
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push((
        "0xcafe::test::more_hi_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    let bad = vec![0x60, 0xff];
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
        "0xcafe::test::more_hi_vec",
        args,
        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
    ));

    fail(tests);
}

#[test]
fn string_args_chopped() {
    let idx = 0u64;
    let s_vec = vec![
        "hi there!".as_bytes(),
        "hello".as_bytes(),
        "world".as_bytes(),
    ];
    let string_arg = bcs::to_bytes(&s_vec).unwrap();
    let mut i = string_arg.len() - 1;
    while i > 1 {
        let mut arg = string_arg.clone();
        arg.remove(i);
        let args = vec![arg, bcs::to_bytes(&idx).unwrap()];
        fail(vec![(
            "0xcafe::test::hi_vec",
            args,
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
        )]);
        i /= 2;
    }
}
