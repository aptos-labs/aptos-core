// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay-style audit harness for the `0x1::confidential_asset` Fiat-Shamir bug
//! fixed in commit `b4a2cac4b8`.
//!
//! On this branch, `sigma_protocol::verify` is the per-component verifier
//! (previously `verify_slow`); `verify_batched` is gone. Every accepted
//! confidential-asset proof now goes through the per-component residual check,
//! which is satisfied by honest σ regardless of the Fiat-Shamir variant in use,
//! but is NOT satisfied by σ forged via the β-aggregation soundness break.
//!
//! Given a NDJSON file produced by the indexer's gRPC `GetTransactions` API
//! filtered to txns emitting `0x1::confidential_asset::*` events
//! (see the sibling `confidential-assets-replay` repo's `fetch.sh`), this test
//! re-executes every user transaction in strict version order against an
//! in-memory `MoveHarness` running our framework. The first abort either
//!
//!   (a) names the version of a forged-proof exploit, or
//!   (b) reveals a state-divergence between mainnet and this harness that we
//!       need to reconcile (missing bootstrap step, missing builder, etc.).
//!
//! Run:
//!   TRANSACTIONS_JSONL=/abs/path/to/transactions.jsonl \
//!       cargo test -p e2e-move-tests replay_mainnet_audit_window -- --nocapture
//!
//! Each historical sender's account is created locally via
//! `MoveHarness::new_account_with_balance_at(addr, ...)`, which under the hood
//! calls `Account::new_genesis_account(addr)` and writes the GENESIS public
//! key as that address's auth_key in the harness's in-memory state. So we hold
//! a valid SK for every historical address regardless of who controlled that
//! address on mainnet — no framework patches or test-only account override
//! needed.

use crate::MoveHarness;
use aptos_language_e2e_tests::account::Account;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus},
};
use move_core_types::{ident_str, identifier::Identifier, language_storage::ModuleId};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
};

// =================================================================================================
// Constants
// =================================================================================================

const FRAMEWORK_ADDRESS: AccountAddress = AccountAddress::ONE;
const MODULE_NAME: &str = "confidential_asset";
const TRANSACTIONS_JSONL_ENV: &str = "TRANSACTIONS_JSONL";
/// Each historical sender we materialize in the harness gets this many octas.
/// Generous so we never have to refill mid-replay.
const BIG_BALANCE: u64 = 1_000_000_000_000_000;

/// The one and only governance proposal we expect to see resolved inside the
/// audit window. Proposal 188 enabled APT confidentiality on mainnet at version
/// 4993477229; we replicate its state change programmatically.
const EXPECTED_GOV_PROPOSAL_ID: u64 = 188;

// =================================================================================================
// Indexer protobuf-JSON deserialization
//
// One line of `transactions.jsonl` is one `Response`. The fields below are the
// subset we actually use; serde tolerates the rest of the payload silently.
// =================================================================================================

#[derive(Deserialize)]
struct Response {
    transactions: Option<Vec<Tx>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tx {
    version: String,
    #[serde(rename = "type")]
    tx_type: Option<String>,
    user: Option<UserTx>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserTx {
    request: UserRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRequest {
    sender: String,
    payload: Payload,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    #[serde(rename = "type")]
    payload_type: String,
    entry_function_payload: Option<EntryFnPayloadJson>,
    script_payload: Option<ScriptPayloadJson>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntryFnPayloadJson {
    entry_function_id_str: String,
    /// Each element of `arguments` is itself a JSON-encoded string. See parse
    /// helpers below for the actual encodings.
    arguments: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScriptPayloadJson {
    arguments: Vec<String>,
}

// =================================================================================================
// JSON-arg parsing helpers
//
// Each element of `Payload.entry_function_payload.arguments` is itself a JSON
// document encoded as a String (note: that's protobuf-JSON's choice, NOT how
// the REST API encodes arguments — REST gives `Vec<serde_json::Value>`).
// Examples of the per-argument shapes we encounter:
//
//   Move type                       inner JSON shape
//   ──────────────────────────      ─────────────────────────────────────────
//   Object<Metadata>                {"inner":"0xa"}
//   address                         "0x9c53...e97259"
//   vector<u8>                      "0xhex"
//   vector<vector<u8>>              ["0xhex1","0xhex2",...]
//   vector<vector<vector<u8>>>      [["0xhex"], ...]
//   u64                             "123"            (numeric string)
//   bool                            true | false
//
// Each helper here parses ONE such string, panicking with context on a bad
// input so we surface the offending value rather than silently misdecoding.
// =================================================================================================

fn parse_hex_to_bytes(s: &str, ctx: &str) -> Vec<u8> {
    let stripped = s
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("{}: expected hex string with 0x prefix, got {:?}", ctx, s));
    hex::decode(stripped).unwrap_or_else(|e| panic!("{}: hex-decode of {:?} failed: {}", ctx, s, e))
}

fn parse_address(arg: &str) -> AccountAddress {
    // arg is a JSON-encoded string like `"0xabc..."`
    let s: String = serde_json::from_str(arg)
        .unwrap_or_else(|e| panic!("parse_address: not a JSON string {:?}: {}", arg, e));
    AccountAddress::from_hex_literal(&s)
        .unwrap_or_else(|e| panic!("parse_address: bad address {:?}: {}", s, e))
}

fn parse_u64(arg: &str) -> u64 {
    // Numerics in indexer protobuf-JSON come through as JSON-quoted strings.
    let s: String = serde_json::from_str(arg)
        .unwrap_or_else(|e| panic!("parse_u64: not a JSON string {:?}: {}", arg, e));
    s.parse::<u64>()
        .unwrap_or_else(|e| panic!("parse_u64: not a u64 {:?}: {}", s, e))
}

fn parse_bool(arg: &str) -> bool {
    serde_json::from_str::<bool>(arg)
        .unwrap_or_else(|e| panic!("parse_bool: not a JSON bool {:?}: {}", arg, e))
}

fn parse_object_metadata_inner_address(arg: &str) -> AccountAddress {
    // arg is `{"inner":"0x..."}` — pull `inner` and parse as address.
    #[derive(Deserialize)]
    struct ObjectShape {
        inner: String,
    }
    let parsed: ObjectShape = serde_json::from_str(arg)
        .unwrap_or_else(|e| panic!("parse_object_metadata_inner_address: bad {:?}: {}", arg, e));
    AccountAddress::from_hex_literal(&parsed.inner).unwrap_or_else(|e| {
        panic!(
            "parse_object_metadata_inner_address: bad inner address {:?}: {}",
            parsed.inner, e
        )
    })
}

fn parse_vec_u8(arg: &str) -> Vec<u8> {
    // arg is `"0xhex"`
    let s: String = serde_json::from_str(arg)
        .unwrap_or_else(|e| panic!("parse_vec_u8: not a JSON string {:?}: {}", arg, e));
    parse_hex_to_bytes(&s, "parse_vec_u8")
}

fn parse_vec_vec_u8(arg: &str) -> Vec<Vec<u8>> {
    // arg is `["0xhex1","0xhex2",...]`
    let items: Vec<String> = serde_json::from_str(arg).unwrap_or_else(|e| {
        panic!(
            "parse_vec_vec_u8: not a JSON array of strings {:?}: {}",
            arg, e
        )
    });
    items
        .into_iter()
        .map(|s| parse_hex_to_bytes(&s, "parse_vec_vec_u8 element"))
        .collect()
}

fn parse_vec_vec_vec_u8(arg: &str) -> Vec<Vec<Vec<u8>>> {
    // arg is `[["0xhex"], ...]`
    let outer: Vec<Vec<String>> = serde_json::from_str(arg).unwrap_or_else(|e| {
        panic!(
            "parse_vec_vec_vec_u8: not a JSON array-of-arrays-of-strings {:?}: {}",
            arg, e
        )
    });
    outer
        .into_iter()
        .map(|inner: Vec<String>| -> Vec<Vec<u8>> {
            inner
                .into_iter()
                .map(|s: String| parse_hex_to_bytes(&s, "parse_vec_vec_vec_u8 inner element"))
                .collect()
        })
        .collect()
}

// =================================================================================================
// Per-function builders
//
// One builder per `public entry fun` in `0x1::confidential_asset`. Each one
// takes the args array as it appears in the indexer JSONL (signer omitted —
// the VM auto-prepends it), parses each element with the helpers above, and
// BCS-encodes them in the same order the on-chain Move signature declares.
// We panic on arity mismatch so a future framework change that adds a
// parameter trips this test loudly instead of silently mis-encoding.
// =================================================================================================

fn make_entry_payload(func_name: &str, args: Vec<Vec<u8>>) -> TransactionPayload {
    let func_ident = Identifier::new(func_name)
        .unwrap_or_else(|e| panic!("make_entry_payload: bad func {:?}: {}", func_name, e));
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(FRAMEWORK_ADDRESS, ident_str!(MODULE_NAME).to_owned()),
        func_ident,
        vec![], // none of the entry functions we replay use type parameters
        args,
    ))
}

fn check_arity(func: &str, args: &[String], expected: usize) {
    assert_eq!(
        args.len(),
        expected,
        "{}: expected {} args, got {}: {:?}",
        func,
        expected,
        args.len(),
        args
    );
}

fn build_register_raw(args: &[String]) -> TransactionPayload {
    check_arity("register_raw", args, 4);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let ek = parse_vec_u8(&args[1]);
    let sigma_proto_comm = parse_vec_vec_u8(&args[2]);
    let sigma_proto_resp = parse_vec_vec_u8(&args[3]);
    make_entry_payload("register_raw", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&ek).unwrap(),
        bcs::to_bytes(&sigma_proto_comm).unwrap(),
        bcs::to_bytes(&sigma_proto_resp).unwrap(),
    ])
}

fn build_deposit(args: &[String]) -> TransactionPayload {
    check_arity("deposit", args, 2);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let amount = parse_u64(&args[1]);
    make_entry_payload("deposit", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&amount).unwrap(),
    ])
}

#[allow(non_snake_case)]
fn build_withdraw_to_raw(args: &[String]) -> TransactionPayload {
    check_arity("withdraw_to_raw", args, 9);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let to = parse_address(&args[1]);
    let amount = parse_u64(&args[2]);
    let new_balance_P = parse_vec_vec_u8(&args[3]);
    let new_balance_R = parse_vec_vec_u8(&args[4]);
    let new_balance_R_aud = parse_vec_vec_u8(&args[5]);
    let zkrp_new_balance = parse_vec_u8(&args[6]);
    let sigma_proto_comm = parse_vec_vec_u8(&args[7]);
    let sigma_proto_resp = parse_vec_vec_u8(&args[8]);
    make_entry_payload("withdraw_to_raw", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&to).unwrap(),
        bcs::to_bytes(&amount).unwrap(),
        bcs::to_bytes(&new_balance_P).unwrap(),
        bcs::to_bytes(&new_balance_R).unwrap(),
        bcs::to_bytes(&new_balance_R_aud).unwrap(),
        bcs::to_bytes(&zkrp_new_balance).unwrap(),
        bcs::to_bytes(&sigma_proto_comm).unwrap(),
        bcs::to_bytes(&sigma_proto_resp).unwrap(),
    ])
}

#[allow(non_snake_case)]
fn build_confidential_transfer_raw(args: &[String]) -> TransactionPayload {
    check_arity("confidential_transfer_raw", args, 16);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let to = parse_address(&args[1]);
    let new_balance_P = parse_vec_vec_u8(&args[2]);
    let new_balance_R = parse_vec_vec_u8(&args[3]);
    let new_balance_R_eff_aud = parse_vec_vec_u8(&args[4]);
    let amount_P = parse_vec_vec_u8(&args[5]);
    let amount_R_sender = parse_vec_vec_u8(&args[6]);
    let amount_R_recip = parse_vec_vec_u8(&args[7]);
    let amount_R_eff_aud = parse_vec_vec_u8(&args[8]);
    let ek_volun_auds = parse_vec_vec_u8(&args[9]);
    let amount_R_volun_auds = parse_vec_vec_vec_u8(&args[10]);
    let zkrp_new_balance = parse_vec_u8(&args[11]);
    let zkrp_amount = parse_vec_u8(&args[12]);
    let sigma_proto_comm = parse_vec_vec_u8(&args[13]);
    let sigma_proto_resp = parse_vec_vec_u8(&args[14]);
    let memo = parse_vec_u8(&args[15]);
    make_entry_payload("confidential_transfer_raw", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&to).unwrap(),
        bcs::to_bytes(&new_balance_P).unwrap(),
        bcs::to_bytes(&new_balance_R).unwrap(),
        bcs::to_bytes(&new_balance_R_eff_aud).unwrap(),
        bcs::to_bytes(&amount_P).unwrap(),
        bcs::to_bytes(&amount_R_sender).unwrap(),
        bcs::to_bytes(&amount_R_recip).unwrap(),
        bcs::to_bytes(&amount_R_eff_aud).unwrap(),
        bcs::to_bytes(&ek_volun_auds).unwrap(),
        bcs::to_bytes(&amount_R_volun_auds).unwrap(),
        bcs::to_bytes(&zkrp_new_balance).unwrap(),
        bcs::to_bytes(&zkrp_amount).unwrap(),
        bcs::to_bytes(&sigma_proto_comm).unwrap(),
        bcs::to_bytes(&sigma_proto_resp).unwrap(),
        bcs::to_bytes(&memo).unwrap(),
    ])
}

#[allow(non_snake_case)]
fn build_rotate_encryption_key_raw(args: &[String]) -> TransactionPayload {
    check_arity("rotate_encryption_key_raw", args, 6);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let new_ek = parse_vec_u8(&args[1]);
    let resume_incoming_transfers = parse_bool(&args[2]);
    let new_R = parse_vec_vec_u8(&args[3]);
    let sigma_proto_comm = parse_vec_vec_u8(&args[4]);
    let sigma_proto_resp = parse_vec_vec_u8(&args[5]);
    make_entry_payload("rotate_encryption_key_raw", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&new_ek).unwrap(),
        bcs::to_bytes(&resume_incoming_transfers).unwrap(),
        bcs::to_bytes(&new_R).unwrap(),
        bcs::to_bytes(&sigma_proto_comm).unwrap(),
        bcs::to_bytes(&sigma_proto_resp).unwrap(),
    ])
}

#[allow(non_snake_case)]
fn build_normalize_raw(args: &[String]) -> TransactionPayload {
    check_arity("normalize_raw", args, 7);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let new_balance_P = parse_vec_vec_u8(&args[1]);
    let new_balance_R = parse_vec_vec_u8(&args[2]);
    let new_balance_R_aud = parse_vec_vec_u8(&args[3]);
    let zkrp_new_balance = parse_vec_u8(&args[4]);
    let sigma_proto_comm = parse_vec_vec_u8(&args[5]);
    let sigma_proto_resp = parse_vec_vec_u8(&args[6]);
    make_entry_payload("normalize_raw", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&new_balance_P).unwrap(),
        bcs::to_bytes(&new_balance_R).unwrap(),
        bcs::to_bytes(&new_balance_R_aud).unwrap(),
        bcs::to_bytes(&zkrp_new_balance).unwrap(),
        bcs::to_bytes(&sigma_proto_comm).unwrap(),
        bcs::to_bytes(&sigma_proto_resp).unwrap(),
    ])
}

fn build_rollover_pending_balance(args: &[String]) -> TransactionPayload {
    check_arity("rollover_pending_balance", args, 1);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    make_entry_payload("rollover_pending_balance", vec![
        bcs::to_bytes(&asset_type).unwrap()
    ])
}

fn build_rollover_pending_balance_and_pause(args: &[String]) -> TransactionPayload {
    check_arity("rollover_pending_balance_and_pause", args, 1);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    make_entry_payload("rollover_pending_balance_and_pause", vec![bcs::to_bytes(
        &asset_type,
    )
    .unwrap()])
}

fn build_set_incoming_transfers_paused(args: &[String]) -> TransactionPayload {
    check_arity("set_incoming_transfers_paused", args, 2);
    let asset_type = parse_object_metadata_inner_address(&args[0]);
    let paused = parse_bool(&args[1]);
    make_entry_payload("set_incoming_transfers_paused", vec![
        bcs::to_bytes(&asset_type).unwrap(),
        bcs::to_bytes(&paused).unwrap(),
    ])
}

fn build_payload(func_id: &str, args: &[String]) -> TransactionPayload {
    match func_id {
        "0x1::confidential_asset::register_raw" => build_register_raw(args),
        "0x1::confidential_asset::deposit" => build_deposit(args),
        "0x1::confidential_asset::withdraw_to_raw" => build_withdraw_to_raw(args),
        "0x1::confidential_asset::confidential_transfer_raw" => {
            build_confidential_transfer_raw(args)
        },
        "0x1::confidential_asset::rotate_encryption_key_raw" => {
            build_rotate_encryption_key_raw(args)
        },
        "0x1::confidential_asset::normalize_raw" => build_normalize_raw(args),
        "0x1::confidential_asset::rollover_pending_balance" => build_rollover_pending_balance(args),
        "0x1::confidential_asset::rollover_pending_balance_and_pause" => {
            build_rollover_pending_balance_and_pause(args)
        },
        "0x1::confidential_asset::set_incoming_transfers_paused" => {
            build_set_incoming_transfers_paused(args)
        },
        other => panic!(
            "build_payload: unknown entry function {} — add a builder for it",
            other
        ),
    }
}

// =================================================================================================
// Human-readable per-tx descriptions
//
// We print *only* public information: addresses, asset_type, plaintext
// amounts, encryption keys, booleans, and small public hex fields. We
// deliberately omit ciphertexts (P/R arrays), bulletproof bytes, and the
// sigma proof itself — those are large, unilluminating to a human reader, and
// already covered by the per-component verification that the harness does
// underneath. The intent is a "tail -f" friendly transcript of who did what.
// =================================================================================================

fn fmt_addr(a: AccountAddress) -> String {
    // Shortest 0x-prefixed form; consistent for special addresses like 0xa.
    a.to_hex_literal()
}

fn fmt_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[allow(non_snake_case)]
fn describe_entry_call(fn_id: &str, args: &[String]) -> String {
    match fn_id {
        "0x1::confidential_asset::register_raw" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            let ek = parse_vec_u8(&args[1]);
            format!(
                "register_raw(asset={}, ek={})",
                fmt_addr(asset),
                fmt_hex(&ek)
            )
        },
        "0x1::confidential_asset::deposit" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            let amount = parse_u64(&args[1]);
            format!("deposit(asset={}, amount={})", fmt_addr(asset), amount)
        },
        "0x1::confidential_asset::withdraw_to_raw" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            let to = parse_address(&args[1]);
            let amount = parse_u64(&args[2]);
            format!(
                "withdraw_to_raw(asset={}, to={}, amount={})",
                fmt_addr(asset),
                fmt_addr(to),
                amount
            )
        },
        "0x1::confidential_asset::confidential_transfer_raw" => {
            // amount is encrypted; only sender / recipient / asset / memo are public.
            let asset = parse_object_metadata_inner_address(&args[0]);
            let to = parse_address(&args[1]);
            let memo = parse_vec_u8(&args[15]);
            let n_volun_auds = parse_vec_vec_u8(&args[9]).len();
            format!(
                "confidential_transfer_raw(asset={}, to={}, memo_len={}, voluntary_auditors={})",
                fmt_addr(asset),
                fmt_addr(to),
                memo.len(),
                n_volun_auds
            )
        },
        "0x1::confidential_asset::rotate_encryption_key_raw" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            let new_ek = parse_vec_u8(&args[1]);
            let resume = parse_bool(&args[2]);
            format!(
                "rotate_encryption_key_raw(asset={}, new_ek={}, resume_incoming={})",
                fmt_addr(asset),
                fmt_hex(&new_ek),
                resume
            )
        },
        "0x1::confidential_asset::normalize_raw" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            format!("normalize_raw(asset={})", fmt_addr(asset))
        },
        "0x1::confidential_asset::rollover_pending_balance" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            format!("rollover_pending_balance(asset={})", fmt_addr(asset))
        },
        "0x1::confidential_asset::rollover_pending_balance_and_pause" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            format!(
                "rollover_pending_balance_and_pause(asset={})",
                fmt_addr(asset)
            )
        },
        "0x1::confidential_asset::set_incoming_transfers_paused" => {
            let asset = parse_object_metadata_inner_address(&args[0]);
            let paused = parse_bool(&args[1]);
            format!(
                "set_incoming_transfers_paused(asset={}, paused={})",
                fmt_addr(asset),
                paused
            )
        },
        other => format!("<unknown fn {}>", other),
    }
}

// =================================================================================================
// Harness bootstrap
// =================================================================================================

fn setup_harness() -> MoveHarness {
    let mut h = MoveHarness::new();
    // The registration / withdraw / transfer / rotation sigma protocols all bind
    // `chain_id::get()` into their Fiat-Shamir domain separator (see e.g.
    // `sigma_protocol_registration::assert_verifies`). Mainnet proofs were
    // computed against chain_id = 1; the harness's default chain_id is the
    // generic test value, so honest proofs would derive a different `e` here
    // and verify_slow would reject them. Forcing the on-chain ChainId resource
    // to mainnet makes the harness's Fiat-Shamir match the prover's.
    h.executor
        .state_store()
        .set_chain_id(ChainId::mainnet())
        .expect("set_chain_id to mainnet");
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

/// The signer BCS layout is an enum: variant 0 = single address.
fn serialize_signer(address: AccountAddress) -> Vec<u8> {
    let mut bytes = vec![0u8]; // enum variant index 0
    bytes.extend_from_slice(&bcs::to_bytes(&address).unwrap());
    bytes
}

/// Replicate proposal 188's effect (enable APT confidentiality). The on-chain
/// function `confidential_asset::set_confidentiality_for_apt` is `public` (not
/// `entry`) and requires `@aptos_framework`, so we call it via the harness's
/// visibility-bypass.
///
/// One subtlety: the on-chain init gates `allow_list_enabled` on
/// `chain_id == MAINNET || chain_id == TESTNET`. The harness's fake chain is
/// neither, so allow-listing defaults to disabled and any attempt to flip
/// `AssetConfig.allowed` aborts with `E_ALLOW_LISTING_IS_DISABLED`. We flip it
/// on first via `set_allow_listing`, then enable APT confidentiality.
fn bootstrap_apt_confidentiality(h: &mut MoveHarness) {
    h.exec_function_bypass_visibility(
        FRAMEWORK_ADDRESS,
        MODULE_NAME,
        "set_allow_listing",
        vec![],
        vec![
            serialize_signer(FRAMEWORK_ADDRESS),
            bcs::to_bytes(&true).unwrap(),
        ],
    )
    .expect("set_allow_listing should succeed");

    h.exec_function_bypass_visibility(
        FRAMEWORK_ADDRESS,
        MODULE_NAME,
        "set_confidentiality_for_apt",
        vec![],
        vec![
            serialize_signer(FRAMEWORK_ADDRESS),
            bcs::to_bytes(&true).unwrap(),
        ],
    )
    .expect("set_confidentiality_for_apt should succeed");
}

// =================================================================================================
// Replay loop
// =================================================================================================

fn ensure_account(
    h: &mut MoveHarness,
    accounts: &mut HashMap<AccountAddress, Account>,
    addr: AccountAddress,
) -> Account {
    accounts
        .entry(addr)
        .or_insert_with(|| h.new_account_with_balance_at(addr, BIG_BALANCE))
        .clone()
}

fn handle_script_payload(payload: &Payload, version: u64, script_payloads_seen: &mut u32) {
    // SCRIPT PAYLOADS
    //
    // Every confidential_asset user-facing operation is a `public entry` function
    // (register_raw, deposit, withdraw_to_raw, confidential_transfer_raw,
    // rotate_encryption_key_raw, normalize_raw, rollover_pending_balance,
    // rollover_pending_balance_and_pause, set_incoming_transfers_paused).
    // Normal users submit them as TYPE_ENTRY_FUNCTION_PAYLOAD transactions,
    // which the entry-function arm handles.
    //
    // Because they are `public entry` (not bare `entry`), a Move script could
    // in principle also call them — a custom user script could bundle multiple
    // ops and emit indistinguishable confidential_asset events. Empirically, no
    // such user scripts appear in our audit window: the only
    // TYPE_SCRIPT_PAYLOAD transactions in the event-filtered stream are
    // governance proposal resolutions (proposal 188 — APT confidentiality
    // enablement — at version 4993477229).
    //
    // We replicate proposal 188's state change programmatically in
    // `bootstrap_apt_confidentiality`, so when we see that specific script
    // here we SKIP its execution. Any other script payload — a different
    // proposal_id OR a user-submitted custom script — would cause silent
    // state-divergence vs mainnet. We refuse to proceed: abort and require
    // the developer to inspect the new script and either (a) expand the
    // bootstrap if it's another governance action, or (b) dispatch its
    // effects explicitly if it's a user multi-op script.
    *script_payloads_seen += 1;
    assert_eq!(*script_payloads_seen, 1,
       "v={}: encountered a second TYPE_SCRIPT_PAYLOAD; only the proposal-188 resolution is expected. Inspect this script and update the test to handle it.",
               version);

    let script = payload.script_payload.as_ref().unwrap_or_else(|| {
        panic!(
            "v={}: TYPE_SCRIPT_PAYLOAD but no scriptPayload field",
            version
        )
    });
    assert!(
        !script.arguments.is_empty(),
        "v={}: script payload has no arguments — cannot identify proposal_id",
        version
    );
    let proposal_id = parse_u64(&script.arguments[0]);
    assert_eq!(
        proposal_id, EXPECTED_GOV_PROPOSAL_ID,
        "v={}: expected proposal {} (APT confidentiality enablement), got {} \
         — update the test to handle this gov action.",
        version, EXPECTED_GOV_PROPOSAL_ID, proposal_id
    );
    // Script is the expected proposal-188 resolution; bootstrap_apt_confidentiality
    // already applied the equivalent state change. Skip execution.
    eprintln!(
        "[v={}] gov_script: resolve proposal {} (APT confidentiality enable) — skipped, bootstrap applied",
        version, proposal_id
    );
}

#[test]
fn replay_mainnet_audit_window() {
    let path = std::env::var(TRANSACTIONS_JSONL_ENV).unwrap_or_else(|_| {
        panic!(
            "Set {env} to the absolute path of transactions.jsonl. Example:\n\
             \n\
             {env}=$HOME/repos/confidential-assets-replay/transactions.jsonl \\\n\
                 cargo test -p e2e-move-tests replay_mainnet_audit_window -- --nocapture",
            env = TRANSACTIONS_JSONL_ENV
        )
    });

    let file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {}", path, e));
    let reader = BufReader::new(file);

    let mut h = setup_harness();
    bootstrap_apt_confidentiality(&mut h);

    let mut accounts: HashMap<AccountAddress, Account> = HashMap::new();
    let mut registered: HashSet<AccountAddress> = HashSet::new();
    let mut script_payloads_seen: u32 = 0;
    let mut entry_fn_replayed: u64 = 0;

    // `transactions.jsonl` is NDJSON (one Response per line) after fetch.sh's
    // jq-based heartbeat filter. Each Response has a `transactions` array; we
    // walk all of them in order.
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.unwrap_or_else(|e| panic!("read line {}: {}", line_no + 1, e));
        if line.trim().is_empty() {
            continue;
        }
        let resp: Response = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("parse line {}: {}", line_no + 1, e));
        let Some(txs) = resp.transactions else {
            continue;
        };
        for tx in txs {
            let v: u64 = tx
                .version
                .parse()
                .unwrap_or_else(|e| panic!("bad version {:?}: {}", tx.version, e));

            // Only user transactions appear in the event-filtered stream.
            if let Some(t) = &tx.tx_type {
                assert!(
                    t == "TRANSACTION_TYPE_USER",
                    "v={}: expected TRANSACTION_TYPE_USER, got {:?}",
                    v,
                    t
                );
            }
            let user = tx
                .user
                .unwrap_or_else(|| panic!("v={}: missing user sub-object", v));

            match user.request.payload.payload_type.as_str() {
                "TYPE_SCRIPT_PAYLOAD" => {
                    handle_script_payload(&user.request.payload, v, &mut script_payloads_seen);
                },
                "TYPE_ENTRY_FUNCTION_PAYLOAD" => {
                    let efp = user
                        .request
                        .payload
                        .entry_function_payload
                        .as_ref()
                        .unwrap_or_else(|| {
                            panic!(
                                "v={}: TYPE_ENTRY_FUNCTION_PAYLOAD but no entryFunctionPayload",
                                v
                            )
                        });
                    let sender = AccountAddress::from_hex_literal(&user.request.sender)
                        .unwrap_or_else(|e| {
                            panic!("v={}: bad sender {:?}: {}", v, user.request.sender, e)
                        });
                    let acc = ensure_account(&mut h, &mut accounts, sender);
                    eprintln!(
                        "[v={}] {} {}",
                        v,
                        fmt_addr(sender),
                        describe_entry_call(&efp.entry_function_id_str, &efp.arguments),
                    );
                    let payload = build_payload(&efp.entry_function_id_str, &efp.arguments);
                    let status = h.run_transaction_payload(&acc, payload);

                    // The first sigma-protocol verification failure here either
                    //   (a) names the version of a forged-proof exploit on the audit window, OR
                    //   (b) reveals a state-divergence between mainnet and this harness that we
                    //       need to reconcile (e.g., a state-changing event we forgot to apply,
                    //       a missing entry-function builder, or a wrong arg encoding).
                    // In either case we abort immediately and surface the version, so the human
                    // auditor can inspect the actual on-chain tx and reproduce.
                    assert!(
                        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
                        "REPLAY FAILED at version {} (fn {}): {:?}",
                        v,
                        efp.entry_function_id_str,
                        status
                    );
                    if efp.entry_function_id_str == "0x1::confidential_asset::register_raw" {
                        registered.insert(sender);
                    }
                    entry_fn_replayed += 1;
                },
                other => panic!(
                    "v={}: unknown payload type {:?}; extend the test to dispatch it",
                    v, other
                ),
            }
        }
    }

    // Total = entry-function txns we re-executed + governance scripts we
    // encountered-but-skipped. This should match the total event count
    // produced by `histogram.sh` over the same JSONL (each confidential-asset
    // tx in our window emits exactly one event).
    let total = entry_fn_replayed + script_payloads_seen as u64;
    eprintln!(
        "[replay] OK: {} txn(s) in audit window = {} entry-function replayed + {} governance script(s) skipped",
        total, entry_fn_replayed, script_payloads_seen
    );

    let mut registered_sorted: Vec<AccountAddress> = registered.into_iter().collect();
    registered_sorted.sort();
    eprintln!(
        "[replay] {} address(es) registered for confidential assets:",
        registered_sorted.len()
    );
    for addr in &registered_sorted {
        eprintln!("  0x{}", addr.to_hex());
    }
}
