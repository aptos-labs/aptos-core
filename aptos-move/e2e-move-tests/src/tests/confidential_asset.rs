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

const FRAMEWORK_ADDRESS: AccountAddress = AccountAddress::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
]);

/// Standard test account addresses
const ALICE_ADDRESS: &str = "0xa11ce";
const BOB_ADDRESS: &str = "0xb0b";

// =================================================================================================
// Helpers: Set up and clean up tests
// =================================================================================================

/// Creates a new MoveHarness with the necessary feature flags enabled, just in case they are not.
/// When the `move-harness-with-test-only` feature is enabled, this includes #[test_only] code.
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

const MODULE_NAME: &str = "confidential_asset";

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

/// Create a payload for primary_fungible_store::transfer (non-confidential transfer).
fn create_fungible_asset_transfer_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            ident_str!("primary_fungible_store").to_owned(),
        ),
        ident_str!("transfer").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("fungible_asset").unwrap(),
            name: Identifier::new("Metadata").unwrap(),
            type_args: vec![],
        }))],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::withdraw_to function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_withdraw_to_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
    new_balance_bytes: Vec<u8>,
    zkrp_new_balance_bytes: Vec<u8>,
    sigma_proof_bytes: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("withdraw_to").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
            bcs::to_bytes(&new_balance_bytes).unwrap(),
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&sigma_proof_bytes).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::confidential_transfer function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_confidential_transfer_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    new_balance_bytes: Vec<u8>,
    sender_amount_bytes: Vec<u8>,
    recipient_amount_bytes: Vec<u8>,
    zkrp_new_balance_bytes: Vec<u8>,
    zkrp_transfer_amount_bytes: Vec<u8>,
    sigma_proof_bytes: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("confidential_transfer").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&new_balance_bytes).unwrap(),
            bcs::to_bytes(&sender_amount_bytes).unwrap(),
            bcs::to_bytes(&recipient_amount_bytes).unwrap(),
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(), // auditor_eks (empty)
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(), // auditor_amounts (empty)
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&zkrp_transfer_amount_bytes).unwrap(),
            bcs::to_bytes(&sigma_proof_bytes).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::rotate_encryption_key function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_rotate_encryption_key_payload(
    token_metadata: AccountAddress,
    new_ek_bytes: Vec<u8>,
    new_balance_bytes: Vec<u8>,
    zkrp_new_balance_bytes: Vec<u8>,
    sigma_proof_bytes: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("rotate_encryption_key").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&new_ek_bytes).unwrap(),
            bcs::to_bytes(&new_balance_bytes).unwrap(),
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&sigma_proof_bytes).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::normalize function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_normalize_payload(
    token_metadata: AccountAddress,
    new_balance_bytes: Vec<u8>,
    zkrp_new_balance_bytes: Vec<u8>,
    sigma_proof_bytes: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("normalize").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&new_balance_bytes).unwrap(),
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&sigma_proof_bytes).unwrap(),
        ],
    ))
}

/// Create a payload for freezing a token (needed before rotate_encryption_key).
#[cfg(feature = "move-harness-with-test-only")]
fn create_rollover_and_freeze_payload(token_metadata: AccountAddress) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("rollover_pending_balance_and_freeze").to_owned(),
        vec![],
        vec![bcs::to_bytes(&token_metadata).unwrap()],
    ))
}

/// Generate a valid twisted ElGamal keypair using the Move function.
/// Returns (dk_bytes, ek_bytes).
#[cfg(feature = "move-harness-with-test-only")]
fn generate_keypair(h: &mut MoveHarness) -> (Vec<u8>, Vec<u8>) {
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            "ristretto255_twisted_elgamal",
            "generate_twisted_elgamal_keypair",
            vec![],
            vec![],
        )
        .expect("generate_twisted_elgamal_keypair should succeed");

    assert_eq!(result.return_values.len(), 2);
    let dk_bytes: Vec<u8> = bcs::from_bytes(&result.return_values[0].0).expect("deserialize dk");
    let ek_bytes: Vec<u8> = bcs::from_bytes(&result.return_values[1].0).expect("deserialize ek");

    (dk_bytes, ek_bytes)
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
fn print_gas_cost(
    function: &str,
    gas_units: u64,
    fee: &FeeStatement,
    address: AccountAddress,
    module_name: &str,
) {
    println!();
    println!(
        "Gas report for {}::{}::{}, assuming 100 octas / gas unit",
        address.to_hex_literal(),
        module_name,
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

    print_gas_cost(
        "register",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
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

    print_gas_cost(
        "deposit_to",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
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
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
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

/// Profile gas usage for a normal (non-confidential) fungible asset transfer.
/// This is useful for comparing gas costs with confidential_transfer.
fn profile_fungible_asset_transfer(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Benchmark a normal fungible asset transfer from Alice to Bob
    let transfer_amount = 100u64;
    let payload =
        create_fungible_asset_transfer_payload(apt_metadata, *bob.address(), transfer_amount);

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "transfer",
        gas_used,
        &fee_statement.unwrap(),
        FRAMEWORK_ADDRESS,
        "primary_fungible_store",
    );
    maybe_generate_html_report(detailed, &gas_log, "transfer");

    assert_success(&status, "transfer");
    assert!(gas_used > 0, "transfer should consume gas");
}

#[test]
fn bench_gas_fungible_asset_transfer() {
    profile_fungible_asset_transfer(false);
}

#[test]
#[ignore]
fn bench_gas_fungible_asset_transfer_detailed() {
    profile_fungible_asset_transfer(true);
}

/// Profile gas usage for the confidential asset `withdraw_to` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_withdraw_to(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate valid keypairs for Alice and Bob
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (_bob_dk, bob_ek) = generate_keypair(&mut h);

    // Register both accounts for confidential assets
    register_account(&mut h, &alice, apt_metadata, &alice_ek);
    register_account(&mut h, &bob, apt_metadata, &bob_ek);

    // Deposit some tokens to Alice and rollover to actual balance
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_to_payload(apt_metadata, *alice.address(), deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit_to");

    let rollover_payload = create_rollover_pending_balance_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_payload);
    assert_success(&status, "rollover_pending_balance");

    // Generate withdrawal proof bytes using test-only Move function
    let withdraw_amount = 100u64;
    let new_balance_amount = (deposit_amount - withdraw_amount) as u128;

    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "generate_withdrawal_proof_bytes",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk).unwrap(),
                bcs::to_bytes(&withdraw_amount).unwrap(),
                bcs::to_bytes(&new_balance_amount).unwrap(),
            ],
        )
        .expect("generate_withdrawal_proof_bytes should succeed");

    // Extract the three return values: (new_balance_bytes, zkrp_bytes, sigma_bytes)
    assert_eq!(result.return_values.len(), 3);
    let new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize new_balance_bytes");
    let zkrp_new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[1].0).expect("deserialize zkrp_bytes");
    let sigma_proof_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[2].0).expect("deserialize sigma_bytes");

    // Create and execute the withdraw_to transaction
    let payload = create_withdraw_to_payload(
        apt_metadata,
        *bob.address(),
        withdraw_amount,
        new_balance_bytes,
        zkrp_new_balance_bytes,
        sigma_proof_bytes,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "withdraw_to",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
    maybe_generate_html_report(detailed, &gas_log, "confidential_asset_withdraw_to");

    assert_success(&status, "withdraw_to");
    assert!(gas_used > 0, "withdraw_to should consume gas");
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_withdraw_to() {
    profile_confidential_asset_withdraw_to(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_withdraw_to_detailed() {
    profile_confidential_asset_withdraw_to(true);
}

/// Profile gas usage for the confidential asset `confidential_transfer` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_confidential_transfer(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate valid keypairs for Alice and Bob
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (_bob_dk, bob_ek) = generate_keypair(&mut h);

    // Register both accounts for confidential assets
    register_account(&mut h, &alice, apt_metadata, &alice_ek);
    register_account(&mut h, &bob, apt_metadata, &bob_ek);

    // Deposit some tokens to Alice and rollover to actual balance
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_to_payload(apt_metadata, *alice.address(), deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit_to");

    let rollover_payload = create_rollover_pending_balance_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_payload);
    assert_success(&status, "rollover_pending_balance");

    // Generate transfer proof bytes using test-only Move function
    let transfer_amount = 100u64;
    let new_balance_amount = (deposit_amount - transfer_amount) as u128;

    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "generate_transfer_proof_bytes",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(bob.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk).unwrap(),
                bcs::to_bytes(&transfer_amount).unwrap(),
                bcs::to_bytes(&new_balance_amount).unwrap(),
            ],
        )
        .expect("generate_transfer_proof_bytes should succeed");

    // Extract the six return values
    assert_eq!(result.return_values.len(), 6);
    let new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize new_balance_bytes");
    let sender_amount_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[1].0).expect("deserialize sender_amount_bytes");
    let recipient_amount_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[2].0).expect("deserialize recipient_amount_bytes");
    let zkrp_new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[3].0).expect("deserialize zkrp_new_balance_bytes");
    let zkrp_transfer_amount_bytes: Vec<u8> = bcs::from_bytes(&result.return_values[4].0)
        .expect("deserialize zkrp_transfer_amount_bytes");
    let sigma_proof_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[5].0).expect("deserialize sigma_proof_bytes");

    // Create and execute the confidential_transfer transaction
    let payload = create_confidential_transfer_payload(
        apt_metadata,
        *bob.address(),
        new_balance_bytes,
        sender_amount_bytes,
        recipient_amount_bytes,
        zkrp_new_balance_bytes,
        zkrp_transfer_amount_bytes,
        sigma_proof_bytes,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "confidential_transfer",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
    maybe_generate_html_report(
        detailed,
        &gas_log,
        "confidential_asset_confidential_transfer",
    );

    assert_success(&status, "confidential_transfer");
    assert!(gas_used > 0, "confidential_transfer should consume gas");
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer() {
    profile_confidential_asset_confidential_transfer(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_detailed() {
    profile_confidential_asset_confidential_transfer(true);
}

/// Profile gas usage for the confidential asset `rotate_encryption_key` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_rotate_encryption_key(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate valid keypairs for Alice (current and new)
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (new_dk, new_ek) = generate_keypair(&mut h);

    // Register Alice for confidential assets
    register_account(&mut h, &alice, apt_metadata, &alice_ek);

    // Deposit some tokens and rollover to actual balance
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_to_payload(apt_metadata, *alice.address(), deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit_to");

    // Before key rotation, need to rollover and freeze
    let rollover_freeze_payload = create_rollover_and_freeze_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_freeze_payload);
    assert_success(&status, "rollover_pending_balance_and_freeze");

    // Generate rotation proof bytes using test-only Move function
    let balance_amount = deposit_amount as u128;

    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "generate_rotation_proof_bytes",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk).unwrap(),
                bcs::to_bytes(&new_dk).unwrap(),
                bcs::to_bytes(&new_ek).unwrap(),
                bcs::to_bytes(&balance_amount).unwrap(),
            ],
        )
        .expect("generate_rotation_proof_bytes should succeed");

    // Extract the three return values
    assert_eq!(result.return_values.len(), 3);
    let new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize new_balance_bytes");
    let zkrp_new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[1].0).expect("deserialize zkrp_bytes");
    let sigma_proof_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[2].0).expect("deserialize sigma_bytes");

    // Create and execute the rotate_encryption_key transaction
    let payload = create_rotate_encryption_key_payload(
        apt_metadata,
        new_ek.clone(),
        new_balance_bytes,
        zkrp_new_balance_bytes,
        sigma_proof_bytes,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "rotate_encryption_key",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
    maybe_generate_html_report(
        detailed,
        &gas_log,
        "confidential_asset_rotate_encryption_key",
    );

    assert_success(&status, "rotate_encryption_key");
    assert!(gas_used > 0, "rotate_encryption_key should consume gas");
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_rotate_encryption_key() {
    profile_confidential_asset_rotate_encryption_key(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_rotate_encryption_key_detailed() {
    profile_confidential_asset_rotate_encryption_key(true);
}

/// Profile gas usage for the confidential asset `normalize` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_normalize(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate valid keypairs for Alice and Bob
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (_bob_dk, bob_ek) = generate_keypair(&mut h);

    // Register both accounts for confidential assets
    register_account(&mut h, &alice, apt_metadata, &alice_ek);
    register_account(&mut h, &bob, apt_metadata, &bob_ek);

    // To create a non-normalized balance, we need to deposit the max chunk value multiple times
    // and rollover. The max chunk value is 2^16 - 1 = 65535.
    let max_chunk_value = 65535u64;

    // Deposit to Alice's pending balance twice (this will cause overflow in chunks after rollover)
    let deposit_payload =
        create_deposit_to_payload(apt_metadata, *alice.address(), max_chunk_value);
    let status = h.run_transaction_payload(&alice, deposit_payload.clone());
    assert_success(&status, "deposit_to (1)");

    // Bob deposits to Alice too
    let deposit_payload2 =
        create_deposit_to_payload(apt_metadata, *alice.address(), max_chunk_value);
    let status = h.run_transaction_payload(&bob, deposit_payload2);
    assert_success(&status, "deposit_to (2)");

    // Rollover - this will cause non-normalized balance
    let rollover_payload = create_rollover_pending_balance_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_payload);
    assert_success(&status, "rollover_pending_balance");

    // Generate normalization proof bytes using test-only Move function
    let balance_amount = (max_chunk_value as u128) * 2;

    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "generate_normalization_proof_bytes",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk).unwrap(),
                bcs::to_bytes(&balance_amount).unwrap(),
            ],
        )
        .expect("generate_normalization_proof_bytes should succeed");

    // Extract the three return values
    assert_eq!(result.return_values.len(), 3);
    let new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize new_balance_bytes");
    let zkrp_new_balance_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[1].0).expect("deserialize zkrp_bytes");
    let sigma_proof_bytes: Vec<u8> =
        bcs::from_bytes(&result.return_values[2].0).expect("deserialize sigma_bytes");

    // Create and execute the normalize transaction
    let payload = create_normalize_payload(
        apt_metadata,
        new_balance_bytes,
        zkrp_new_balance_bytes,
        sigma_proof_bytes,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "normalize",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
    maybe_generate_html_report(detailed, &gas_log, "confidential_asset_normalize");

    assert_success(&status, "normalize");
    assert!(gas_used > 0, "normalize should consume gas");
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_normalize() {
    profile_confidential_asset_normalize(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_normalize_detailed() {
    profile_confidential_asset_normalize(true);
}

// =================================================================================================
// Tests: Miscellaneous
// =================================================================================================

/// Test that we can call a private Move function using exec_function_bypass_visibility.
#[test]
#[ignore]
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

/// Complete test that calls a #[test_only] function with proper setup.
/// This test:
/// 1. Sets up a harness (with test-only code when feature is enabled)
/// 2. Creates Alice with APT balance
/// 3. Registers Alice for confidential assets
/// 4. Deposits APT to Alice's confidential balance
/// 5. Calls verify_pending_balance to verify the deposit succeeded
#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn call_test_only_function() {
    // Use harness with test-only code included
    let mut h = setup_harness();

    // Create Alice with some APT
    let alice = h.new_account_with_balance_at(
        AccountAddress::from_hex_literal(ALICE_ADDRESS).unwrap(),
        1_000_000_000, // 10 APT
    );

    // Set up the confidential store for APT
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate a valid keypair for Alice
    let (alice_dk, alice_ek) = generate_keypair(&mut h);

    // Register Alice for confidential assets
    let register_payload = create_register_payload(apt_metadata, &alice_ek);
    let status = h.run_transaction_payload(&alice, register_payload);
    assert_success(&status, "register");
    println!("Alice registered for confidential assets");

    // Deposit 100 units to Alice's pending balance
    let deposit_amount: u64 = 100;
    let deposit_payload = create_deposit_to_payload(apt_metadata, *alice.address(), deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit_to");
    println!("Deposited {} to Alice's pending balance", deposit_amount);

    // Now call the #[test_only] function verify_pending_balance
    // This should succeed and return true since we just deposited `deposit_amount`
    let result = h.exec_function_bypass_visibility(
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
        "verify_pending_balance",
        vec![],
        vec![
            bcs::to_bytes(alice.address()).unwrap(),
            bcs::to_bytes(&apt_metadata).unwrap(),
            bcs::to_bytes(&alice_dk).unwrap(),
            bcs::to_bytes(&deposit_amount).unwrap(),
        ],
    );

    match result {
        Ok(return_values) => {
            assert_eq!(return_values.return_values.len(), 1);
            let (bytes, _) = &return_values.return_values[0];
            let verified: bool = bcs::from_bytes(bytes).expect("Failed to deserialize bool");
            println!(
                "verify_pending_balance returned: {} (expected: true)",
                verified
            );
            assert!(
                verified,
                "verify_pending_balance should return true for correct amount"
            );
            println!("SUCCESS: #[test_only] function verify_pending_balance worked correctly!");
        },
        Err(e) => {
            panic!(
                "verify_pending_balance should succeed with proper setup, but got: {:?}",
                e
            );
        },
    }

    // Also verify that wrong amount returns false
    let wrong_amount: u64 = 999;
    let result = h.exec_function_bypass_visibility(
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
        "verify_pending_balance",
        vec![],
        vec![
            bcs::to_bytes(alice.address()).unwrap(),
            bcs::to_bytes(&apt_metadata).unwrap(),
            bcs::to_bytes(&alice_dk).unwrap(),
            bcs::to_bytes(&wrong_amount).unwrap(),
        ],
    );

    match result {
        Ok(return_values) => {
            let (bytes, _) = &return_values.return_values[0];
            let verified: bool = bcs::from_bytes(bytes).expect("Failed to deserialize bool");
            println!(
                "verify_pending_balance with wrong amount returned: {} (expected: false)",
                verified
            );
            assert!(
                !verified,
                "verify_pending_balance should return false for wrong amount"
            );
        },
        Err(e) => {
            panic!(
                "verify_pending_balance should succeed (and return false), but got: {:?}",
                e
            );
        },
    }
}
