// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator::{
        add, add_and_materialize, check, destroy, initialize, materialize, materialize_and_add,
        materialize_and_sub, new, sub, sub_add, sub_and_materialize,
    },
    tests::common,
    BlockSplit, MoveHarness, SUCCESS,
};
use velor_language_e2e_tests::account::Account;
use proptest::prelude::*;
use test_case::test_case;

const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;
const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("aggregator.data/pack"))
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
    #[should_panic]
    fn test_aggregator_underflow(block_split in BlockSplit::arbitrary(3)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            // We cannot catch it, because we don't materialize it.
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
    #[should_panic]
    fn test_aggregator_overflow(block_split in BlockSplit::arbitrary(4)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (SUCCESS, new(&mut h, &acc, 0)),
            (SUCCESS, add(&mut h, &acc, 0, u128::MAX - 600)),
            (SUCCESS, add(&mut h, &acc, 0, 400)),
            // Currently, this one will panic, instead of throwing this code.
            // We cannot catch it, because we don't materialize it.
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
