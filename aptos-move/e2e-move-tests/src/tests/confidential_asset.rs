// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas profiling tests for confidential asset operations.
//!
//! These tests measure the gas consumption of various confidential asset functions.
//! Run with `--release` and `--nocapture` for accurate measurements:
//!
//! ```bash
//! cargo test -p e2e-move-tests confidential_asset --release -- --nocapture
//! ```

use crate::MoveHarness;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus},
};
use move_core_types::{ident_str, language_storage::ModuleId};

/// The Ristretto255 basepoint in compressed form (32 bytes).
/// This is a well-known valid point that can be used as a public key for testing.
const RISTRETTO_BASEPOINT_COMPRESSED: [u8; 32] = [
    0xE2, 0xF2, 0xAE, 0x0A, 0x6A, 0xBC, 0x4E, 0x71, 0xA8, 0x84, 0xA9, 0x61, 0xC5, 0x00, 0x51, 0x5F,
    0x58, 0xE3, 0x0B, 0x6A, 0xA5, 0x82, 0xDD, 0x8D, 0xB6, 0xA6, 0x59, 0x45, 0xE0, 0x8D, 0x2D, 0x76,
];

/// Address where aptos_experimental modules are deployed (0x7)
const EXPERIMENTAL_ADDRESS: AccountAddress = AccountAddress::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
]);

/// Helper to print gas cost in a readable format
fn print_gas_cost(function: &str, gas_units: u64) {
    println!(
        "Gas used for {} (assuming 100 octas / gas unit): {} units",
        function, gas_units
    );
}

/// Get APT metadata object address.
/// APT (AptosCoin) metadata is stored at a deterministic address.
fn get_apt_metadata_address() -> AccountAddress {
    // The APT metadata object is at address 0xa (derived from AptosCoin type)
    AccountAddress::from_hex_literal("0xa").unwrap()
}

/// Create a payload for the confidential_asset::register function.
fn create_register_payload(
    token_metadata: AccountAddress,
    encryption_key: &[u8],
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            EXPERIMENTAL_ADDRESS,
            ident_str!("confidential_asset").to_owned(),
        ),
        ident_str!("register").to_owned(),
        vec![], // no type args
        vec![
            bcs::to_bytes(&token_metadata).unwrap(), // token: Object<Metadata>
            bcs::to_bytes(&encryption_key.to_vec()).unwrap(), // ek: vector<u8>
        ],
    ))
}

/// Profile gas usage for the confidential asset `register` function.
///
/// When `detailed` is true, generates an HTML report in `./gas-profiling/`.
fn profile_confidential_asset_register(detailed: bool) {
    let mut h = MoveHarness::new();

    // Enable required feature flags for confidential assets
    h.enable_features(
        vec![
            FeatureFlag::BULLETPROOFS_NATIVES,
            FeatureFlag::BULLETPROOFS_BATCH_NATIVES,
            FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE,
        ],
        vec![],
    );

    // Note: The confidential asset module is pre-initialized at genesis.

    // Create a test account with some balance
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());

    // Get APT token metadata address
    let apt_metadata = get_apt_metadata_address();

    // Create the register payload
    let payload = create_register_payload(apt_metadata, &RISTRETTO_BASEPOINT_COMPRESSED);

    // Run with gas profiler and get status
    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    // Print results
    println!("\n=== Confidential Asset Register Gas Profile ===");

    print_gas_cost("register", gas_used);

    if let Some(fee) = &fee_statement {
        println!("  Execution gas: {} units", fee.execution_gas_used());
        println!("  IO gas: {} units", fee.io_gas_used());
        println!("  Storage fee: {} octas", fee.storage_fee_used());
        println!("  Storage refund: {} octas", fee.storage_fee_refund());
    }

    // Generate detailed HTML report if requested
    if detailed {
        std::fs::create_dir_all("./gas-profiling").ok();
        gas_log
            .generate_html_report(
                "./gas-profiling/confidential_asset_register",
                "Confidential Asset Register Gas Report".to_string(),
            )
            .expect("failed to generate gas report");
        println!(
            "HTML report generated at: ./gas-profiling/confidential_asset_register/index.html"
        );
    }

    // Assert transaction succeeded
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "Transaction should succeed, but got: {:?}",
        status
    );

    // Basic sanity check - register should use some gas
    assert!(gas_used > 0, "Register should consume gas");

    println!("=== End Gas Profile ===\n");
}

/// Test that profiles gas usage for the confidential asset `register` function.
///
/// Run with:
/// ```bash
/// cargo test -p e2e-move-tests bench_gas_register --release -- --nocapture
/// ```
#[test]
fn bench_gas_register() {
    profile_confidential_asset_register(false);
}

/// Test that profiles gas usage with detailed HTML report generation.
///
/// Note: As confirmed by Victor, the report actually does not include the aggregated numbers from
/// `TransactiongGasLog` (e.g., execution gas, IO gas), so do not attempt to reconcile the two.
/// Nonetheless, the HTML report remains useful for its flamegraphs of gas usage.
///
/// Run with:
/// ```bash
/// cargo test -p e2e-move-tests bench_gas_register_detailed --release -- --nocapture --ignored
/// ```
#[test]
#[ignore] // Ignored by default since it writes files; run explicitly when needed
fn bench_gas_register_detailed() {
    profile_confidential_asset_register(true);
}

/// Test that we can call a private Move function using exec_function_bypass_visibility.
///
/// Run with:
/// ```bash
/// cargo test -p e2e-move-tests test_call_private_function --release -- --nocapture
/// ```
#[test]
fn test_call_private_function() {
    let mut h = MoveHarness::new();

    // Call the private function using the new bypass visibility method
    let return_values = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            "confidential_asset",
            "get_fa_store_address",
            vec![],
            vec![],
        )
        .unwrap();

    println!("\n=== Test Call Private Function ===");

    // The function returns a u64, so we expect one return value
    assert_eq!(return_values.return_values.len(), 1);
    let (bytes, _) = &return_values.return_values[0];
    let expected = AccountAddress::from_hex_literal(
        "0x5d35f41578f4cebfdc2c4ae38761b890950dfc3c24315e8b5bafd003e8165db9",
    )
    .unwrap();
    let value: AccountAddress = bcs::from_bytes(bytes).expect("Failed to deserialize u64");
    println!(
        "Successfully called private function! Returned value: {}",
        value
    );
    assert_eq!(value, expected, "Wrong address returned!");

    println!("=== End Test ===\n");
}
