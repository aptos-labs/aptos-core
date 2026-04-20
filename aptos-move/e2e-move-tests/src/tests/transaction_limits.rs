// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for staking-backed transaction limits.

use crate::{assert_success, stake::setup_staking, MoveHarness};
use aptos_cached_packages::aptos_stdlib::{
    aptos_coin_transfer, delegation_pool_add_stake, delegation_pool_initialize_delegation_pool,
    stake_set_delegated_voter,
};
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    move_utils::MemberId,
    on_chain_config::FeatureFlag,
    transaction::{
        RequestedMultipliers, SignedTransaction, TransactionExecutable, TransactionExtraConfig,
        TransactionPayload, TransactionPayloadInner, TransactionStatus, UserTxnLimitsRequest,
    },
};
use move_core_types::{ident_str, language_storage::StructTag, vm_status::StatusCode};
use serde::Serialize;
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

fn stake_pool_owner(execution_bps: u64, io_bps: u64) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::StakePoolOwner {
        multipliers: RequestedMultipliers::V1 {
            execution_bps,
            io_bps,
        },
    }
}

fn delegated_voter(pool: AccountAddress, execution_bps: u64, io_bps: u64) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::DelegatedVoter {
        pool_address: pool,
        multipliers: RequestedMultipliers::V1 {
            execution_bps,
            io_bps,
        },
    }
}

fn delegation_pool_delegator(
    pool: AccountAddress,
    execution_bps: u64,
    io_bps: u64,
) -> UserTxnLimitsRequest {
    UserTxnLimitsRequest::DelegationPoolDelegator {
        pool_address: pool,
        multipliers: RequestedMultipliers::V1 {
            execution_bps,
            io_bps,
        },
    }
}

fn payload_with_limits(
    sender_addr: AccountAddress,
    request: UserTxnLimitsRequest,
) -> TransactionPayload {
    match aptos_coin_transfer(sender_addr, 0) {
        TransactionPayload::EntryFunction(entry_func) => {
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable: TransactionExecutable::EntryFunction(entry_func),
                extra_config: TransactionExtraConfig::V2 {
                    multisig_address: None,
                    replay_protection_nonce: None,
                    txn_limits_request: Some(request),
                },
            })
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
    // Override gas unit price because high-limit transactions require
    // 10x default minimum value.
    h.create_transaction_without_sign(acc, payload_with_limits(*acc.address(), request))
        .gas_unit_price(gas_unit_price)
        .sign()
}

fn sign_txn_with_limits(
    h: &mut MoveHarness,
    acc: &Account,
    request: UserTxnLimitsRequest,
) -> SignedTransaction {
    sign_txn_with_limits_and_gas_unit_price(h, acc, request, 10 * DEFAULT_GAS_UNIT_PRICE)
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
        .gas_unit_price(10 * DEFAULT_GAS_UNIT_PRICE)
        .sign_fee_payer()
}

// Mirrors `0x1::transaction_limits::TxnLimitTier` for BCS serialization.
#[derive(Serialize)]
struct TxnLimitTier {
    min_stake: u64,
    multiplier_bps: u64,
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
        .map(|(min_stake, multiplier_bps)| TxnLimitTier {
            min_stake: *min_stake,
            multiplier_bps: *multiplier_bps,
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

fn setup_delegation_pool(
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
