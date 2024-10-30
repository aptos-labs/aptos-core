// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    parser::parse_struct_tag,
    vm_status::StatusCode,
};
use rstest::rstest;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

fn module_data() -> StructTag {
    parse_struct_tag("0xCAFE::test::ModuleData").unwrap()
}

fn success(
    h: &mut MoveHarness,
    tests: Vec<(&str, Vec<Vec<u8>>, &str)>,
    stateless_account: bool,
    object_address: AccountAddress,
) {
    success_generic(h, vec![], tests.clone(), stateless_account, object_address);
}

fn success_generic(
    h: &mut MoveHarness,
    ty_args: Vec<TypeTag>,
    tests: Vec<(&str, Vec<Vec<u8>>, &str)>,
    stateless_account: bool,
    object_address: AccountAddress,
) {
    // Load the code
    let acc = h.new_account_at(
        AccountAddress::from_hex_literal("0xcafe").unwrap(),
        if stateless_account { None } else { Some(0) },
    );

    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("constructor_args.data/pack")
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
                h.read_resource::<ModuleData>(&object_address, module_data())
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
    stateless_account: bool,
) {
    // Load the code
    let seq_num = if stateless_account { None } else { Some(0) };
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), seq_num);
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("constructor_args.data/pack")
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

fn fail(h: &mut MoveHarness, tests: Vec<(&str, Vec<Vec<u8>>, Closure)>, stateless_sender: bool) {
    fail_generic(h, vec![], tests, stateless_sender);
}

fn fail_generic(
    h: &mut MoveHarness,
    ty_args: Vec<TypeTag>,
    tests: Vec<(&str, Vec<Vec<u8>>, Closure)>,
    stateless_sender: bool,
) {
    // Load the code
    let seq_num = if stateless_sender { None } else { Some(0) };
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), seq_num);
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("constructor_args.data/pack")
    ));

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data()));

    for (entry, args, err) in tests {
        // Now send hi transaction, after that resource should exist and carry value
        err(h.run_entry_function(&acc, str::parse(entry).unwrap(), ty_args.clone(), args));
    }
}

// TODO[Orderless]: The object address created by an orderless transaction varies based on the nonce used.
// Support orderless transactions here, by computing the object address that could have been created by the orderless transaction.
#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    // case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    // case(false, true, true),
)]
fn constructor_args_good(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let object_address = AccountAddress::from_hex_literal(
        "0x2b8c0bdf1e6d3886f2fbf7e9cee0fb1fadd9bad887fb93175af7cc6b5bb8ca02",
    )
    .expect("can't compute object address");
    let tests = vec![
        // ensure object exist
        ("0xcafe::test::initialize", vec![], ""),
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&object_address).unwrap(),
            ],
            "hi",
        ),
        (
            "0xcafe::test::pass_optional_fixedpoint32",
            vec![
                bcs::to_bytes(&object_address).unwrap(),     // Object<T>
                bcs::to_bytes(&vec![(1u64 << 32)]).unwrap(), // Option<FixedPoint32>
            ],
            "4294967296",
        ),
        (
            "0xcafe::test::pass_optional_vector_fixedpoint64",
            vec![
                bcs::to_bytes(&object_address).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![(1u128 << 64), (2u128 << 64)]]).unwrap(), // Option<vector<FixedPoint64>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "36893488147419103232", // 2 in fixedpoint64
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&object_address).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec!["a"], vec!["b"]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "b", // second element of the vector
        ),
        (
            "0xcafe::test::pass_vector_optional_object",
            vec![
                bcs::to_bytes(&vec![vec![object_address], vec![]]).unwrap(), // vector<Option<Object<T>>>
                bcs::to_bytes(&"pff vectors of optionals").unwrap(),
                bcs::to_bytes(&0u64).unwrap(),
            ],
            "pff vectors of optionals",
        ),
        (
            "0xcafe::test::ensure_vector_vector_u8",
            vec![
                bcs::to_bytes(&object_address).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![1u8], vec![2u8]]).unwrap(), // vector<vector<u8>>
            ],
            "vector<vector<u8>>",
        ),
    ];

    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    success(&mut h, tests, stateless_account, object_address);
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    // case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    // case(false, true, true),
)]
fn view_constructor_args(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let object_address = AccountAddress::from_hex_literal(
        "0x2b8c0bdf1e6d3886f2fbf7e9cee0fb1fadd9bad887fb93175af7cc6b5bb8ca02",
    )
    .expect("can't compute object address");
    let tests = vec![
        // ensure object exist
        ("0xcafe::test::initialize", vec![], ""),
        // make state equal hi
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&object_address).unwrap(),
            ],
            "hi",
        ),
    ];

    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    success(&mut h, tests, stateless_account, object_address);

    let view = vec![(
        "0xcafe::test::get_state",
        vec![bcs::to_bytes(&object_address).unwrap()],
        "hi",
    )];
    let module_data_type = TypeTag::Struct(Box::new(module_data()));
    success_generic_view(
        &mut h,
        vec![module_data_type.clone()],
        view,
        stateless_account,
    );
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    // case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    // case(false, true, true),
)]
fn constructor_args_bad(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let object_address = AccountAddress::from_hex_literal(
        "0x2b8c0bdf1e6d3886f2fbf7e9cee0fb1fadd9bad887fb93175af7cc6b5bb8ca02",
    )
    .expect("can't compute object address");
    let good: &[u8] = "a".as_bytes();
    let bad: &[u8] = &[0x80u8; 1];

    let tests: Vec<(&str, Vec<Vec<u8>>, Closure)> = vec![
        // object doesnt exist
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&object_address).unwrap(),
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
                )
            }),
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&object_address).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec![good], vec![bad]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT
                    )))
                )
            }),
        ),
        (
            "0xcafe::test::ensure_no_fabrication",
            vec![
                bcs::to_bytes(&vec![1u64]).unwrap(), // Option<MyPrecious>
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

    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    fail(&mut h, tests, stateless_account);
}
