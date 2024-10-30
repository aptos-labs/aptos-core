// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{
    assert_success,
    tests::{common, gas::print_gas_cost},
    MoveHarness,
};
use aptos_types::account_address::AccountAddress;
use rstest::rstest;

/// Run with `cargo test test_smart_data_structures_gas -- --nocapture` to see output.
#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_smart_data_structures_gas(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    // This test uses a lot of execution gas so the upper bound need to be bumped to accommodate it.
    h.modify_gas_schedule(|params| params.vm.txn.max_execution_gas = 40_000_000_000.into());
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0) });
    assert_success!(h.publish_package(&acc, &common::test_dir_path("smart_data_structures.data")));

    print_gas_cost(
        "huge_smart_vector_create_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::create_smart_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_smart_vector_update_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::update_smart_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_smart_vector_read_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::read_smart_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_vector_create_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::create_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_vector_update_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::update_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_vector_read_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::read_vector").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_smart_table_create_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::create_smart_table").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_smart_table_update_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::update_smart_table").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_smart_table_read_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::read_smart_table").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_table_create_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::create_table").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_table_update_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::update_table").unwrap(),
            vec![],
            vec![],
        ),
    );
    print_gas_cost(
        "huge_table_read_gas",
        h.evaluate_entry_function_gas(
            &acc,
            str::parse("0xcafe::test::read_table").unwrap(),
            vec![],
            vec![],
        ),
    );
}
