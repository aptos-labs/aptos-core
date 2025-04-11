// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, build_package, tests::common, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{ident_str, language_storage::ModuleId, parser::parse_struct_tag};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
struct ModuleData {
    resource_signer_cap: AccountAddress,
    burn_cap: Vec<u8>, // placeholder for burn capability
    mint_cap: Vec<u8>, // placeholder for mint capability
}

const APTOS_COIN_STRUCT_STRING: &str = "0x1::aptos_coin::AptosCoin";
const EXCHANGE_FROM_FUNCTION: &str = "exchange_from_entry";
const EXCHANGE_TO_FUNCTION: &str = "exchange_to_entry";

#[rstest(
    origin_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn exchange_e2e_test(
    origin_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    // create an origin account and create a resource address from it
    let origin_account = h.new_account_with_key_pair(
        if origin_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let resource_address = create_resource_address(*origin_account.address(), vec![].as_slice());

    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_account".to_string(), resource_address);
    build_options
        .named_addresses
        .insert("source_addr".to_string(), *origin_account.address());
    let package = build_package(
        common::test_dir_path("../../../move-examples/resource_account"),
        build_options,
    )
    .expect("building package must succeed");
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    // create the resource account and publish the code under the resource account's address
    let result = h.run_transaction_payload(
        &origin_account,
        aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );
    assert_success!(result);

    // verify that we store the signer cap within the module
    let module_data = parse_struct_tag(&format!(
        "0x{}::simple_defi::ModuleData",
        resource_address.to_hex()
    ))
    .unwrap();

    assert_eq!(
        h.read_resource::<ModuleData>(&resource_address, module_data)
            .unwrap()
            .resource_signer_cap,
        resource_address
    );

    // verify that exchange_to() and exchange_from() are working properly
    // We are not testing with `test_user_account` being a stateless account, as the account creation costs
    // then need to be factored into the below assertion statements.
    let test_user_account = h.new_account_with_balance_and_sequence_number(20, Some(0));
    assert_coin_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        20,
    );

    // swap from 5 aptos coins to 5 chloe's coins
    run_exchange_function(
        &mut h,
        resource_address,
        &test_user_account,
        EXCHANGE_TO_FUNCTION,
        5,
        0,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_coin_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        15,
    );

    let chloe_coin_struct_string: &str =
        &format!("0x{}::simple_defi::ChloesCoin", resource_address.to_hex());

    assert_coin_balance(
        &mut h,
        test_user_account.address(),
        chloe_coin_struct_string,
        5,
    );
    assert_coin_balance(&mut h, &resource_address, APTOS_COIN_STRUCT_STRING, 5);
    assert_coin_balance(&mut h, &resource_address, chloe_coin_struct_string, 0);

    // swap to 3 aptos coins from 3 chloe's aptos coins
    run_exchange_function(
        &mut h,
        resource_address,
        &test_user_account,
        EXCHANGE_FROM_FUNCTION,
        3,
        1,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_coin_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        18,
    );
    assert_coin_balance(
        &mut h,
        test_user_account.address(),
        chloe_coin_struct_string,
        2,
    );
    assert_coin_balance(&mut h, &resource_address, APTOS_COIN_STRUCT_STRING, 2);
    assert_coin_balance(&mut h, &resource_address, chloe_coin_struct_string, 0);
}

/// check the coin store balance of `struct_tag_string` CoinType at the given `address` is the same as the `expected_coin_amount`
fn assert_coin_balance(
    h: &mut MoveHarness,
    address: &AccountAddress,
    struct_tag_string: &str,
    expected_coin_amount: u64,
) {
    let bytes = h
        .execute_view_function(
            str::parse("0x1::coin::balance").unwrap(),
            vec![move_core_types::language_storage::TypeTag::from_str(struct_tag_string).unwrap()],
            vec![address.to_vec()],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let balance = bcs::from_bytes::<u64>(bytes.as_slice()).unwrap();
    assert_eq!(balance, expected_coin_amount);
}

/// run the specified exchange function and check if it runs successfully
fn run_exchange_function(
    h: &mut MoveHarness,
    resource_address: AccountAddress,
    account: &Account,
    function: &'static str,
    amount: u64,
    sequence_number: u64,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let exchange_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(resource_address, ident_str!("simple_defi").to_owned()),
        ident_str!(function).to_owned(),
        vec![],
        vec![bcs::to_bytes::<u64>(&amount).unwrap()],
    ));

    // set the transaction gas unit price to 0 for testing purpose,
    // so we'd know for sure how many remaining coins are in the user's CoinStore
    let txn = account
        .transaction()
        .sequence_number(sequence_number)
        .max_gas_amount(100_000)
        .gas_unit_price(0)
        .payload(exchange_payload)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    assert_success!(h.run(txn));
}
