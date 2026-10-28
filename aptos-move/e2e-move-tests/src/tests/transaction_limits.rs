// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for staking-backed transaction limits.

use crate::{
    assert_success, assert_vm_status,
    stake::{initialize_staking, join_validator_set, rotate_consensus_key, setup_staking},
    tests::common,
    MoveHarness,
};
use aptos_cached_packages::aptos_stdlib::{
    aptos_account_transfer, aptos_coin_transfer, delegation_pool_add_stake,
    delegation_pool_initialize_delegation_pool, stake_set_delegated_voter,
};
use aptos_crypto::HashValue;
use aptos_gas_algebra::Gas;
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_49;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    error::{split_canonical, OUT_OF_RANGE},
    fee_statement::FeeStatement,
    move_utils::MemberId,
    on_chain_config::{ApprovedExecutionHashes, FeatureFlag, OnChainConfig},
    transaction::{
        validation::ECANT_PAY_GAS_DEPOSIT, EntryFunction, ExecutionStatus, RequestedMultipliers,
        Script, SignedTransaction, TransactionExecutable, TransactionExtraConfig,
        TransactionOutput, TransactionPayload, TransactionPayloadInner, TransactionStatus,
        UserTxnLimitsRequest,
    },
};
use move_core_types::{
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const DEFAULT_GAS_UNIT_PRICE: u64 = 100;

// Default balance is 1M APT.
const DEFAULT_BALANCE: u64 = 100_000_000_000_000;
// Default stake amount is 0.25 APT.
const DEFAULT_STAKE: u64 = 25_000_000;

// Minimum stake for delegation 20 APT (has to be above 10 APT)
const DEFAULT_DELEGATION_STAKE: u64 = 2_000_000_000;

// Default tiers: 0.1 APT, 1 APT and 5 APT.
const DEFAULT_EXECUTION_TIERS: [(u64, u64); 3] =
    [(10_000_000, 200), (100_000_000, 400), (500_000_000, 800)];

// Default tiers: 0.2 APT, 2 APT and 10 APT.
const DEFAULT_IO_TIERS: [(u64, u64); 3] =
    [(20_000_000, 200), (200_000_000, 400), (1_000_000_000, 800)];

fn stake_pool_owner(
    execution_multiplier_percent: u64,
    io_multiplier_percent: u64,
) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::StakePoolOwner {
        multipliers: RequestedMultipliers::V1 {
            execution_multiplier_percent,
            io_multiplier_percent,
        },
    }
}

fn delegated_voter(
    pool: AccountAddress,
    execution_multiplier_percent: u64,
    io_multiplier_percent: u64,
) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::DelegatedVoter {
        pool_address: pool,
        multipliers: RequestedMultipliers::V1 {
            execution_multiplier_percent,
            io_multiplier_percent,
        },
    }
}

fn delegation_pool_delegator(
    pool: AccountAddress,
    execution_multiplier_percent: u64,
    io_multiplier_percent: u64,
) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::DelegationPoolDelegator {
        pool_address: pool,
        multipliers: RequestedMultipliers::V1 {
            execution_multiplier_percent,
            io_multiplier_percent,
        },
    }
}

fn entry_function_with_limits(
    entry_func: EntryFunction,
    request: UserTxnLimitsRequest,
) -> TransactionPayload {
    TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::EntryFunction(entry_func),
        extra_config: TransactionExtraConfig::V2 {
            multisig_address: None,
            replay_protection_nonce: None,
            txn_limits_request: Some(request),
        },
    })
}

fn payload_with_limits(
    sender_addr: AccountAddress,
    request: UserTxnLimitsRequest,
) -> TransactionPayload {
    match aptos_coin_transfer(sender_addr, 0) {
        TransactionPayload::EntryFunction(entry_func) => {
            entry_function_with_limits(entry_func, request)
        },
        _ => unreachable!(),
    }
}

fn sign_txn_with_limits_and_gas_unit_price(
    h: &mut MoveHarness,
    acc: &Account,
    request: UserTxnLimitsRequest,
    gas_unit_price: u64,
) -> SignedTransaction {
    h.create_transaction_without_sign(acc, payload_with_limits(*acc.address(), request))
        .gas_unit_price(gas_unit_price)
        .sign()
}

fn sign_txn_with_limits(
    h: &mut MoveHarness,
    acc: &Account,
    request: UserTxnLimitsRequest,
) -> SignedTransaction {
    sign_txn_with_limits_and_gas_unit_price(h, acc, request, DEFAULT_GAS_UNIT_PRICE)
}

fn sign_fee_payer_txn_with_limits(
    h: &mut MoveHarness,
    sender: &Account,
    fee_payer: &Account,
    request: UserTxnLimitsRequest,
) -> SignedTransaction {
    TransactionBuilder::new(sender.clone())
        .fee_payer(fee_payer.clone())
        .payload(payload_with_limits(*sender.address(), request))
        .sequence_number(h.sequence_number(sender.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(DEFAULT_GAS_UNIT_PRICE)
        .sign_fee_payer()
}

// Mirrors `0x1::staking_config::StakingConfig` for BCS serialization.
#[derive(Deserialize, Serialize)]
struct StakingConfig {
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
}

fn staking_config_struct_tag() -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("staking_config").to_owned(),
        name: ident_str!("StakingConfig").to_owned(),
        type_args: vec![],
    }
}

// Mirrors `0x1::transaction_limits::TxnLimitTier` for BCS serialization.
#[derive(Serialize)]
struct TxnLimitTier {
    min_stake: u64,
    multiplier_percent: u64,
}

// Mirrors `0x1::transaction_limits::TxnLimitsConfig` for BCS serialization.
#[derive(Serialize)]
enum TxnLimitsConfig {
    V1 {
        execution_tiers: Vec<TxnLimitTier>,
        io_tiers: Vec<TxnLimitTier>,
    },
}

fn to_tiers(tiers: &[(u64, u64)]) -> Vec<TxnLimitTier> {
    tiers
        .iter()
        .map(|(min_stake, multiplier_percent)| TxnLimitTier {
            min_stake: *min_stake,
            multiplier_percent: *multiplier_percent,
        })
        .collect()
}

fn new_test_harness() -> MoveHarness {
    new_test_harness_with_tiers(&DEFAULT_EXECUTION_TIERS, &DEFAULT_IO_TIERS)
}

fn new_test_harness_with_tiers(execution: &[(u64, u64)], io: &[(u64, u64)]) -> MoveHarness {
    let mut h = MoveHarness::new();
    let config = TxnLimitsConfig::V1 {
        execution_tiers: to_tiers(execution),
        io_tiers: to_tiers(io),
    };
    let struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("transaction_limits").to_owned(),
        name: ident_str!("TxnLimitsConfig").to_owned(),
        type_args: vec![],
    };
    h.set_resource(AccountAddress::ONE, struct_tag, &config);
    h
}

fn setup_validator(h: &mut MoveHarness) -> Account {
    let acc = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    assert_success!(setup_staking(h, &acc, DEFAULT_STAKE));
    acc
}

fn setup_stake_pool_without_joining_validator_set(h: &mut MoveHarness) -> Account {
    let acc = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let address = *acc.address();
    assert_success!(initialize_staking(h, &acc, DEFAULT_STAKE, address, address));
    acc
}

fn setup_delegation_pool_without_joining_validator_set(
    h: &mut MoveHarness,
    pool_owner: &Account,
    delegator: &Account,
    stake_amount: u64,
) -> AccountAddress {
    assert_success!(h.run_transaction_payload(
        pool_owner,
        delegation_pool_initialize_delegation_pool(0, vec![])
    ));

    let results = h
        .execute_view_function(
            MemberId::from_str("0x1::delegation_pool::get_owned_pool_address").unwrap(),
            vec![],
            vec![bcs::to_bytes(&pool_owner.address()).unwrap()],
        )
        .values
        .unwrap();
    let pool_address = bcs::from_bytes::<AccountAddress>(&results[0]).unwrap();

    assert_success!(h.run_transaction_payload(
        delegator,
        delegation_pool_add_stake(pool_address, stake_amount)
    ));
    pool_address
}

fn setup_delegation_pool(
    h: &mut MoveHarness,
    pool_owner: &Account,
    delegator: &Account,
    stake_amount: u64,
) -> AccountAddress {
    // The genesis sets a small voting-power-increase cap; a 20 APT delegation
    // pool would exceed it in a single epoch. Raise the cap so the pool can
    // join the validator set. The protocol limit is enforced by
    // `staking_config::update_voting_power_increase_limit`, but we are writing
    // the resource directly here, which bypasses that check.
    let mut staking_config: StakingConfig = h
        .read_resource(&AccountAddress::ONE, staking_config_struct_tag())
        .unwrap();
    staking_config.voting_power_increase_limit = 1_000_000;
    h.set_resource(
        AccountAddress::ONE,
        staking_config_struct_tag(),
        &staking_config,
    );

    let pool_address =
        setup_delegation_pool_without_joining_validator_set(h, pool_owner, delegator, stake_amount);
    assert_success!(rotate_consensus_key(h, pool_owner, pool_address));
    assert_success!(join_validator_set(h, pool_owner, pool_address));
    pool_address
}

fn run_and_assert_discard(h: &mut MoveHarness, txn: SignedTransaction, code: StatusCode) {
    let status = h.run(txn);
    assert_eq!(status, TransactionStatus::Discard(code));
}

#[test]
fn test_high_limit_txn_gas_price_too_low() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    // In test builds, gas price parameters default to 0. Set them so the
    // high-limit threshold (10x the base) is meaningful.
    h.modify_gas_schedule(|params| {
        params.vm.txn.min_price_per_gas_unit = DEFAULT_GAS_UNIT_PRICE.into();
        params.vm.txn.high_limit_txn_min_price_per_gas_unit = (10 * DEFAULT_GAS_UNIT_PRICE).into();
    });

    // Gas unit price of DEFAULT_GAS_UNIT_PRICE satisfies the normal minimum
    // but is below the 10x scaled minimum required for high-limit transactions.
    let txn = sign_txn_with_limits_and_gas_unit_price(
        &mut h,
        &acc,
        stake_pool_owner(200, 200),
        DEFAULT_GAS_UNIT_PRICE,
    );
    run_and_assert_discard(
        &mut h,
        txn,
        StatusCode::HIGH_LIMIT_TXN_GAS_UNIT_PRICE_BELOW_MIN_BOUND,
    );
}

#[test]
fn test_feature_disabled() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::TRANSACTION_LIMITS]);
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    run_and_assert_discard(&mut h, txn, StatusCode::FEATURE_UNDER_GATING);
}

#[test]
fn test_invalid_multiplier_1x() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(100, 100));
    run_and_assert_discard(&mut h, txn, StatusCode::INVALID_HIGH_TXN_LIMITS_MULTIPLIER);
}

#[test]
fn test_invalid_multiplier_larger_than_100x() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(100, 10001));
    run_and_assert_discard(&mut h, txn, StatusCode::INVALID_HIGH_TXN_LIMITS_MULTIPLIER);
}

#[test]
fn test_multiplier_not_available() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(900, 900));
    run_and_assert_discard(&mut h, txn, StatusCode::MULTIPLIER_NOT_AVAILABLE);
}

#[test]
fn test_stake_pool_owner_success() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    assert_success!(h.run(txn));
}

#[test]
fn test_not_stake_pool_owner() {
    let mut h = new_test_harness();
    let acc = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    run_and_assert_discard(&mut h, txn, StatusCode::NOT_STAKE_POOL_OWNER);
}

#[test]
fn test_stake_pool_owner_not_enough_stake() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(800, 800));
    run_and_assert_discard(&mut h, txn, StatusCode::INSUFFICIENT_STAKE);
}

#[test]
fn test_stake_pool_owner_not_enough_stake_for_io() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    assert_success!(h.run(txn));

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 400));
    run_and_assert_discard(&mut h, txn, StatusCode::INSUFFICIENT_STAKE);
}

#[test]
fn test_delegated_voter() {
    let mut h = new_test_harness();
    let pool_owner = setup_validator(&mut h);
    let voter = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let status =
        h.run_transaction_payload(&pool_owner, stake_set_delegated_voter(*voter.address()));
    assert_success!(status);
    h.new_epoch();

    let txn = sign_txn_with_limits(
        &mut h,
        &voter,
        delegated_voter(*pool_owner.address(), 200, 200),
    );
    assert_success!(h.run(txn));
}

#[test]
fn test_not_delegated_voter() {
    let mut h = new_test_harness();
    let pool_owner = setup_validator(&mut h);
    h.new_epoch();

    let acc = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let txn = sign_txn_with_limits(
        &mut h,
        &acc,
        delegated_voter(*pool_owner.address(), 200, 200),
    );
    run_and_assert_discard(&mut h, txn, StatusCode::NOT_DELEGATED_VOTER);
}

#[test]
fn test_stake_pool_owner_not_in_validator_set() {
    let mut h = new_test_harness();
    let acc = setup_stake_pool_without_joining_validator_set(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    run_and_assert_discard(&mut h, txn, StatusCode::STAKE_POOL_NOT_IN_VALIDATOR_SET);
}

#[test]
fn test_delegation_pool_not_found() {
    let mut h = new_test_harness();
    let delegator = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let fake_pool = AccountAddress::from_hex_literal("0xdead").unwrap();

    let txn = sign_txn_with_limits(
        &mut h,
        &delegator,
        delegation_pool_delegator(fake_pool, 200, 200),
    );
    run_and_assert_discard(&mut h, txn, StatusCode::DELEGATION_POOL_NOT_FOUND);
}

#[test]
fn test_delegation_pool_not_in_validator_set() {
    let mut h = new_test_harness();
    let pool_owner = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let delegator = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let pool_addr = setup_delegation_pool_without_joining_validator_set(
        &mut h,
        &pool_owner,
        &delegator,
        DEFAULT_DELEGATION_STAKE,
    );
    h.new_epoch();

    let txn = sign_txn_with_limits(
        &mut h,
        &delegator,
        delegation_pool_delegator(pool_addr, 200, 200),
    );
    run_and_assert_discard(&mut h, txn, StatusCode::STAKE_POOL_NOT_IN_VALIDATOR_SET);
}

#[test]
fn test_delegation_pool_delegator_success() {
    let mut h = new_test_harness();
    let pool_owner = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let delegator = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let pool_addr =
        setup_delegation_pool(&mut h, &pool_owner, &delegator, DEFAULT_DELEGATION_STAKE);
    h.new_epoch();

    let txn = sign_txn_with_limits(
        &mut h,
        &delegator,
        delegation_pool_delegator(pool_addr, 200, 200),
    );
    assert_success!(h.run(txn));
}

#[test]
fn test_delegation_pool_zero_stake() {
    let mut h = new_test_harness();
    let pool_owner = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let delegator = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let impostor = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let pool_addr =
        setup_delegation_pool(&mut h, &pool_owner, &delegator, DEFAULT_DELEGATION_STAKE);
    h.new_epoch();

    let txn = sign_txn_with_limits(
        &mut h,
        &impostor,
        delegation_pool_delegator(pool_addr, 200, 200),
    );
    run_and_assert_discard(&mut h, txn, StatusCode::INSUFFICIENT_STAKE);
}

#[test]
fn test_delegation_pool_insufficient_stake() {
    let execution = [
        DEFAULT_EXECUTION_TIERS[0],
        DEFAULT_EXECUTION_TIERS[1],
        (50_0000_0000, 800),
    ];
    let io = [
        DEFAULT_IO_TIERS[0],
        DEFAULT_IO_TIERS[1],
        (50_0000_0000, 800),
    ];
    let mut h = new_test_harness_with_tiers(&execution, &io);

    let pool_owner = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let delegator = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let pool_addr =
        setup_delegation_pool(&mut h, &pool_owner, &delegator, DEFAULT_DELEGATION_STAKE);
    h.new_epoch();

    let txn = sign_txn_with_limits(
        &mut h,
        &delegator,
        delegation_pool_delegator(pool_addr, 800, 800),
    );
    run_and_assert_discard(&mut h, txn, StatusCode::INSUFFICIENT_STAKE);
}

#[test]
fn test_fee_payer_provides_stake() {
    let mut h = new_test_harness();
    let sender = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    let fee_payer = setup_validator(&mut h);
    h.new_epoch();

    let txn =
        sign_fee_payer_txn_with_limits(&mut h, &sender, &fee_payer, stake_pool_owner(200, 200));
    assert_success!(h.run(txn));
}

#[test]
fn test_sender_stake_ignored_for_fee_payer_txn() {
    let mut h = new_test_harness();
    let sender = setup_validator(&mut h);
    let fee_payer = h.new_account_with_balance_and_sequence_number(DEFAULT_BALANCE, 0);
    h.new_epoch();

    let txn =
        sign_fee_payer_txn_with_limits(&mut h, &sender, &fee_payer, stake_pool_owner(200, 200));
    run_and_assert_discard(&mut h, txn, StatusCode::NOT_STAKE_POOL_OWNER);
}

#[test]
fn test_approved_gov_script_with_txn_limits_request_rejected() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    // A minimal valid script: the actual content does not matter as long as
    // the hash is in the approved list.
    let script_code = vec![0xA1, 0x1C, 0xEB, 0x0B, 0x06, 0x00, 0x00, 0x00];
    let script_hash = HashValue::sha3_256_of(&script_code).to_vec();
    let aptos_framework_addr = *h.aptos_framework_account().address();
    h.set_resource(
        aptos_framework_addr,
        ApprovedExecutionHashes::struct_tag(),
        &ApprovedExecutionHashes {
            entries: vec![(0, script_hash)],
        },
    );

    let script = Script::new(script_code, vec![], vec![]);
    let payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::Script(script),
        extra_config: TransactionExtraConfig::V2 {
            multisig_address: None,
            replay_protection_nonce: None,
            txn_limits_request: Some(stake_pool_owner(200, 200)),
        },
    });
    let txn = TransactionBuilder::new(acc.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(acc.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(DEFAULT_GAS_UNIT_PRICE)
        .sign();
    run_and_assert_discard(
        &mut h,
        txn,
        StatusCode::TXN_LIMITS_REQUEST_NOT_ALLOWED_FOR_GOVERNANCE_SCRIPT,
    );
}

const EXCESS_PRICE_MULTIPLIER: u64 = 10;

// Iterations of `0xbeef::limits::spin` used to set up execution gas above the
// base limit.
const SPIN_ITERATIONS: u64 = 50_000;

/// The base execution and IO limits, in gas units.
struct BaseLimits {
    execution: u64,
    io: u64,
}

fn base_limits(h: &MoveHarness) -> BaseLimits {
    let (_, params) = h.get_gas_params();
    let txn = &params.vm.txn;
    let execution: Gas = txn.max_execution_gas.to_unit_round_up_with_params(txn);
    let io: Gas = txn.max_io_gas.to_unit_round_up_with_params(txn);
    BaseLimits {
        execution: execution.into(),
        io: io.into(),
    }
}

fn beef_entry_function(name: &str, args: Vec<Vec<u8>>) -> EntryFunction {
    EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal("0xbeef").unwrap(),
            ident_str!("limits").to_owned(),
        ),
        Identifier::new(name).unwrap(),
        vec![],
        args,
    )
}

fn spin(n: u64) -> EntryFunction {
    beef_entry_function("spin", vec![bcs::to_bytes(&n).unwrap()])
}

fn spin_then_transfer(to: AccountAddress, amount: u64, n: u64) -> EntryFunction {
    beef_entry_function("spin_then_transfer", vec![
        bcs::to_bytes(&to).unwrap(),
        bcs::to_bytes(&amount).unwrap(),
        bcs::to_bytes(&n).unwrap(),
    ])
}

/// Runs the transaction. Returns its status, fee statement, and how much the
/// payer's balance dropped. `FeeStatement.gas_used()` is the amount the
/// transaction is actually billed against (for a high-limit request, that
/// already includes the excess surcharge), so it must never exceed the
/// transaction's own signed `max_gas_amount`.
fn run_and_get_fee(
    h: &mut MoveHarness,
    payer: &Account,
    txn: SignedTransaction,
) -> (TransactionStatus, FeeStatement, u64) {
    let max_gas_amount = txn.max_gas_amount();
    let before = h.read_aptos_balance(payer.address());
    let output: TransactionOutput = h.run_raw(txn);
    let fee_statement = output
        .try_extract_fee_statement()
        .expect("fee statement should decode")
        .expect("kept transaction should emit a fee statement");
    assert!(
        fee_statement.gas_used() <= max_gas_amount,
        "gas_used {} exceeds the signed max_gas_amount {}",
        fee_statement.gas_used(),
        max_gas_amount,
    );
    let paid = before - h.read_aptos_balance(payer.address());
    (output.status().clone(), fee_statement, paid)
}

/// The fee actually charged, and the excess gas units over the base limits.
/// For a high-limit request, `fee_statement.gas_used()` already includes the
/// excess surcharge, so `price * gas_used - refund` is the real fee with no
/// further adjustment.
fn fee_and_excess(fee_statement: &FeeStatement, base: &BaseLimits) -> (u64, u64) {
    let excess = (fee_statement
        .execution_gas_used()
        .saturating_sub(base.execution)
        + fee_statement.io_gas_used().saturating_sub(base.io))
    .min(fee_statement.gas_used());
    let fee =
        DEFAULT_GAS_UNIT_PRICE * fee_statement.gas_used() - fee_statement.storage_fee_refund();
    (fee, excess)
}

/// Sets up a validator and publishes `0xbeef::limits`. Then sets the base
/// execution limit to two thirds of what `spin(SPIN_ITERATIONS)` uses, so a 2x
/// request runs it with excess execution gas.
fn setup_excess_execution(h: &mut MoveHarness) -> (Account, BaseLimits) {
    let acc = setup_validator(h);
    let beef = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &beef,
        &common::test_dir_path("transaction_limits.data/pack"),
    ));
    h.new_epoch();

    // Measure the execution gas `spin` uses under a limit it cannot reach.
    h.modify_gas_schedule(|params| {
        params.vm.txn.max_execution_gas = params.vm.txn.max_execution_gas_gov;
    });
    let txn = h.create_transaction_payload(
        &acc,
        TransactionPayload::EntryFunction(spin(SPIN_ITERATIONS)),
    );
    let (status, fee_statement, _) = run_and_get_fee(h, &acc, txn);
    assert_success!(status);

    let base_execution_gas = fee_statement.execution_gas_used() * 2 / 3;
    h.modify_gas_schedule(|params| {
        params.vm.txn.max_execution_gas =
            Gas::new(base_execution_gas) * params.vm.txn.gas_unit_scaling_factor;
    });
    let base = base_limits(h);
    assert_eq!(base.execution, base_execution_gas);
    (acc, base)
}

/// Transfers away most of the account's balance. Returns a max gas amount
/// whose fee at the gas price is affordable, but not at the multiplier. Only
/// meaningful when `HighLimitPricing` is off (pre `RELEASE_V1_50`), where the
/// prologue reserve is `price * max_gas_amount` with no multiplier at all.
fn max_gas_between_reserves(h: &mut MoveHarness, acc: &Account) -> u64 {
    let other = h.new_account_with_balance_and_sequence_number(0, 0);
    let keep = 5_0000_0000;
    let amount = h.read_aptos_balance(acc.address()) - keep;
    assert_success!(
        h.run_transaction_payload(acc, aptos_account_transfer(*other.address(), amount))
    );

    let balance = h.read_aptos_balance(acc.address());
    let max_gas_amount = balance / (5 * DEFAULT_GAS_UNIT_PRICE);
    assert!(DEFAULT_GAS_UNIT_PRICE * max_gas_amount <= balance);
    assert!(EXCESS_PRICE_MULTIPLIER * DEFAULT_GAS_UNIT_PRICE * max_gas_amount > balance);
    max_gas_amount
}

/// Transfers away most of the account's balance. Returns a max gas amount,
/// a multiple of the multiplier, whose scaled-down reserve (`price * k *
/// (max_gas_amount / k)`, which is exactly `price * max_gas_amount` for a
/// multiple of `k`) the remaining balance cannot cover.
fn max_gas_above_scaled_reserve(h: &mut MoveHarness, acc: &Account) -> u64 {
    let other = h.new_account_with_balance_and_sequence_number(0, 0);
    let keep = 5_0000_0000;
    let amount = h.read_aptos_balance(acc.address()) - keep;
    assert_success!(
        h.run_transaction_payload(acc, aptos_account_transfer(*other.address(), amount))
    );

    let balance = h.read_aptos_balance(acc.address());
    let max_gas_amount = (balance / (DEFAULT_GAS_UNIT_PRICE * EXCESS_PRICE_MULTIPLIER) + 1)
        * EXCESS_PRICE_MULTIPLIER;
    assert_eq!(max_gas_amount % EXCESS_PRICE_MULTIPLIER, 0);
    assert!(DEFAULT_GAS_UNIT_PRICE * max_gas_amount > balance);
    max_gas_amount
}

#[test]
fn test_high_limit_txn_within_base_limits_pays_gas_price() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let txn = sign_txn_with_limits(&mut h, &acc, stake_pool_owner(200, 200));
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    assert_success!(status);

    let (fee, excess) = fee_and_excess(&fee_statement, &base_limits(&h));
    assert_eq!(excess, 0);
    assert_eq!(paid, fee);
}

#[test]
fn test_high_limit_txn_excess_pays_multiplier() {
    let mut h = new_test_harness();
    let (acc, base) = setup_excess_execution(&mut h);

    let payload = entry_function_with_limits(spin(SPIN_ITERATIONS), stake_pool_owner(200, 200));
    let txn = h.create_transaction_payload(&acc, payload);
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    assert_success!(status);

    let (fee, excess) = fee_and_excess(&fee_statement, &base);
    assert!(excess > 0);
    assert_eq!(paid, fee);
}

#[test]
fn test_high_limit_txn_excess_pays_multiplier_on_failure() {
    let mut h = new_test_harness();
    let (acc, base) = setup_excess_execution(&mut h);

    // Spins past the raised execution limit.
    let payload = entry_function_with_limits(spin(3 * SPIN_ITERATIONS), stake_pool_owner(200, 200));
    let txn = h.create_transaction_payload(&acc, payload);
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    assert_vm_status!(status, StatusCode::EXECUTION_LIMIT_REACHED);

    let (fee, excess) = fee_and_excess(&fee_statement, &base);
    assert!(excess > 0);
    assert_eq!(paid, fee);
}

#[test]
fn test_normal_txn_above_base_limits_pays_gas_price() {
    let mut h = new_test_harness();
    let (acc, base) = setup_excess_execution(&mut h);

    let txn = h.create_transaction_payload(
        &acc,
        TransactionPayload::EntryFunction(spin(SPIN_ITERATIONS)),
    );
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    assert_vm_status!(status, StatusCode::EXECUTION_LIMIT_REACHED);

    let (fee, excess) = fee_and_excess(&fee_statement, &base);
    assert!(excess > 0);
    assert_eq!(paid, fee);
}

#[test]
fn test_high_limit_txn_reserves_max_fee_at_multiplier() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    let max_gas_amount = max_gas_above_scaled_reserve(&mut h, &acc);
    let txn = h
        .create_transaction_without_sign(
            &acc,
            payload_with_limits(*acc.address(), stake_pool_owner(200, 200)),
        )
        .max_gas_amount(max_gas_amount)
        .sign();
    run_and_assert_discard(
        &mut h,
        txn,
        StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE,
    );
}

#[test]
fn test_high_limit_txn_max_fee_overflow() {
    let mut h = new_test_harness();
    let acc = setup_validator(&mut h);
    h.new_epoch();

    // `max_gas_amount` is scaled down by the multiplier before
    // `HighLimitPricing::max_fee` sees it, so (for a `max_gas_amount` that's
    // a multiple of the multiplier) the overflow threshold is the same
    // `price * max_gas_amount` a normal transaction would hit.
    let max_gas_amount = 1_000_000;
    assert_eq!(max_gas_amount % EXCESS_PRICE_MULTIPLIER, 0);
    let gas_unit_price = u64::MAX / max_gas_amount + 1;
    h.modify_gas_schedule(|params| {
        params.vm.txn.max_price_per_gas_unit = gas_unit_price.into();
    });
    let txn = h
        .create_transaction_without_sign(
            &acc,
            payload_with_limits(*acc.address(), stake_pool_owner(200, 200)),
        )
        .gas_unit_price(gas_unit_price)
        .max_gas_amount(max_gas_amount)
        .sign();
    run_and_assert_discard(&mut h, txn, StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND);
}

#[test]
fn test_high_limit_txn_drained_balance_pays_on_failure() {
    let mut h = new_test_harness();
    let (acc, base) = setup_excess_execution(&mut h);
    let recipient = h.new_account_with_balance_and_sequence_number(0, 0);

    // The payload leaves too little to pay the fee, so the success epilogue
    // fails and the failure epilogue charges against the pre-payload balance.
    let amount = h.read_aptos_balance(acc.address()) - 1;
    let payload = entry_function_with_limits(
        spin_then_transfer(*recipient.address(), amount, SPIN_ITERATIONS),
        stake_pool_owner(200, 200),
    );
    let txn = h.create_transaction_payload(&acc, payload);
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { code, .. }) => {
            assert_eq!(split_canonical(code), (OUT_OF_RANGE, ECANT_PAY_GAS_DEPOSIT));
        },
        status => panic!("Expected the epilogue to abort, got {:?}", status),
    }

    let (fee, excess) = fee_and_excess(&fee_statement, &base);
    assert!(excess > 0);
    assert_eq!(paid, fee);
    assert_eq!(h.read_aptos_balance(recipient.address()), 0);
}

#[test]
fn test_high_limit_txn_before_v1_50_pays_gas_price() {
    let mut h = new_test_harness();
    let (acc, base) = setup_excess_execution(&mut h);
    h.modify_gas_schedule_raw(|gas_schedule| gas_schedule.feature_version = RELEASE_V1_49);

    let payload = entry_function_with_limits(spin(SPIN_ITERATIONS), stake_pool_owner(200, 200));
    let txn = h.create_transaction_payload(&acc, payload);
    let (status, fee_statement, paid) = run_and_get_fee(&mut h, &acc, txn);
    assert_success!(status);

    let (fee, excess) = fee_and_excess(&fee_statement, &base);
    assert!(excess > 0);
    assert_eq!(paid, fee);

    // The prologue reserves the max fee at the gas price.
    let max_gas_amount = max_gas_between_reserves(&mut h, &acc);
    let txn = h
        .create_transaction_without_sign(
            &acc,
            payload_with_limits(*acc.address(), stake_pool_owner(200, 200)),
        )
        .max_gas_amount(max_gas_amount)
        .sign();
    assert_success!(h.run(txn));
}
