// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_move_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use velor_types::{
    account_address::AccountAddress,
    transaction::{AbortInfo, TransactionStatus},
};
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
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("string_args.data/pack"))
    );

    let mut module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();
    let string_struct = StructTag {
        address: AccountAddress::from_hex_literal("0x1").expect("valid address"),
        module: Identifier::new("string").expect("valid identifier"),
        name: Identifier::new("String").expect("valid identifier"),
        type_args: vec![],
    };
    let string_type = TypeTag::Struct(Box::new(string_struct));
    module_data.type_args.push(string_type);

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

fn deserialization_failure() -> impl Fn(TransactionStatus) {
    let status_code = StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT;
    move |txn_status| assert_vm_status!(txn_status, status_code)
}

fn abort_info() -> impl Fn(TransactionStatus) {
    let abort_info = Some(AbortInfo {
        reason_name: "EINVALID_UTF8".to_string(),
        description: "An invalid UTF8 encoding.".to_string(),
    });
    move |txn_status| assert_move_abort!(txn_status, abort_info)
}

fn fail(tests: Vec<(&str, Vec<Vec<u8>>, impl Fn(TransactionStatus))>) {
    fail_generic(vec![], tests)
}

fn fail_generic(
    ty_args: Vec<TypeTag>,
    tests: Vec<(&str, Vec<Vec<u8>>, impl Fn(TransactionStatus))>,
) {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("string_args.data/pack"))
    );

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    for (entry, args, err) in tests {
        // Now send hi transaction, after that resource should exist and carry value
        let status = h.run_entry_function(&acc, str::parse(entry).unwrap(), ty_args.clone(), args);
        err(status);
    }
}

// Generates a vector of a vector of strings. Used to produce big size arguments
// that require more than 1 byte length when compressed in uleb128
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
fn string_args_good() {
    let mut tests = vec![];

    // just strings
    let args = vec![bcs::to_bytes("hi there!".as_bytes()).unwrap()];
    let expected_change = "hi there!";

    tests.push(("0xcafe::test::hi", vec![(args, expected_change)]));

    let str128 = std::str::from_utf8(&[97u8; 128] as &[u8]).unwrap();
    tests.push(("0xcafe::test::hi", vec![(
        vec![bcs::to_bytes(str128.as_bytes()).unwrap()],
        str128,
    )]));

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
    let u64_vec_max = vec![u64::MAX, u64::MAX, u64::MAX];
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
fn string_args_bad_utf8() {
    let mut tests = vec![];

    // simple strings
    let args = vec![bcs::to_bytes(&vec![0xF0u8, 0x28u8, 0x8Cu8, 0xBCu8]).unwrap()];
    tests.push(("0xcafe::test::hi", args, abort_info()));

    let args = vec![bcs::to_bytes(&vec![0xC3u8, 0x28u8]).unwrap()];
    tests.push(("0xcafe::test::hi", args, abort_info()));

    // vector of strings
    let bad = [0xC3u8, 0x28u8];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let i = 0u64;
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push(("0xcafe::test::str_vec", args, abort_info()));

    let bad = [0xC3u8, 0x28u8];
    let s_vec = vec![&bad[..], "hello".as_bytes(), "world".as_bytes()];
    let args = vec![bcs::to_bytes(&s_vec).unwrap(), bcs::to_bytes(&i).unwrap()];
    tests.push(("0xcafe::test::str_vec", args, abort_info()));

    // vector of vector of strings
    let i = 0u64;
    let j = 0u64;

    let bad = [0x40u8, 0xFEu8];
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
    tests.push(("0xcafe::test::str_vec_vec", args, abort_info()));

    let bad = [0xF0u8, 0x28u8, 0x8Cu8, 0x28u8];
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
    tests.push(("0xcafe::test::str_vec_vec", args, abort_info()));

    let bad = [0x60u8, 0xFFu8];
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
    tests.push(("0xcafe::test::str_vec_vec", args, abort_info()));

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
            "0xcafe::test::str_vec",
            args,
            deserialization_failure(),
        )]);
        i /= 2;
    }
}

#[test]
fn string_args_bad_length() {
    // chop after bcs so length stays big but payload gets small basically a bogus input
    let mut tests = vec![];

    // simple strings

    // length over max size
    let mut args = bcs::to_bytes(&vec![0x30u8; 100000]).unwrap();
    args.truncate(20);
    tests.push(("0xcafe::test::hi", vec![args], deserialization_failure()));

    // length in size but input chopped
    let mut args = bcs::to_bytes(&vec![0x30u8; 30000]).unwrap();
    args.truncate(300);
    tests.push(("0xcafe::test::hi", vec![args], deserialization_failure()));

    // vector of strings

    // length over max size after 2 good strings
    let bad = vec![0x30u8; 100000];
    let s_vec = vec!["hello".as_bytes(), "world".as_bytes(), &bad[..]];
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(200);
    let i = 0u64;
    let args = vec![bcs_vec, bcs::to_bytes(&i).unwrap()];
    tests.push(("0xcafe::test::str_vec", args, deserialization_failure()));

    // length over max size after 2 big-ish strings
    let bad = vec![0x30u8; 100000];
    let big = vec![0x30u8; 10000];
    let s_vec = vec![&big[..], &big[..], &bad[..]];
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(30000);
    let args = vec![bcs_vec, bcs::to_bytes(&i).unwrap()];
    tests.push(("0xcafe::test::str_vec", args, deserialization_failure()));

    // length in size but input chopped
    let big = vec![0x30u8; 10000];
    let s_vec = vec![&big[..], &big[..], &big[..]];
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(20000);
    let args = vec![bcs_vec, bcs::to_bytes(&i).unwrap()];
    tests.push(("0xcafe::test::str_vec", args, deserialization_failure()));

    // vector of vector of strings

    let i = 0u64;
    let j = 0u64;

    let bad = vec![0x30u8; 100000];
    let s_vec = vec![
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
        vec![&bad[..], "hello".as_bytes(), "world".as_bytes()],
    ];
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(30000);
    let args = vec![
        bcs_vec,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push(("0xcafe::test::str_vec_vec", args, deserialization_failure()));

    let bad = vec![0x30u8; 10000];
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
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(10000);
    let args = vec![
        bcs_vec,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push(("0xcafe::test::str_vec_vec", args, deserialization_failure()));

    let bad = vec![0x30u8; 100000];
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
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(30000);
    let args = vec![
        bcs_vec,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];
    tests.push(("0xcafe::test::str_vec_vec", args, deserialization_failure()));

    // length over max size with 0 length strings
    let s_vec = vec![vec!["".as_bytes(); 3]; 100000];
    let mut bcs_vec = bcs::to_bytes(&s_vec).unwrap();
    bcs_vec.truncate(30000);
    // replace the length with u64::max
    // 100000 is the first 3 bytes in the buffer so... we push
    // u64 max in ule128 in opposite order so vector swap_remove is good
    // but we need to remove a 0 after to keep the vector consistent... don't ask...
    // u64 max in ule128 in opposite order so vector swap_remove is good
    let mut u64_max: Vec<u8> = vec![0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let len = u64_max.len();
    bcs_vec.append(&mut u64_max);
    let mut i = 0;
    while i < len {
        bcs_vec.swap_remove(i);
        i += 1;
    }
    bcs_vec.remove(i);

    let args = vec![
        bcs_vec,
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];

    tests.push(("0xcafe::test::str_vec_vec", args, deserialization_failure()));

    fail(tests);
}

#[test]
fn string_args_non_generic_call() {
    let tests = vec![("0xcafe::test::non_generic_call", vec![(
        vec![bcs::to_bytes("hi".as_bytes()).unwrap()],
        "hi",
    )])];

    success_generic(vec![], tests);
}

#[test]
fn string_args_generic_call() {
    let tests = vec![("0xcafe::test::generic_call", vec![(
        vec![bcs::to_bytes("hi".as_bytes()).unwrap()],
        "hi",
    )])];

    let string_struct = StructTag {
        address: AccountAddress::from_hex_literal("0x1").expect("valid address"),
        module: Identifier::new("string").expect("valid identifier"),
        name: Identifier::new("String").expect("valid identifier"),
        type_args: vec![],
    };
    let string_type = TypeTag::Struct(Box::new(string_struct));

    success_generic(vec![string_type], tests);
}

#[test]
fn string_args_generic_instantiation() {
    let mut tests = vec![];
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
    let u8_vec = vec![0xFFu8; 100];
    let u64_vec = vec![u64::MAX, u64::MAX, u64::MAX];
    let val1 = long_addr;
    let val2 = "hi there! hello".as_bytes();
    let i = 0u64;
    let j = 0u64;
    let args = vec![
        bcs::to_bytes(&a_vec).unwrap(),
        bcs::to_bytes(&s_vec).unwrap(),
        bcs::to_bytes(&u8_vec).unwrap(),
        bcs::to_bytes(&u64_vec).unwrap(),
        bcs::to_bytes(&val1).unwrap(),
        bcs::to_bytes(&val2).unwrap(),
        bcs::to_bytes(&i).unwrap(),
        bcs::to_bytes(&j).unwrap(),
    ];

    tests.push(("0xcafe::test::generic_multi_vec", vec![(
        args,
        "hi there! hello",
    )]));

    let address_type = TypeTag::Address;
    let string_struct = StructTag {
        address: AccountAddress::from_hex_literal("0x1").expect("valid address"),
        module: Identifier::new("string").expect("valid identifier"),
        name: Identifier::new("String").expect("valid identifier"),
        type_args: vec![],
    };
    let string_type = TypeTag::Struct(Box::new(string_struct));

    success_generic(vec![string_type, address_type], tests);
}

#[test]
fn huge_string_args_are_not_allowed() {
    let mut tests = vec![];
    let mut len: u64 = 1_000_000_000_000;
    let mut big_str_arg = vec![];
    loop {
        let cur = len & 0x7F;
        if cur != len {
            big_str_arg.push((cur | 0x80) as u8);
            len >>= 7;
        } else {
            big_str_arg.push(cur as u8);
            break;
        }
    }
    tests.push((
        "0xcafe::test::hi",
        vec![big_str_arg],
        deserialization_failure(),
    ));
    fail(tests);
}
