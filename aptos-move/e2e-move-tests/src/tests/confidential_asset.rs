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

/// Auditor benchmarking mode.
#[cfg(feature = "move-harness-with-test-only")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum AuditorMode {
    /// No auditor is set for the asset type.
    NoAuditor,
    /// An auditor is set; this is the first audited operation (incurs one-time storage expansion).
    AuditorFirst,
    /// An auditor is set; a prior audited operation already paid the storage expansion cost.
    AuditorSubsequent,
}

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
        10_000_000_000,
    )
}

/// Creates a test account, funded with some APT, for Bob.
fn create_bob(h: &mut MoveHarness) -> Account {
    h.new_account_with_balance_at(
        AccountAddress::from_hex_literal(BOB_ADDRESS).unwrap(),
        10_000_000_000,
    )
}

/// Register an account for confidential assets using a real sigma protocol proof.
/// Takes the decryption key and encryption key bytes. Generates the sigma proof via Move test helper.
#[cfg(feature = "move-harness-with-test-only")]
fn register_account(
    h: &mut MoveHarness,
    account: &Account,
    token_metadata: AccountAddress,
    dk_bytes: &[u8],
    ek_bytes: &[u8],
) {
    // Generate real registration proof via Move test helper
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "prove_registration",
            vec![],
            vec![
                bcs::to_bytes(account.address()).unwrap(),
                bcs::to_bytes(&token_metadata).unwrap(),
                bcs::to_bytes(&dk_bytes.to_vec()).unwrap(),
            ],
        )
        .expect("prove_registration should succeed");

    assert_eq!(result.return_values.len(), 1);
    let proof: MoveRegistrationProof =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize RegistrationProof");
    let MoveRegistrationProof::V1 { sigma } = proof;
    let sigma_comm = extract_compressed_bytes(sigma.compressed_comm_a);
    let sigma_resp = extract_scalar_bytes(sigma.resp_sigma);

    let payload = create_register_payload(token_metadata, ek_bytes, sigma_comm, sigma_resp);
    let status = h.run_transaction_payload(account, payload);
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "Register for {} should succeed, but got: {:?}",
        account.address(),
        status
    );
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
            "get_global_config_address",
            vec![],
            vec![],
        )
        .expect("get_global_config_address should succeed");

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

/// Create a payload for the confidential_asset::register_raw entry function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_register_payload(
    token_metadata: AccountAddress,
    encryption_key: &[u8],
    sigma_proto_comm: Vec<Vec<u8>>,
    sigma_proto_resp: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("register_raw").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&encryption_key.to_vec()).unwrap(),
            bcs::to_bytes(&sigma_proto_comm).unwrap(),
            bcs::to_bytes(&sigma_proto_resp).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::deposit function.
#[cfg(feature = "move-harness-with-test-only")]
fn create_deposit_payload(token_metadata: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("deposit").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::rollover_pending_balance function.
#[cfg(feature = "move-harness-with-test-only")]
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

/// Create a payload for the confidential_asset::withdraw_to_raw entry function.
#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
fn create_withdraw_to_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
    new_balance_P_bytes: Vec<Vec<u8>>,
    new_balance_R_bytes: Vec<Vec<u8>>,
    new_balance_R_aud_bytes: Vec<Vec<u8>>,
    zkrp_new_balance_bytes: Vec<u8>,
    sigma_proto_comm: Vec<Vec<u8>>,
    sigma_proto_resp: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("withdraw_to_raw").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
            bcs::to_bytes(&new_balance_P_bytes).unwrap(),
            bcs::to_bytes(&new_balance_R_bytes).unwrap(),
            bcs::to_bytes(&new_balance_R_aud_bytes).unwrap(),
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&sigma_proto_comm).unwrap(),
            bcs::to_bytes(&sigma_proto_resp).unwrap(),
        ],
    ))
}

/// Create a payload for the confidential_asset::confidential_transfer_raw entry function.
#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
fn create_confidential_transfer_payload(
    token_metadata: AccountAddress,
    recipient: AccountAddress,
    new_balance_P_bytes: Vec<Vec<u8>>,
    new_balance_R_bytes: Vec<Vec<u8>>,
    new_balance_R_eff_aud_bytes: Vec<Vec<u8>>,
    amount_P_bytes: Vec<Vec<u8>>,
    amount_R_sender_bytes: Vec<Vec<u8>>,
    amount_R_recip_bytes: Vec<Vec<u8>>,
    amount_R_eff_aud_bytes: Vec<Vec<u8>>,
    ek_volun_auds: Vec<Vec<u8>>,
    amount_R_volun_auds_bytes: Vec<Vec<Vec<u8>>>,
    zkrp_new_balance_bytes: Vec<u8>,
    zkrp_transfer_amount_bytes: Vec<u8>,
    sigma_proto_comm: Vec<Vec<u8>>,
    sigma_proto_resp: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("confidential_transfer_raw").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&new_balance_P_bytes).unwrap(),
            bcs::to_bytes(&new_balance_R_bytes).unwrap(),
            bcs::to_bytes(&new_balance_R_eff_aud_bytes).unwrap(),
            bcs::to_bytes(&amount_P_bytes).unwrap(),
            bcs::to_bytes(&amount_R_sender_bytes).unwrap(),
            bcs::to_bytes(&amount_R_recip_bytes).unwrap(),
            bcs::to_bytes(&amount_R_eff_aud_bytes).unwrap(),
            bcs::to_bytes(&ek_volun_auds).unwrap(),
            bcs::to_bytes(&amount_R_volun_auds_bytes).unwrap(),
            bcs::to_bytes(&zkrp_new_balance_bytes).unwrap(),
            bcs::to_bytes(&zkrp_transfer_amount_bytes).unwrap(),
            bcs::to_bytes(&sigma_proto_comm).unwrap(),
            bcs::to_bytes(&sigma_proto_resp).unwrap(),
        ],
    ))
}

/// Serialize an address as a Move signer value for use with exec_function_bypass_visibility.
/// The signer BCS layout is an enum: variant 0 = single address.
#[cfg(feature = "move-harness-with-test-only")]
fn serialize_signer(address: AccountAddress) -> Vec<u8> {
    let mut bytes = vec![0u8]; // enum variant index 0
    bytes.extend_from_slice(&bcs::to_bytes(&address).unwrap());
    bytes
}

/// Set the auditor for a given asset type. Requires calling from the framework address.
#[cfg(feature = "move-harness-with-test-only")]
fn set_auditor_for_asset_type(
    h: &mut MoveHarness,
    token_metadata: AccountAddress,
    ek_aud_bytes: &[u8],
) {
    let ek_aud_option: Option<Vec<u8>> = Some(ek_aud_bytes.to_vec());
    h.exec_function_bypass_visibility(
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
        "set_auditor_for_asset_type",
        vec![],
        vec![
            serialize_signer(FRAMEWORK_ADDRESS),
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&ek_aud_option).unwrap(),
        ],
    )
    .expect("set_auditor_for_asset_type should succeed");
}

/// Create a payload for the confidential_asset::rotate_encryption_key_raw entry function.
#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
fn create_rotate_encryption_key_payload(
    token_metadata: AccountAddress,
    new_ek_bytes: Vec<u8>,
    unpause: bool,
    new_R_bytes: Vec<Vec<u8>>,
    sigma_proto_comm: Vec<Vec<u8>>,
    sigma_proto_resp: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("rotate_encryption_key_raw").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&token_metadata).unwrap(),
            bcs::to_bytes(&new_ek_bytes).unwrap(),
            bcs::to_bytes(&unpause).unwrap(),
            bcs::to_bytes(&new_R_bytes).unwrap(),
            bcs::to_bytes(&sigma_proto_comm).unwrap(),
            bcs::to_bytes(&sigma_proto_resp).unwrap(),
        ],
    ))
}

/// Create a payload for freezing a token (needed before rotate_encryption_key).
#[cfg(feature = "move-harness-with-test-only")]
fn create_rollover_and_freeze_payload(token_metadata: AccountAddress) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(EXPERIMENTAL_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        ident_str!("rollover_pending_balance_and_pause").to_owned(),
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
// BCS mirror types for deserializing Move proof structs
// =================================================================================================

// RistrettoPoint in Move is { handle: u64 } (a VM-internal handle, not actual point bytes).
// CompressedRistretto and Scalar are { data: Vec<u8> }.

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct MoveRistrettoPoint {
    handle: u64,
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
struct MoveCompressedRistretto {
    data: Vec<u8>,
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
struct MoveScalar {
    data: Vec<u8>,
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct MoveSigmaProof {
    comm_a: Vec<MoveRistrettoPoint>,
    compressed_comm_a: Vec<MoveCompressedRistretto>,
    resp_sigma: Vec<MoveScalar>,
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
struct MoveRangeProof {
    bytes: Vec<u8>,
}

#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
enum MoveCompressedAvailableBalance {
    V1 {
        P: Vec<MoveCompressedRistretto>,
        R: Vec<MoveCompressedRistretto>,
        R_aud: Vec<MoveCompressedRistretto>,
    },
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
enum MoveRegistrationProof {
    V1 { sigma: MoveSigmaProof },
}

#[cfg(feature = "move-harness-with-test-only")]
#[derive(serde::Deserialize)]
#[allow(dead_code)]
enum MoveWithdrawalProof {
    V1 {
        compressed_new_balance: MoveCompressedAvailableBalance,
        zkrp_new_balance: MoveRangeProof,
        sigma: MoveSigmaProof,
    },
}

#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct MoveCompressedAmount {
    compressed_P: Vec<MoveCompressedRistretto>,
    compressed_R_sender: Vec<MoveCompressedRistretto>,
    compressed_R_recip: Vec<MoveCompressedRistretto>,
    compressed_R_eff_aud: Vec<MoveCompressedRistretto>,
    compressed_R_volun_auds: Vec<Vec<MoveCompressedRistretto>>,
}

#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case, dead_code)]
#[derive(serde::Deserialize)]
enum MoveTransferProof {
    V1 {
        compressed_new_balance: MoveCompressedAvailableBalance,
        compressed_amount: MoveCompressedAmount,
        compressed_ek_volun_auds: Vec<MoveCompressedRistretto>,
        zkrp_new_balance: MoveRangeProof,
        zkrp_amount: MoveRangeProof,
        sigma: MoveSigmaProof,
    },
}

#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case, dead_code)]
#[derive(serde::Deserialize)]
enum MoveKeyRotationProof {
    V1 {
        compressed_new_ek: MoveCompressedRistretto,
        compressed_new_R: Vec<MoveCompressedRistretto>,
        sigma: MoveSigmaProof,
    },
}

/// Extract byte vectors from compressed Ristretto points.
#[cfg(feature = "move-harness-with-test-only")]
fn extract_compressed_bytes(points: Vec<MoveCompressedRistretto>) -> Vec<Vec<u8>> {
    points.into_iter().map(|p| p.data).collect()
}

/// Extract byte vectors from scalars.
#[cfg(feature = "move-harness-with-test-only")]
fn extract_scalar_bytes(scalars: Vec<MoveScalar>) -> Vec<Vec<u8>> {
    scalars.into_iter().map(|s| s.data).collect()
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

#[cfg(feature = "move-harness-with-test-only")]
/// Profile gas usage for the confidential asset `register` function.
fn profile_confidential_asset_register(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate keypair and registration proof
    let (dk_bytes, ek_bytes) = generate_keypair(&mut h);
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "prove_registration",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&dk_bytes).unwrap(),
            ],
        )
        .expect("prove_registration should succeed");

    assert_eq!(result.return_values.len(), 1);
    let proof: MoveRegistrationProof =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize RegistrationProof");
    let MoveRegistrationProof::V1 { sigma } = proof;
    let sigma_comm = extract_compressed_bytes(sigma.compressed_comm_a);
    let sigma_resp = extract_scalar_bytes(sigma.resp_sigma);

    let payload = create_register_payload(apt_metadata, &ek_bytes, sigma_comm, sigma_resp);

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
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_register() {
    profile_confidential_asset_register(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_register_detailed() {
    profile_confidential_asset_register(true);
}

#[cfg(feature = "move-harness-with-test-only")]
/// Profile gas usage for the confidential asset `deposit` function.
fn profile_confidential_asset_deposit(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Register Alice for confidential assets with a real keypair
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);

    // Record balance before deposit
    let alice_balance_before = h.read_aptos_balance(alice.address());

    // Benchmark deposit
    let deposit_amount = 1000u64;
    let payload = create_deposit_payload(apt_metadata, deposit_amount);

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    print_gas_cost(
        "deposit",
        gas_used,
        &fee_statement.unwrap(),
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
    );
    maybe_generate_html_report(detailed, &gas_log, "confidential_asset_deposit");

    assert_success(&status, "deposit");
    assert!(gas_used > 0, "deposit should consume gas");

    // Verify balance
    let alice_balance_after = h.read_aptos_balance(alice.address());

    assert!(
        alice_balance_before - alice_balance_after >= deposit_amount,
        "Alice's balance should decrease by at least the deposit amount. Before: {}, After: {}, Deposit: {}",
        alice_balance_before.separate_with_commas(), alice_balance_after.separate_with_commas(), deposit_amount.separate_with_commas()
    );

    println!(
        "  Alice balance: {} -> {} (deposited {})",
        alice_balance_before.separate_with_commas(),
        alice_balance_after.separate_with_commas(),
        deposit_amount.separate_with_commas()
    );
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_deposit() {
    profile_confidential_asset_deposit(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_deposit_detailed() {
    profile_confidential_asset_deposit(true);
}

#[cfg(feature = "move-harness-with-test-only")]
/// Profile gas usage for the confidential asset `rollover_pending_balance` function.
fn profile_confidential_asset_rollover_pending_balance(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Register Alice for confidential assets with a real keypair
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);

    // Deposit some tokens to Alice so she has a pending balance to rollover
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_payload(apt_metadata, deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit");

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
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_rollover_pending_balance() {
    profile_confidential_asset_rollover_pending_balance(false);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
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

/// Generate a withdrawal proof and build the transaction payload.
#[cfg(feature = "move-harness-with-test-only")]
fn prove_and_build_withdraw_to(
    h: &mut MoveHarness,
    alice: &Account,
    bob: &Account,
    apt_metadata: AccountAddress,
    alice_dk: &[u8],
    withdraw_amount: u64,
    new_balance_amount: u128,
) -> TransactionPayload {
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "prove_withdrawal",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk.to_vec()).unwrap(),
                bcs::to_bytes(&withdraw_amount).unwrap(),
                bcs::to_bytes(&new_balance_amount).unwrap(),
            ],
        )
        .expect("prove_withdrawal should succeed");

    assert_eq!(result.return_values.len(), 1);
    let proof: MoveWithdrawalProof =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize WithdrawalProof");
    let MoveWithdrawalProof::V1 {
        compressed_new_balance,
        zkrp_new_balance,
        sigma,
    } = proof;

    let MoveCompressedAvailableBalance::V1 {
        P: new_bal_p,
        R: new_bal_r,
        R_aud: new_bal_r_aud,
    } = compressed_new_balance;

    create_withdraw_to_payload(
        apt_metadata,
        *bob.address(),
        withdraw_amount,
        extract_compressed_bytes(new_bal_p),
        extract_compressed_bytes(new_bal_r),
        extract_compressed_bytes(new_bal_r_aud),
        zkrp_new_balance.bytes,
        extract_compressed_bytes(sigma.compressed_comm_a),
        extract_scalar_bytes(sigma.resp_sigma),
    )
}

/// Profile gas usage for the confidential asset `withdraw_to` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_withdraw_to(detailed: bool, auditor_mode: AuditorMode) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    if auditor_mode != AuditorMode::NoAuditor {
        let (_dk_aud, ek_aud) = generate_keypair(&mut h);
        set_auditor_for_asset_type(&mut h, apt_metadata, &ek_aud);
    }

    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (bob_dk, bob_ek) = generate_keypair(&mut h);
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);
    register_account(&mut h, &bob, apt_metadata, &bob_dk, &bob_ek);

    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_payload(apt_metadata, deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit");

    let rollover_payload = create_rollover_pending_balance_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_payload);
    assert_success(&status, "rollover_pending_balance");

    let mut remaining = deposit_amount;
    let withdraw_amount = 100u64;

    if auditor_mode == AuditorMode::AuditorSubsequent {
        remaining -= withdraw_amount;
        let payload = prove_and_build_withdraw_to(
            &mut h,
            &alice,
            &bob,
            apt_metadata,
            &alice_dk,
            withdraw_amount,
            remaining as u128,
        );
        let status = h.run_transaction_payload(&alice, payload);
        assert_success(&status, "withdraw_to (warmup)");
    }

    remaining -= withdraw_amount;
    let payload = prove_and_build_withdraw_to(
        &mut h,
        &alice,
        &bob,
        apt_metadata,
        &alice_dk,
        withdraw_amount,
        remaining as u128,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    let label = match auditor_mode {
        AuditorMode::NoAuditor => "withdraw_to",
        AuditorMode::AuditorFirst => "withdraw_to (with auditor, first time)",
        AuditorMode::AuditorSubsequent => "withdraw_to (with auditor, subsequent)",
    };
    print_gas_cost(
        label,
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
    profile_confidential_asset_withdraw_to(false, AuditorMode::NoAuditor);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_withdraw_to_with_auditor() {
    profile_confidential_asset_withdraw_to(false, AuditorMode::AuditorFirst);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_withdraw_to_with_auditor_subsequent() {
    profile_confidential_asset_withdraw_to(false, AuditorMode::AuditorSubsequent);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_withdraw_to_detailed() {
    profile_confidential_asset_withdraw_to(true, AuditorMode::NoAuditor);
}

/// Generate a transfer proof and build the transaction payload.
#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
fn prove_and_build_confidential_transfer(
    h: &mut MoveHarness,
    alice: &Account,
    bob: &Account,
    apt_metadata: AccountAddress,
    alice_dk: &[u8],
    transfer_amount: u64,
    new_balance_amount: u128,
    volun_auditor_eks: &[Vec<u8>],
) -> TransactionPayload {
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "prove_transfer",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(bob.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk.to_vec()).unwrap(),
                bcs::to_bytes(&transfer_amount).unwrap(),
                bcs::to_bytes(&new_balance_amount).unwrap(),
                bcs::to_bytes(&volun_auditor_eks.to_vec()).unwrap(),
            ],
        )
        .expect("prove_transfer should succeed");

    assert_eq!(result.return_values.len(), 1);
    let proof: MoveTransferProof =
        bcs::from_bytes(&result.return_values[0].0).expect("deserialize TransferProof");
    let MoveTransferProof::V1 {
        compressed_new_balance,
        compressed_amount,
        compressed_ek_volun_auds,
        zkrp_new_balance,
        zkrp_amount,
        sigma,
    } = proof;

    let ek_volun_auds_bytes: Vec<Vec<u8>> = compressed_ek_volun_auds
        .into_iter()
        .map(|p| p.data)
        .collect();

    let amount_R_volun_auds_bytes: Vec<Vec<Vec<u8>>> = compressed_amount
        .compressed_R_volun_auds
        .into_iter()
        .map(|rs| extract_compressed_bytes(rs))
        .collect();

    let MoveCompressedAvailableBalance::V1 {
        P: new_bal_P,
        R: new_bal_R,
        R_aud: new_bal_R_aud,
    } = compressed_new_balance;

    create_confidential_transfer_payload(
        apt_metadata,
        *bob.address(),
        extract_compressed_bytes(new_bal_P),
        extract_compressed_bytes(new_bal_R),
        extract_compressed_bytes(new_bal_R_aud),
        extract_compressed_bytes(compressed_amount.compressed_P),
        extract_compressed_bytes(compressed_amount.compressed_R_sender),
        extract_compressed_bytes(compressed_amount.compressed_R_recip),
        extract_compressed_bytes(compressed_amount.compressed_R_eff_aud),
        ek_volun_auds_bytes,
        amount_R_volun_auds_bytes,
        zkrp_new_balance.bytes,
        zkrp_amount.bytes,
        extract_compressed_bytes(sigma.compressed_comm_a),
        extract_scalar_bytes(sigma.resp_sigma),
    )
}

/// Profile gas usage for the confidential asset `confidential_transfer` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
fn profile_confidential_asset_confidential_transfer(
    detailed: bool,
    auditor_mode: AuditorMode,
    num_volun_auditors: u8,
) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let bob = create_bob(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    if auditor_mode != AuditorMode::NoAuditor {
        let (_dk_aud, ek_aud) = generate_keypair(&mut h);
        set_auditor_for_asset_type(&mut h, apt_metadata, &ek_aud);
    }

    let volun_auditor_eks: Vec<Vec<u8>> = (0..num_volun_auditors)
        .map(|_| {
            let (_dk, ek) = generate_keypair(&mut h);
            ek
        })
        .collect();

    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (bob_dk, bob_ek) = generate_keypair(&mut h);
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);
    register_account(&mut h, &bob, apt_metadata, &bob_dk, &bob_ek);

    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_payload(apt_metadata, deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit");

    let rollover_payload = create_rollover_pending_balance_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_payload);
    assert_success(&status, "rollover_pending_balance");

    let mut remaining = deposit_amount;
    let transfer_amount = 100u64;

    if auditor_mode == AuditorMode::AuditorSubsequent {
        remaining -= transfer_amount;
        let payload = prove_and_build_confidential_transfer(
            &mut h,
            &alice,
            &bob,
            apt_metadata,
            &alice_dk,
            transfer_amount,
            remaining as u128,
            &volun_auditor_eks,
        );
        let status = h.run_transaction_payload(&alice, payload);
        assert_success(&status, "confidential_transfer (warmup)");
    }

    remaining -= transfer_amount;
    let payload = prove_and_build_confidential_transfer(
        &mut h,
        &alice,
        &bob,
        apt_metadata,
        &alice_dk,
        transfer_amount,
        remaining as u128,
        &volun_auditor_eks,
    );

    let (status, gas_log, gas_used, fee_statement) =
        h.evaluate_gas_with_profiler_and_status(&alice, payload);

    let volun_label = if num_volun_auditors > 0 {
        format!(", {} volun", num_volun_auditors)
    } else {
        String::new()
    };
    let label = match auditor_mode {
        AuditorMode::NoAuditor => format!("confidential_transfer ({} volun)", num_volun_auditors),
        AuditorMode::AuditorFirst => {
            format!(
                "confidential_transfer (with auditor, first time{})",
                volun_label
            )
        },
        AuditorMode::AuditorSubsequent => {
            format!(
                "confidential_transfer (with auditor, subsequent{})",
                volun_label
            )
        },
    };
    print_gas_cost(
        &label,
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
    profile_confidential_asset_confidential_transfer(false, AuditorMode::NoAuditor, 0);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_1_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::NoAuditor, 1);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_2_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::NoAuditor, 2);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_3_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::NoAuditor, 3);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorFirst, 0);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_subsequent() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorSubsequent, 0);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_1_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorFirst, 1);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_1_volun_subsequent() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorSubsequent, 1);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_2_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorFirst, 2);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_2_volun_subsequent() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorSubsequent, 2);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_3_volun() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorFirst, 3);
}

#[test]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_with_auditor_3_volun_subsequent() {
    profile_confidential_asset_confidential_transfer(false, AuditorMode::AuditorSubsequent, 3);
}

#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn bench_gas_confidential_transfer_detailed() {
    profile_confidential_asset_confidential_transfer(true, AuditorMode::NoAuditor, 0);
}

/// Profile gas usage for the confidential asset `rotate_encryption_key` function.
/// This requires generating ZK proofs via test-only Move functions.
#[cfg(feature = "move-harness-with-test-only")]
#[allow(non_snake_case)]
fn profile_confidential_asset_rotate_encryption_key(detailed: bool) {
    let mut h = setup_harness();
    let alice = create_alice(&mut h);
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate valid keypairs for Alice (current and new)
    let (alice_dk, alice_ek) = generate_keypair(&mut h);
    let (new_dk, _new_ek) = generate_keypair(&mut h);

    // Register Alice for confidential assets
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);

    // Deposit some tokens and rollover to actual balance
    let deposit_amount = 1000u64;
    let deposit_payload = create_deposit_payload(apt_metadata, deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit");

    // Before key rotation, need to rollover and freeze
    let rollover_freeze_payload = create_rollover_and_freeze_payload(apt_metadata);
    let status = h.run_transaction_payload(&alice, rollover_freeze_payload);
    assert_success(&status, "rollover_pending_balance_and_freeze");

    // Generate rotation proof using test-only Move function.
    // Note: Scalar has the same BCS layout as Vec<u8>, so alice_dk/new_dk pass through directly.
    let result = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "prove_key_rotation",
            vec![],
            vec![
                bcs::to_bytes(alice.address()).unwrap(),
                bcs::to_bytes(&apt_metadata).unwrap(),
                bcs::to_bytes(&alice_dk).unwrap(),
                bcs::to_bytes(&new_dk).unwrap(),
            ],
        )
        .expect("prove_key_rotation should succeed");

    assert_eq!(result.return_values.len(), 1);
    let (bytes, _) = &result.return_values[0];
    let proof: MoveKeyRotationProof = bcs::from_bytes(bytes).expect("deserialize KeyRotationProof");
    let MoveKeyRotationProof::V1 {
        compressed_new_ek,
        compressed_new_R,
        sigma,
    } = proof;

    let new_ek_bytes = compressed_new_ek.data;
    let new_R_bytes = extract_compressed_bytes(compressed_new_R);
    let sigma_proto_comm = extract_compressed_bytes(sigma.compressed_comm_a);
    let sigma_proto_resp = extract_scalar_bytes(sigma.resp_sigma);

    // Create and execute the rotate_encryption_key transaction
    let payload = create_rotate_encryption_key_payload(
        apt_metadata,
        new_ek_bytes,
        true, // unpause
        new_R_bytes,
        sigma_proto_comm,
        sigma_proto_resp,
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

// =================================================================================================
// Tests: Miscellaneous
// =================================================================================================

/// Test that we can call a private Move function using exec_function_bypass_visibility.
#[test]
#[ignore]
fn test_call_private_function() {
    let mut h = setup_harness();

    let return_values = h
        .exec_function_bypass_visibility(
            EXPERIMENTAL_ADDRESS,
            MODULE_NAME,
            "get_global_config_address",
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
/// 5. Calls check_pending_balance_decrypts_to to verify the deposit succeeded
#[test]
#[ignore]
#[cfg(feature = "move-harness-with-test-only")]
fn call_test_only_function() {
    // Use harness with test-only code included
    let mut h = setup_harness();

    // Create Alice with some APT
    let alice = h.new_account_with_balance_at(
        AccountAddress::from_hex_literal(ALICE_ADDRESS).unwrap(),
        10_000_000_000, // 10 APT
    );

    // Set up the confidential store for APT
    let apt_metadata = set_up_confidential_store_for_apt(&mut h);

    // Generate a valid keypair for Alice
    let (alice_dk, alice_ek) = generate_keypair(&mut h);

    // Register Alice for confidential assets with real sigma proof
    register_account(&mut h, &alice, apt_metadata, &alice_dk, &alice_ek);
    println!("Alice registered for confidential assets");

    // Deposit 100 units to Alice's pending balance
    let deposit_amount: u64 = 100;
    let deposit_payload = create_deposit_payload(apt_metadata, deposit_amount);
    let status = h.run_transaction_payload(&alice, deposit_payload);
    assert_success(&status, "deposit");
    println!("Deposited {} to Alice's pending balance", deposit_amount);

    // Now call the #[test_only] function check_pending_balance_decrypts_to
    // This should succeed and return true since we just deposited `deposit_amount`
    let result = h.exec_function_bypass_visibility(
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
        "check_pending_balance_decrypts_to",
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
                "check_pending_balance_decrypts_to returned: {} (expected: true)",
                verified
            );
            assert!(
                verified,
                "check_pending_balance_decrypts_to should return true for correct amount"
            );
            println!("SUCCESS: #[test_only] function check_pending_balance_decrypts_to worked correctly!");
        },
        Err(e) => {
            panic!(
                "check_pending_balance_decrypts_to should succeed with proper setup, but got: {:?}",
                e
            );
        },
    }

    // Also verify that wrong amount returns false
    let wrong_amount: u64 = 999;
    let result = h.exec_function_bypass_visibility(
        EXPERIMENTAL_ADDRESS,
        MODULE_NAME,
        "check_pending_balance_decrypts_to",
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
                "check_pending_balance_decrypts_to with wrong amount returned: {} (expected: false)",
                verified
            );
            assert!(
                !verified,
                "check_pending_balance_decrypts_to should return false for wrong amount"
            );
        },
        Err(e) => {
            panic!(
                "check_pending_balance_decrypts_to should succeed (and return false), but got: {:?}",
                e
            );
        },
    }
}
