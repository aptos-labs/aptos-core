// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aggregator::{
        add, add_and_materialize, check, destroy, initialize, materialize, materialize_and_add,
        materialize_and_sub, new, sub, sub_add, sub_and_materialize,
    },
    assert_success,
    tests::common,
    BlockSplit, MoveHarness, SUCCESS,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{account_config::CoinInfoResource, AptosCoinType};
use move_core_types::account_address::AccountAddress;
use proptest::prelude::*;
use test_case::test_case;

const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;
const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("aggregator.data/pack"))
}

/// Abort code for `error::permission_denied(ENOT_APTOS_FRAMEWORK_ADDRESS)`
/// raised by `system_addresses::assert_aptos_framework`.
const ENOT_APTOS_FRAMEWORK_ADDRESS: u64 = 0x5_0003;

/// Reads APT supply tracked by `CoinInfo<AptosCoin>` (excluding the supply of
/// the paired fungible asset).
fn coin_supply(h: &mut MoveHarness) -> u128 {
    let bytes = h
        .execute_view_function(
            str::parse("0x1::coin::coin_supply").unwrap(),
            vec![str::parse("0x1::aptos_coin::AptosCoin").unwrap()],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    bcs::from_bytes::<Option<u128>>(&bytes)
        .unwrap()
        .expect("APT supply is tracked")
}

#[test_case(BlockSplit::Whole)]
#[test_case(BlockSplit::SingleTxnPerBlock)]
#[test_case(BlockSplit::SplitIntoThree { first_len: 10, second_len: 12 })]
fn test_aggregator_supply_switch(block_split: BlockSplit) {
    const MINT_AMOUNT: u64 = 100_000;
    const NUM_MINTS_BEFORE_SWITCH: usize = 15;
    const NUM_MINTS_AFTER_SWITCH: usize = 15;
    // Balance used by `MoveHarness::new_account_at`.
    const FUNDED_BALANCE: u64 = 1_000_000_000_000_000;

    let mut h = MoveHarness::new();
    let framework = h.aptos_framework_account();
    // Core resources account holds the mint capability in test genesis.
    let core_resources = h.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
    let attacker = h.new_account_at(AccountAddress::random());
    let dsts = (0..NUM_MINTS_BEFORE_SWITCH + NUM_MINTS_AFTER_SWITCH)
        .map(|_| h.new_account_at(AccountAddress::random()))
        .collect::<Vec<_>>();

    // Test genesis tracks APT supply with a plain integer, while mainnet
    // still uses a parallelizable aggregator. Patch CoinInfo<AptosCoin> and
    // the aggregator table item to recreate the mainnet state.
    let initial_supply = coin_supply(&mut h);
    let coin_info = CoinInfoResource::<AptosCoinType>::random(u128::MAX);
    h.executor
        .apply_write_set(&coin_info.to_writeset(initial_supply).unwrap());
    let aggregator_key = coin_info.supply_aggregator_state_key();

    assert_eq!(coin_supply(&mut h), initial_supply);
    let coin_info = h.executor.read_apt_coin_info_resource().unwrap();
    let supply = coin_info.supply().as_ref().unwrap();
    assert!(supply.aggregator.is_some());
    assert!(supply.integer.is_none());

    // Each mint adds the amount to the supply aggregator, and subtracts it
    // back when the minted coin is converted to FA on deposit. This
    // exercises the aggregator in both directions, keeping the coin supply
    // constant.
    let mint = |h: &mut MoveHarness, dst: &Account| {
        (
            SUCCESS,
            h.create_transaction_payload(
                &core_resources,
                aptos_stdlib::aptos_coin_mint(*dst.address(), MINT_AMOUNT),
            ),
        )
    };

    let mut txns = vec![];
    for dst in &dsts[..NUM_MINTS_BEFORE_SWITCH] {
        txns.push(mint(&mut h, dst));
    }
    // Only the framework account can switch the supply to an integer.
    txns.push((
        ENOT_APTOS_FRAMEWORK_ADDRESS,
        h.create_transaction_payload(&attacker, aptos_stdlib::aptos_coin_upgrade_supply()),
    ));
    txns.push((
        SUCCESS,
        h.create_transaction_payload(&framework, aptos_stdlib::aptos_coin_upgrade_supply()),
    ));
    for dst in &dsts[NUM_MINTS_BEFORE_SWITCH..] {
        txns.push(mint(&mut h, dst));
    }
    h.run_block_in_parts_and_check(block_split, txns);

    // The supply is now a plain integer with the value preserved, and the
    // aggregator table item has been removed.
    let coin_info = h.executor.read_apt_coin_info_resource().unwrap();
    let supply = coin_info.supply().as_ref().unwrap();
    assert!(supply.aggregator.is_none());
    assert_eq!(supply.integer.as_ref().unwrap().value, initial_supply);
    assert!(h.read_state_value(&aggregator_key).is_none());
    assert_eq!(coin_supply(&mut h), initial_supply);

    // All mints landed (destination accounts do not pay gas).
    for dst in &dsts {
        assert_eq!(
            h.read_aptos_balance(dst.address()),
            FUNDED_BALANCE + MINT_AMOUNT
        );
    }

    // Upgrading again is a no-op.
    assert_success!(
        h.run_transaction_payload(&framework, aptos_stdlib::aptos_coin_upgrade_supply())
    );
    let coin_info = h.executor.read_apt_coin_info_resource().unwrap();
    let supply = coin_info.supply().as_ref().unwrap();
    assert!(supply.aggregator.is_none());
    assert_eq!(supply.integer.as_ref().unwrap().value, initial_supply);
}

#[test_case(BlockSplit::Whole, false)]
#[test_case(BlockSplit::Whole, true)]
#[test_case(BlockSplit::SingleTxnPerBlock, false)]
#[test_case(BlockSplit::SingleTxnPerBlock, true)]
fn test_aggregators_e2e(block_split: BlockSplit, upper_limit: bool) {
    let (mut h, acc) = setup();
    let block_size = 200;

    // Create many aggregators with deterministic limit.
    let txns = (0..block_size)
        .map(|i| (SUCCESS, new(&mut h, &acc, i)))
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);

    if upper_limit {
        let txns = (0..block_size)
            .map(|i| {
                (
                    SUCCESS,
                    add(&mut h, &acc, i, u128::MAX - (i as u128) * 100000),
                )
            })
            .collect();
        h.run_block_in_parts_and_check(block_split, txns);
    }

    // All transactions in block must fail, so values of aggregators are still 0.
    let failed_txns = (0..block_size)
        .filter_map(|i| {
            if upper_limit {
                match i % 2 {
                    0 => Some((
                        EAGGREGATOR_OVERFLOW,
                        materialize_and_add(&mut h, &acc, i, (i as u128) * 100000 + 1),
                    )),
                    _ => None,
                }
            } else {
                match i % 2 {
                    0 => None,
                    _ => Some((
                        EAGGREGATOR_UNDERFLOW,
                        materialize_and_sub(&mut h, &acc, i, (i as u128) * 100000 + 1),
                    )),
                }
            }
        })
        .collect();
    h.run_block_in_parts_and_check(block_split, failed_txns);

    // Now test all operations. To do that, make sure aggregator have values large enough.
    let txns = (0..block_size)
        .map(|i| (SUCCESS, add(&mut h, &acc, i, (i as u128) * 1000)))
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);

    // TODO: proptests with random transaction generator might be useful here.
    let txns = (0..block_size)
        .map(|i| {
            (SUCCESS, match i % 4 {
                0 => sub_add(&mut h, &acc, i, (i as u128) * 1000, (i as u128) * 3000),
                1 => materialize_and_add(&mut h, &acc, i, (i as u128) * 1000),
                2 => sub_and_materialize(&mut h, &acc, i, (i as u128) * 1000),
                _ => add(&mut h, &acc, i, i as u128),
            })
        })
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);

    // Finally, check values.
    let txns = (0..block_size)
        .map(|i| {
            let offset = if upper_limit {
                u128::MAX - (i as u128) * 100000
            } else {
                0
            };
            (SUCCESS, match i % 4 {
                0 => check(&mut h, &acc, i, offset + (i as u128) * 3000),
                1 => check(&mut h, &acc, i, offset + (i as u128) * 2000),
                2 => check(&mut h, &acc, i, offset),
                _ => check(&mut h, &acc, i, offset + (i as u128) * 1000 + (i as u128)),
            })
        })
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        cases: 5,
        // TODO: result cache breaks with proptest v1.1 and above because of this change: https://github.com/proptest-rs/proptest/pull/295.
        // result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime_upper_limit(block_split in BlockSplit::arbitrary(15)) {
        let (mut h, acc) = setup();

        let offset = u128::MAX - 1500;
        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, offset)),
            (SUCCESS, add(&mut h, &acc, 0, 400)), // 400
            (SUCCESS, materialize(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, 500)), // 900
            (SUCCESS, check(&mut h, &acc, 0, offset + 900)),
            (SUCCESS, materialize_and_add(&mut h, &acc, 0, 600)), // 1500
            (SUCCESS, materialize_and_sub(&mut h, &acc, 0, 600)), // 900
            (SUCCESS, check(&mut h, &acc, 0, offset + 900)),
            (SUCCESS, sub_add(&mut h, &acc, 0, 200, 300)), // 1000
            (SUCCESS, check(&mut h, &acc, 0, offset + 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (EAGGREGATOR_OVERFLOW, add_and_materialize(&mut h, &acc, 0, 501)),
            (SUCCESS, check(&mut h, &acc, 0, offset + 1000)),
            (SUCCESS, destroy(&mut h, &acc, 0)),
            // Aggregator has been destroyed and we cannot add this delta.
            (25863, add(&mut h, &acc, 0, 1)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    fn test_aggregator_lifetime_lower_limit(block_split in BlockSplit::arbitrary(14)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, 400)), // 400
            (SUCCESS, materialize(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, 500)), // 900
            (SUCCESS, check(&mut h, &acc, 0, 900)),
            (SUCCESS, materialize_and_add(&mut h, &acc, 0, 600)), // 1500
            (SUCCESS, materialize_and_sub(&mut h, &acc, 0, 600)), // 900
            (SUCCESS, check(&mut h, &acc, 0, 900)),
            (SUCCESS, sub_add(&mut h, &acc, 0, 200, 300)), // 1000
            (SUCCESS, check(&mut h, &acc, 0, 1000)),
            // transactions fails, and should have no side-effects.
            (EAGGREGATOR_UNDERFLOW, sub_and_materialize(&mut h, &acc, 0, 1001)),
            (SUCCESS, check(&mut h, &acc, 0, 1000)),
            (SUCCESS, destroy(&mut h, &acc, 0)),
            // Aggregator has been destroyed and we cannot add this delta.
            (25863, add(&mut h, &acc, 0, 1)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    fn test_aggregator_underflow(block_split in BlockSplit::arbitrary(3)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, 400)),
            // Value would drop below zero.
            (EAGGREGATOR_UNDERFLOW, sub(&mut h, &acc, 0, 500)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    fn test_aggregator_materialize_underflow(block_split in BlockSplit::arbitrary(2)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),

            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            // We can catch it, because we materialize it.
            (EAGGREGATOR_UNDERFLOW, materialize_and_sub(&mut h, &acc, 0, 400)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    fn test_aggregator_overflow(block_split in BlockSplit::arbitrary(4)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, u128::MAX - 600)),
            (SUCCESS, add(&mut h, &acc, 0, 400)),
            // Value would exceed the limit.
            (EAGGREGATOR_OVERFLOW, add(&mut h, &acc, 0, 201)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }


    #[test]
    fn test_aggregator_materialize_overflow(block_split in BlockSplit::arbitrary(3)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, u128::MAX - 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            // We can catch it, because we materialize it.
            (EAGGREGATOR_OVERFLOW, materialize_and_add(&mut h, &acc, 0, 400)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

}
