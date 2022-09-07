use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    event::EventHandle,
    transaction::{EntryFunction, TransactionPayload},
};
use cached_packages::aptos_stdlib;
use framework::{BuildOptions, BuiltPackage};
use language_e2e_tests::account::Account;
use move_deps::move_core_types::{
    ident_str, identifier::Identifier, language_storage::ModuleId, parser::parse_struct_tag,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ModuleData {
    resource_signer_cap: AccountAddress,
}

#[derive(Serialize, Deserialize)]
struct Coin {
    value: u64,
}

#[derive(Serialize, Deserialize)]
struct CoinStore {
    coin: Coin,
    frozen: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
}

const APTOS_COIN_STRUCT_STRING: &str = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
const WRAPPED_APTOS_COIN_STRUCT_STRING: &str ="0x1::coin::CoinStore<0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a::resource_account::WrappedAptosCoin>";

#[test]
fn resource_account_exchange_e2e_test() {
    let mut h = MoveHarness::new();

    // create an origin account and create a resource address from it
    let origin_account = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*origin_account.address(), vec![].as_slice());

    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_account".to_string(), resource_address);
    let package = BuiltPackage::build(
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
        "0x{}::resource_account::ModuleData",
        resource_address
    ))
    .unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(&resource_address, module_data)
            .unwrap()
            .resource_signer_cap,
        resource_address
    );

    // verify that exchange_to() and exchange_from() are working properly
    let test_user_account = h.new_account_with_balance_and_sequence_number(20, 10);
    assert_coin_store_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        20,
    );

    // swap from 5 aptos coins to 5 wrapped aptos coins
    run_exchange_function(
        &mut h,
        test_user_account.clone(),
        ident_str!("exchange_to_entry").to_owned(),
        5,
        10,
    );
    assert_coin_store_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        15,
    );
    assert_coin_store_balance(
        &mut h,
        test_user_account.address(),
        WRAPPED_APTOS_COIN_STRUCT_STRING,
        5,
    );
    assert_coin_store_balance(&mut h, &resource_address, APTOS_COIN_STRUCT_STRING, 5);
    assert_coin_store_balance(
        &mut h,
        &resource_address,
        WRAPPED_APTOS_COIN_STRUCT_STRING,
        0,
    );

    // swap to 3 aptos coins from 3 wrapped aptos coins
    run_exchange_function(
        &mut h,
        test_user_account.clone(),
        ident_str!("exchange_from_entry").to_owned(),
        3,
        11,
    );
    assert_coin_store_balance(
        &mut h,
        test_user_account.address(),
        APTOS_COIN_STRUCT_STRING,
        18,
    );
    assert_coin_store_balance(
        &mut h,
        test_user_account.address(),
        WRAPPED_APTOS_COIN_STRUCT_STRING,
        2,
    );
    assert_coin_store_balance(&mut h, &resource_address, APTOS_COIN_STRUCT_STRING, 2);
    assert_coin_store_balance(
        &mut h,
        &resource_address,
        WRAPPED_APTOS_COIN_STRUCT_STRING,
        0,
    );
}

// check the coin store balance of `struct_tag_string` CoinType at the given `address` is the same as the expected coin balance
fn assert_coin_store_balance(
    h: &mut MoveHarness,
    address: &AccountAddress,
    struct_tag_string: &str,
    expected_coin_amount: u64,
) {
    let coin_store_balance = h
        .read_resource::<CoinStore>(address, parse_struct_tag(struct_tag_string).unwrap())
        .unwrap()
        .coin;
    assert_eq!(coin_store_balance.value, expected_coin_amount);
}

fn run_exchange_function(
    h: &mut MoveHarness,
    account: Account,
    function: Identifier,
    amount: u64,
    sequence_number: u64,
) {
    let exchange_to_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            create_resource_address(
                AccountAddress::from_hex_literal("0xcafe").unwrap(),
                vec![].as_slice(),
            ),
            ident_str!("resource_account").to_owned(),
        ),
        function,
        vec![],
        vec![bcs::to_bytes::<u64>(&amount).unwrap()],
    ));

    // set the transaction gas unit price to 0 for testing purpose,
    // so we'd know for sure how many remaining coins are in the user's CoinStore
    assert_success!(h.run(
        account
            .transaction()
            .sequence_number(sequence_number)
            .max_gas_amount(100_000)
            .gas_unit_price(0)
            .payload(exchange_to_payload)
            .sign()
    ));
}
