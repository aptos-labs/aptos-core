// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::{init, initialize, AggV2TestHarness, AggregatorLocation, ElementType, UseType},
    assert_abort, assert_success,
    tests::common,
    MoveHarness,
};
use aptos_framework::natives::aggregator_natives::aggregator_v2::{
    EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED, EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
};
use aptos_language_e2e_tests::executor::{DelayedFieldOptimizationMode, ExecutorMode};
use aptos_types::transaction::SignedTransaction;
use proptest::prelude::*;

const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;
const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

const DEFAULT_EXECUTOR_MODE: ExecutorMode = ExecutorMode::BothComparison;
const DEFAULT_DELAYED_FIELDS_MODE: DelayedFieldOptimizationMode =
    DelayedFieldOptimizationMode::BothComparison;

fn setup(
    executor_mode: ExecutorMode,
    delayed_fields_mode: DelayedFieldOptimizationMode,
    txns: usize,
) -> AggV2TestHarness {
    initialize(
        common::test_dir_path("aggregator_v2.data/pack"),
        executor_mode,
        delayed_fields_mode,
        txns,
    )
}

#[cfg(test)]
mod test_cases {
    use super::*;

    #[test]
    fn test_copy_snapshot() {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, DEFAULT_DELAYED_FIELDS_MODE, 1);
        let txn = h.verify_copy_snapshot();
        assert_abort!(h.harness.run(txn), EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED);
    }

    #[test]
    fn test_copy_string_snapshot() {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, DEFAULT_DELAYED_FIELDS_MODE, 1);
        let txn = h.verify_copy_string_snapshot();
        assert_abort!(h.harness.run(txn), EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED);
    }

    #[test]
    fn test_snapshot_concat() {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, DEFAULT_DELAYED_FIELDS_MODE, 1);
        let txn = h.verify_string_concat();
        assert_success!(h.harness.run(txn));
    }

    #[test]
    fn test_string_snapshot_concat() {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, DEFAULT_DELAYED_FIELDS_MODE, 1);
        let txn = h.verify_string_snapshot_concat();
        assert_abort!(h.harness.run(txn), EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE);
    }

    #[test]
    fn test_aggregators_e2e() {
        println!("Testing test_aggregators_e2e");
        let element_type = ElementType::U64;
        let use_type = UseType::UseTableType;

        let mut h = setup(DEFAULT_EXECUTOR_MODE, DEFAULT_DELAYED_FIELDS_MODE, 100);

        let init_txn = init(&mut h.harness, &h.account, use_type, element_type, true);
        h.harness.run(init_txn);

        let addr = *h.account.address();
        let loc = |i| AggregatorLocation::new(addr, element_type, use_type, i);

        let block_size = 30;

        // Create many aggregators with deterministic limit.
        let txns = (0..block_size)
            .map(|i| (0, h.new(&loc(i), (i as u128) * 100000)))
            .collect();
        run_block_in_parts(&mut h.harness, BlockSplit::Whole, txns);

        // All transactions in block must fail, so values of aggregators are still 0.
        let failed_txns = (0..block_size)
            .map(|i| match i % 2 {
                0 => (
                    EAGGREGATOR_OVERFLOW,
                    h.materialize_and_add(&loc(i), (i as u128) * 100000 + 1),
                ),
                _ => (
                    EAGGREGATOR_UNDERFLOW,
                    h.materialize_and_sub(&loc(i), (i as u128) * 100000 + 1),
                ),
            })
            .collect();
        run_block_in_parts(&mut h.harness, BlockSplit::Whole, failed_txns);

        // Now test all operations. To do that, make sure aggregator have values large enough.
        let txns = (0..block_size)
            .map(|i| (0, h.add(&loc(i), (i as u128) * 1000)))
            .collect();

        run_block_in_parts(&mut h.harness, BlockSplit::Whole, txns);

        // TODO[agg_v2](test): proptests with random transaction generator might be useful here.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (
                    0,
                    h.sub_add(&loc(i), (i as u128) * 1000, (i as u128) * 3000),
                ),
                1 => (0, h.materialize_and_add(&loc(i), (i as u128) * 1000)),
                2 => (0, h.sub_and_materialize(&loc(i), (i as u128) * 1000)),
                _ => (0, h.add(&loc(i), i as u128)),
            })
            .collect();
        run_block_in_parts(&mut h.harness, BlockSplit::Whole, txns);

        // Finally, check values.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (0, h.check(&loc(i), (i as u128) * 3000)),
                1 => (0, h.check(&loc(i), (i as u128) * 2000)),
                2 => (0, h.check(&loc(i), 0)),
                _ => (0, h.check(&loc(i), (i as u128) * 1000 + (i as u128))),
            })
            .collect();
        run_block_in_parts(&mut h.harness, BlockSplit::Whole, txns);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockSplit {
    Whole,
    TxnPerBlock,
    SplitIntoThree { first_len: usize, second_len: usize },
}

pub fn run_block_in_parts(
    harness: &mut MoveHarness,
    block_split: BlockSplit,
    txn_block: Vec<(u64, SignedTransaction)>,
) {
    fn run_and_check_block(
        harness: &mut MoveHarness,
        txn_block: Vec<(u64, SignedTransaction)>,
        offset: usize,
    ) {
        if txn_block.is_empty() {
            return;
        }
        let (errors, txns): (Vec<_>, Vec<_>) = txn_block.into_iter().unzip();
        println!(
            "=== E2E move test: Running block from {} with {} tnx ===",
            offset,
            txns.len()
        );
        let outputs = harness.run_block(txns);
        for (idx, (error, status)) in errors.into_iter().zip(outputs.into_iter()).enumerate() {
            if error > 0 {
                assert_abort!(
                    status,
                    error,
                    "Error code missmaptch on txn {} that should've failed, with block starting at {}. Expected {}, gotten {:?}",
                    idx + offset,
                    offset,
                    error,
                    status,
                );
            } else {
                assert_success!(
                    status,
                    "Didn't succeed on txn {}, with block starting at {}",
                    idx + offset,
                    offset,
                );
            }
        }
    }

    match block_split {
        BlockSplit::Whole => {
            run_and_check_block(harness, txn_block, 0);
        },
        BlockSplit::TxnPerBlock => {
            for (idx, (error, status)) in txn_block.into_iter().enumerate() {
                run_and_check_block(harness, vec![(error, status)], idx);
            }
        },
        BlockSplit::SplitIntoThree {
            first_len,
            second_len,
        } => {
            assert!(first_len + second_len <= txn_block.len());
            let (left, rest) = txn_block.split_at(first_len);
            let (mid, right) = rest.split_at(second_len);

            run_and_check_block(harness, left.to_vec(), 0);
            run_and_check_block(harness, mid.to_vec(), first_len);
            run_and_check_block(harness, right.to_vec(), first_len + second_len);
        },
    }
}

#[allow(dead_code)]
fn arb_block_split(len: usize) -> BoxedStrategy<BlockSplit> {
    (0..3)
        .prop_flat_map(move |enum_type| {
            // making running a test with a full block likely
            if enum_type == 0 {
                Just(BlockSplit::Whole).boxed()
            } else if enum_type == 1 {
                Just(BlockSplit::TxnPerBlock).boxed()
            } else {
                // First is non-empty, and not the whole block here: [1, len)
                (1usize..len)
                    .prop_flat_map(move |first| {
                        // Second is non-empty, but can finish the block: [1, len - first]
                        (Just(first), 1usize..len - first + 1)
                    })
                    .prop_map(|(first, second)| BlockSplit::SplitIntoThree {
                        first_len: first,
                        second_len: second,
                    })
                    .boxed()
            }
        })
        .boxed()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestEnvConfig {
    pub executor_mode: ExecutorMode,
    pub delayed_fields_mode: DelayedFieldOptimizationMode,
    pub block_split: BlockSplit,
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            delayed_fields_mode: DelayedFieldOptimizationMode::EnabledOnly,
            block_split
        }),
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            delayed_fields_mode: DelayedFieldOptimizationMode::DisabledOnly,
            block_split
        }),

        // TODO[agg_v2](fix) currently fails, replace instead of the above separate tests.
        // arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
        //     executor_mode: ExecutorMode::BothComparison,
        //     delayed_fields_mode: DelayedFieldOptimizationMode::BothComparison,
        //     block_split
        // }),
    ]
    .boxed()
}

fn arb_agg_type() -> BoxedStrategy<ElementType> {
    prop_oneof![Just(ElementType::U64), Just(ElementType::U128),].boxed()
}

// fn arb_snap_type() -> BoxedStrategy<ElementType> {
//     prop_oneof![
//         Just(ElementType::U64),
//         Just(ElementType::U128),
//         Just(ElementType::String),
//     ].boxed()
// }

fn arb_use_type() -> BoxedStrategy<UseType> {
    prop_oneof![
        Just(UseType::UseResourceType),
        Just(UseType::UseTableType),
        // TODO[agg_v2](fix) add back once ResourceGroups are supported
        // Just(UseType::UseResourceGroupType),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        // We will test a few more comprehensive tests more times, and the rest even fewer.
        // when trying to stress-test, increase (to 200 or more), and disable result cache.
        cases: 10,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime(test_env in arb_test_env(14), element_type in arb_agg_type(), use_type in arb_use_type()) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 14);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new(&agg_loc, 1500)),
            (0, h.add(&agg_loc, 400)), // 400
            (0, h.materialize(&agg_loc)),
            (0, h.add(&agg_loc, 500)), // 900
            (0, h.check(&agg_loc, 900)),
            (0, h.materialize_and_add(&agg_loc, 600)), // 1500
            (0, h.materialize_and_sub(&agg_loc, 600)), // 900
            (0, h.check(&agg_loc, 900)),
            (0, h.sub_add(&agg_loc, 200, 300)), // 1000
            (0, h.check(&agg_loc, 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (0x02_0001, h.add_and_materialize(&agg_loc, 501)),
            (0x02_0002, h.sub_and_materialize(&agg_loc, 1001)),
            (0, h.check(&agg_loc, 1000)),
        ];
        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_multiple_aggregators_and_collocation(
        test_env in arb_test_env(24),
        element_type in arb_agg_type(),
        use_type in arb_use_type(),
        is_2_collocated in any::<bool>(),
        is_3_collocated in any::<bool>(),
    ) {
        println!("Testing test_multiple_aggregators_and_collocation {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 24);
        let acc_2 = h.harness.new_account_with_key_pair();
        let acc_3 = h.harness.new_account_with_key_pair();

        let mut idx_1 = 0;
        let agg_1_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let agg_2_loc = {
            let (cur_acc, idx_2) = if is_2_collocated { idx_1 += 1; (h.account.address(), idx_1) } else { (acc_2.address(), 0)};
            AggregatorLocation::new(*cur_acc, element_type, use_type, idx_2)
        };
        let agg_3_loc = {
            let (cur_acc, idx_3) = if is_3_collocated { idx_1 += 1; (h.account.address(), idx_1) } else { (acc_3.address(), 0)};
            AggregatorLocation::new(*cur_acc, element_type, use_type, idx_3)
        };
        println!("agg_1_loc: {:?}", agg_1_loc);
        println!("agg_2_loc: {:?}", agg_2_loc);
        println!("agg_3_loc: {:?}", agg_3_loc);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, init(&mut h.harness, &acc_2, use_type, element_type, true)),
            (0, init(&mut h.harness, &acc_3, use_type, element_type, true)),
            (0, h.new_add(&agg_1_loc, 10, 5)),
            (0, h.new_add(&agg_2_loc, 10, 5)),
            (0, h.new_add(&agg_3_loc, 10, 5)),  // 5, 5, 5
            (0, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // 6, 6, 5
            (0, h.add_2(&agg_1_loc, &agg_3_loc, 1, 1)), // 7, 6, 6
            (0x02_0001, h.add(&agg_1_loc, 5)), // X
            (0, h.add_sub(&agg_1_loc, 3, 3)), // 7, 6, 6
            (0x02_0001, h.add_2(&agg_1_loc, &agg_2_loc, 3, 5)), // X
            (0, h.add_2(&agg_1_loc, &agg_2_loc, 3, 1)), // 10, 7, 6
            (0x02_0001, h.add_sub(&agg_1_loc, 3, 3)), // X
            (0, h.sub(&agg_1_loc, 3)), // 7, 7, 6
            (0, h.add_2(&agg_2_loc, &agg_3_loc, 2, 2)), // 7, 9, 8
            (0, h.check(&agg_2_loc, 9)),
            (0x02_0001, h.add_2(&agg_1_loc, &agg_2_loc, 1, 2)), // X
            (0, h.add_2(&agg_2_loc, &agg_3_loc, 1, 2)), // 7, 10, 10
            (0x02_0001, h.add(&agg_2_loc, 1)), // X
            (0x02_0001, h.add_and_materialize(&agg_3_loc, 1)), // X
            (0x02_0001, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // X
            (0, h.check(&agg_1_loc, 7)),
            (0, h.check(&agg_2_loc, 10)),
            (0, h.check(&agg_3_loc, 10)),
        ];
        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough for these
        // when trying to stress-test, increase (to 200 or more), and disable result cache.
        cases: 5,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_underflow(test_env in arb_test_env(4)) {
        println!("Testing test_aggregator_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 4);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new(&agg_loc, 600)),
            (0, h.add(&agg_loc, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            (0x02_0002, h.sub(&agg_loc, 500))
        ];
        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_underflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new(&agg_loc, 600)),
            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            (0x02_0002, h.materialize_and_sub(&agg_loc, 400)),
        ];

        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new_add(&agg_loc, 600, 400)),
            // Limit exceeded - abort with EAGGREGATOR_OVERFLOW.
            (0x02_0001, h.add(&agg_loc, 201))
        ];

        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h= setup(test_env.executor_mode, test_env.delayed_fields_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new(&agg_loc, 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            (0x02_0001, h.materialize_and_add(&agg_loc, 400)),
        ];

        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_snapshot(test_env in arb_test_env(9)) {
        println!("Testing test_aggregator_snapshot {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 9);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let snap_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let derived_snap_loc = AggregatorLocation::new(*h.account.address(), ElementType::String, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, init(&mut h.harness, &h.account, use_type, element_type, false)),
            (0, init(&mut h.harness, &h.account, use_type, ElementType::String, false)),
            (0, h.new_add(&agg_loc, 400, 100)),
            (0, h.snapshot(&agg_loc, &snap_loc)),
            (0, h.check_snapshot(&snap_loc, 100)),
            (0, h.read_snapshot(&agg_loc)),
            (0, h.add_and_read_snapshot_u128(&agg_loc, 100)),
            (0, h.concat(&snap_loc, &derived_snap_loc, "12", "13")),
            (0, h.check_snapshot(&derived_snap_loc, 1210013)),
        ];

        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_write_ops(test_env in arb_test_env(4)) {
        println!("Testing test_aggregator_write_ops {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.delayed_fields_mode, 4);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h.harness, &h.account, use_type, element_type, true)),
            (0, h.new_add(&agg_loc, 1000, 100)),
            (0, h.add(&agg_loc, 200)),
            (0, h.sub(&agg_loc, 100))
        ];
        run_block_in_parts(
            &mut h.harness,
            test_env.block_split,
            txns,
        );
    }
}
