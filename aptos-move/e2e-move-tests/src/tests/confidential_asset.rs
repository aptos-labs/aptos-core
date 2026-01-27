// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas profiling tests for confidential asset operations.
//!
//! These tests measure the gas consumption of various confidential asset functions.
//! Run with `--nocapture` to actually print the results.
//!
//! ```bash
//! cargo test -p e2e-move-tests confidential_asset -- --nocapture
//! ```

use crate::MoveHarness;
use aptos_gas_profiling::TransactionGasLog;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    fee_statement::FeeStatement,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus},
};
use move_core_types::{
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use thousands::Separable;

// =================================================================================================
// Constants
// =================================================================================================

/// A dummy encryption key to be used in the tests: currently, just the Ristretto255 basepoint in
/// compressed form (32 bytes).
const DUMMY_EK: [u8; 32] = [
    0xE2, 0xF2, 0xAE, 0x0A, 0x6A, 0xBC, 0x4E, 0x71, 0xA8, 0x84, 0xA9, 0x61, 0xC5, 0x00, 0x51, 0x5F,
    0x58, 0xE3, 0x0B, 0x6A, 0xA5, 0x82, 0xDD, 0x8D, 0xB6, 0xA6, 0x59, 0x45, 0xE0, 0x8D, 0x2D, 0x76,
];

/// Address where aptos_experimental modules are deployed (0x7)
const EXPERIMENTAL_ADDRESS: AccountAddress = AccountAddress::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
]);

/// Standard test account addresses
const ALICE_ADDRESS: &str = "0xa11ce";
const BOB_ADDRESS: &str = "0xb0b";

// =================================================================================================
// Helpers: Set up and clean up tests
// =================================================================================================

/// Creates a new MoveHarness with the necessary feature flags enabled, just in case they are not.
fn setup_harness() -> MoveHarness {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::BULLETPROOFS_NATIVES,
            FeatureFlag::BULLETPROOFS_BATCH_NATIVES,
            FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE,
        ],
        vec![],
    );
    h
}

/// Creates a test account, funded with some APT, for Alice.
fn create_alice(h: &mut MoveHarness) -> Account {
    h.new_account_with_balance_at(
        AccountAddress::from_hex_literal(ALICE_ADDRESS).unwrap(),
        1_000_000_000,
    )
}

/// Creates a test account, funded with some APT, for Bob.
fn create_bob(h: &mut MoveHarness) -> Account {
    h.new_account_with_balance_at(
        AccountAddress::from_hex_literal(BOB_ADDRESS).unwrap(),
        1_000_000_000,
    )
}

/// Register an account for confidential assets with the given encryption key.
fn register_account(
    h: &mut MoveHarness,
    account: &Account,
    token_metadata: AccountAddress,
    encryption_key: &[u8],
) {
    let payload = create_register_payload(token_metadata, encryption_key);
    let status = h.run_transaction_payload(account, payload);
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "Register for {} should succeed, but got: {:?}",
        account.address(),
        status
    );
}

/// Get APT metadata object address.
fn get_apt_metadata_address() -> AccountAddress {
    AccountAddress::from_hex_literal("0xa").unwrap()
}

const MODULE_NAME: &'static str = "confidential_asset";

/// Set up the confidential asset FA store for APT.
/// This ensures the primary FA store exists for the confidential asset module's FA store address.
/// Returns the APT metadata address.
fn set_up_confidential_store_for_apt(h: &mut MoveHarness) -> AccountAddress {
    let apt_metadata = get_apt_metadata_address();

    // Get the FA store address used by confidential assets
    let return_values = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "get_fa_store_address",
            vec![],
            vec![],
        )
        .expect("get_fa_store_address should succeed");

    let (bytes, _) = &return_values.return_values[0];
    let fa_store_address: AccountAddress =
        bcs::from_bytes(bytes).expect("Failed to deserialize FA store address");

    // Construct the type tag for Metadata (0x1::fungible_asset::Metadata)
    let metadata_type = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("fungible_asset").unwrap(),
        name: Identifier::new("Metadata").unwrap(),
        type_args: vec![],
    }));

    // Ensure the primary store exists for the FA store address
    h.exec_function_bypass_visibility(
        AccountAddress::ONE, // 0x1::primary_fungible_store
        "primary_fungible_store",
        "ensure_primary_store_exists",
        vec![metadata_type],
        vec![
            bcs::to_bytes(&fa_store_address).unwrap(), // owner: address
            bcs::to_bytes(&apt_metadata).unwrap(),     // metadata: Object<Metadata>
        ],
    )
    .expect("ensure_primary_store_exists should succeed");

    apt_metadata
}

/// Create a payload for the confidential_asset::register function.
fn create_register_payload(
    token_metadata: AccountAddress,
    encryption_key: &[u8],
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("register").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&encryption_key.to_vec()).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::deposit_to function.
fn create_deposit_to_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("deposit_to").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::rollover_pending_balance function.
fn create_rollover_pending_balance_payload(token_metadata: AccountAddress) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("rollover_pending_balance").to_owned(),
        vec![],
        vec![bcs::to_bytes(&token_metadata).unwrap()],
    ))
}

/// Assert that a transaction succeeded.
fn assert_success(status: &TransactionStatus, function_name: &str) {
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "{} should succeed, but got: {:?}",
        function_name,
        status
    );
}

// =================================================================================================
// Helpers: Gas profiling
// =================================================================================================

/// Print gas cost in a readable format.
fn print_gas_cost(function: &str, gas_units: u64, fee: &FeeStatement) {
    println!();
    println!(
        "Gas report for {}::{}::{}, assuming 100 octas / gas unit",
        EXPERIMENTAL_ADDRESS.to_hex_literal(),
        MODULE_NAME,
        function
    );
    println!(
        "|  Execution gas:   {} units",
        fee.execution_gas_used().separate_with_commas()
    );
    println!(
        "|  IO gas:          {} units",
        fee.io_gas_used().separate_with_commas()
    );
    println!(
        "|  Storage fee:     {} octas",
        fee.storage_fee_used().separate_with_commas()
    );
    println!(
        "|  Storage refund:  {} octas",
        fee.storage_fee_refund().separate_with_commas()
    );
    println!("* ----------------------------------");
    println!(
        "|  Total gas units: {} units",
        gas_units.separate_with_commas()
    );
    println!(
        "|  Total octas:     {} octas",
        (gas_units * 100).separate_with_commas()
    );
    println!(
        "|  Total APT:       {:.8} APT",
        (gas_units * 100) as f64 / 100_000_000f64
    );
    println!("\\-----------------------------------");
}

/// Generate HTML gas report if detailed profiling is requested.
fn maybe_generate_html_report(detailed: bool, gas_log: &TransactionGasLog, report_name: &str) {
    if detailed {
        let report_path = format!("./gas-profiling/{}", report_name);
        std::fs::create_dir_all("./gas-profiling").ok();
        gas_log
            .generate_html_report(&report_path, format!("{} gas report", report_name))
            .expect("failed to generate gas report");
        println!("HTML report generated at: {}/index.html", report_path);
    }
}

// =================================================================================================
// Tests: Gas benchmarks for *key* confidential asset operations
// =================================================================================================

/// Profile gas usage for the confidential asset `register` function.
fn profile_confidential_asset_register(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    let payload = create_register_payload(apt_metadata, &DUMMY_EK);

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost("register", gas_used, &fee_statement.unwrap());
    maybe_generate_html_report(detailed, &gas_log, "confidential_asset_register");

    assert_success(&status, "register");
    assert!(gas_used > 0, "Register should consume gas");
}

#[test]
fn bench_gas_register() {
    profile_confidential_asset_register(false);
}

#[test]
#[ignore]
fn bench_gas_register_detailed() {
    profile_confidential_asset_register(true);
}

/// Profile gas usage for the confidential asset `deposit_to` function.
fn profile_confidential_asset_deposit_to(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Register both accounts for confidential assets
    register_account(&mut h, &alice, apt_metadata, &DUMMY_EK);
    register_account(&mut h, &bob, apt_metadata, &DUMMY_EK);

    // Record balances before deposit
    let alice_balance_before = h.read_aptos_balance(alice.address());
    let bob_balance_before = h.read_aptos_balance(bob.address());

    // Benchmark deposit_to
    let deposit_amount = 1000u64;
    let payload = create_deposit_to_payload(apt_metadata, *bob.address(), deposit_amount);

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost("deposit_to", gas_used, &fee_statement.unwrap());
    maybe_generate_html_report(detailed, &gas_log, "confidential_asset_deposit_to");

    assert_success(&status, "deposit_to");
    assert!(gas_used > 0, "deposit_to should consume gas");

    // Verify balances
    let alice_balance_after = h.read_aptos_balance(alice.address());
    let bob_balance_after = h.read_aptos_balance(bob.address());

    assert!(
        alice_balance_before - alice_balance_after >= deposit_amount,
        "Alice's balance should decrease by at least the deposit amount. Before: {}, After: {}, Deposit: {}",
        alice_balance_before.separate_with_commas(), alice_balance_after.separate_with_commas(), deposit_amount.separate_with_commas()
    );
    assert_eq!(
        bob_balance_before, bob_balance_after,
        "Bob's public balance should remain unchanged"
    );

    println!(
        "  Alice balance: {} -> {} (deposited {})",
        alice_balance_before.separate_with_commas(),
        alice_balance_after.separate_with_commas(),
        deposit_amount.separate_with_commas()
    );
    println!(
        "  Bob public balance: {} (unchanged)",
        bob_balance_after.separate_with_commas()
    );
}

#[test]
fn bench_gas_deposit_to() {
    profile_confidential_asset_deposit_to(false);
}

#[test]
#[ignore]
fn bench_gas_deposit_to_detailed() {
    profile_confidential_asset_deposit_to(true);
}

/// Profile gas usage for the confidential asset `rollover_pending_balance` function.
fn profile_confidential_asset_rollover_pending_balance(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Register Alice for confidential assets
    register_account(&mut h, &alice, apt_metadata, &DUMMY_EK);

    // Deposit some tokens to Alice so she has a pending balance to rollover
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_to_payload(apt_metadata, *alice.address(), deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit_to");

    // Benchmark rollover_pending_balance
    let payload = create_rollover_pending_balance_payload(apt_metadata);

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "rollover_pending_balance",
        gas_used,
        &fee_statement.unwrap(),
    );
    maybe_generate_html_report(
        detailed,
        &gas_log,
        "confidential_asset_rollover_pending_balance",
    );

    assert_success(&status, "rollover_pending_balance");
    assert!(gas_used > 0, "rollover_pending_balance should consume gas");
}

#[test]
fn bench_gas_rollover_pending_balance() {
    profile_confidential_asset_rollover_pending_balance(false);
}

#[test]
#[ignore]
fn bench_gas_rollover_pending_balance_detailed() {
    profile_confidential_asset_rollover_pending_balance(true);
}

// =================================================================================================
// Tests: Miscellaneous
// =================================================================================================

/// Test that we can call a private Move function using exec_function_bypass_visibility.
#[test]
fn test_call_private_function() {
    let mut h = MoveHarness::new();

    let return_values = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "get_fa_store_address",
            vec![],
            vec![],
        )
        .unwrap();

    assert_eq!(return_values.return_values.len(), 1);
    let (bytes, _) = &return_values.return_values[0];
    let expected = AccountAddress::from_hex_literal(
        "0x5d35f41578f4cebfdc2c4ae38761b890950dfc3c24315e8b5bafd003e8165db9",
    )
    .unwrap();
    let value: AccountAddress = bcs::from_bytes(bytes).expect("Failed to deserialize address");
    println!("Called private function, returned: {}", value);
    assert_eq!(value, expected, "Wrong address returned!");
}
