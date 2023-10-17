// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator::{
        add, add_and_materialize, check, destroy, initialize, materialize, materialize_and_add,
        materialize_and_sub, new, sub, sub_add, sub_and_materialize,
    },
    tests::common,
    BlockSplit, MoveHarness,
};
use aptos_language_e2e_tests::account::Account;
use proptest::prelude::*;
use test_case::test_case;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("aggregator.data/pack"))
}

#[test_case(BlockSplit::Whole)]
#[test_case(BlockSplit::SingleTxnPerBlock)]
fn test_aggregators_e2e(block_split: BlockSplit) {
    let (mut h, acc) = setup();
    let block_size = 200;

    // Create many aggregators with deterministic limit.
    let txns = (0..block_size)
        .map(|i| (0, new(&mut h, &acc, i, (i as u128) * 100000)))
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);

    // All transactions in block must fail, so values of aggregators are still 0.
    let failed_txns = (0..block_size)
        .map(|i| match i % 2 {
            0 => (
                0x02_0001,
                materialize_and_add(&mut h, &acc, i, (i as u128) * 100000 + 1),
            ),
            _ => (
                0x02_0002,
                materialize_and_sub(&mut h, &acc, i, (i as u128) * 100000 + 1),
            ),
        })
        .collect();
    h.run_block_in_parts_and_check(block_split, failed_txns);

    // Now test all operations. To do that, make sure aggregator have values large enough.
    let txns = (0..block_size)
        .map(|i| (0, add(&mut h, &acc, i, (i as u128) * 1000)))
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);

    // TODO: proptests with random transaction generator might be useful here.
    let txns = (0..block_size)
        .map(|i| {
            (0, match i % 4 {
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
            (0, match i % 4 {
                0 => check(&mut h, &acc, i, (i as u128) * 3000),
                1 => check(&mut h, &acc, i, (i as u128) * 2000),
                2 => check(&mut h, &acc, i, 0),
                _ => check(&mut h, &acc, i, (i as u128) * 1000 + (i as u128)),
            })
        })
        .collect();
    h.run_block_in_parts_and_check(block_split, txns);
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        cases: 5,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime(block_split in BlockSplit::arbitrary(15)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (0, new(&mut h, &acc, 0, 1500)),
            (0, add(&mut h, &acc, 0, 400)), // 400
            (0, materialize(&mut h, &acc, 0)),
            (0, add(&mut h, &acc, 0, 500)), // 900
            (0, check(&mut h, &acc, 0, 900)),
            (0, materialize_and_add(&mut h, &acc, 0, 600)), // 1500
            (0, materialize_and_sub(&mut h, &acc, 0, 600)), // 900
            (0, check(&mut h, &acc, 0, 900)),
            (0, sub_add(&mut h, &acc, 0, 200, 300)), // 1000
            (0, check(&mut h, &acc, 0, 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (0x02_0001, add_and_materialize(&mut h, &acc, 0, 501)),
            (0x02_0002, sub_and_materialize(&mut h, &acc, 0, 1001)),
            (0, check(&mut h, &acc, 0, 1000)),
            (0, destroy(&mut h, &acc, 0)),
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

            (0, new(&mut h, &acc, 0, 600)),
            (0, add(&mut h, &acc, 0, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            // we cannot catch it, because we don't materialize it.
            (0x02_0002, sub(&mut h, &acc, 0, 500)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    fn test_aggregator_materialize_underflow(block_split in BlockSplit::arbitrary(2)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (0, new(&mut h, &acc, 0, 600)),
            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            // we can catch it, because we materialize it.
            (0x02_0002, materialize_and_sub(&mut h, &acc, 0, 400)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

    #[test]
    #[should_panic]
    fn test_aggregator_overflow(block_split in BlockSplit::arbitrary(3)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (0, new(&mut h, &acc, 0, 600)),
            (0, add(&mut h, &acc, 0, 400)),
            // Currently, this one will panic, instead of throwing this code.
            // we cannot catch it, because we don't materialize it.
            (0x02_0001, add(&mut h, &acc, 0, 201)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }


    #[test]
    fn test_aggregator_materialize_overflow(block_split in BlockSplit::arbitrary(2)) {
        let (mut h, acc) = setup();

        let txns = vec![
            (0, new(&mut h, &acc, 0, 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            // we can catch it, because we materialize it.
            (0x02_0001, materialize_and_add(&mut h, &acc, 0, 400)),
        ];

        h.run_block_in_parts_and_check(block_split, txns);
    }

}
