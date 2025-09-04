// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::{
        initialize, initialize_enabled_disabled_comparison, AggV2TestHarness, AggregatorLocation,
        ElementType, StructType, UseType,
    },
    tests::common,
    BlockSplit, SUCCESS,
};
use velor_language_e2e_tests::executor::ExecutorMode;
use velor_types::{transaction::ExecutionStatus, vm_status::StatusCode};
use claims::assert_ok_eq;
use proptest::prelude::*;
use test_case::test_case;

const STRESSTEST_MODE: bool = false;

pub(crate) const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;
const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

const DEFAULT_EXECUTOR_MODE: ExecutorMode = ExecutorMode::SequentialOnly;

fn _setup(
    executor_mode: ExecutorMode,
    aggregator_execution_mode: AggregatorMode,
    txns: usize,
    allow_block_executor_fallback: bool,
) -> AggV2TestHarness {
    let path = common::test_dir_path("aggregator_v2.data/pack");
    match aggregator_execution_mode {
        AggregatorMode::EnabledOnly => initialize(
            path,
            executor_mode,
            true,
            txns,
            allow_block_executor_fallback,
        ),
        AggregatorMode::DisabledOnly => initialize(
            path,
            executor_mode,
            false,
            txns,
            allow_block_executor_fallback,
        ),
        AggregatorMode::BothComparison => initialize_enabled_disabled_comparison(
            path,
            executor_mode,
            txns,
            allow_block_executor_fallback,
        ),
    }
}

pub(crate) fn setup(
    executor_mode: ExecutorMode,
    aggregator_execution_mode: AggregatorMode,
    txns: usize,
) -> AggV2TestHarness {
    _setup(executor_mode, aggregator_execution_mode, txns, false)
}

pub(crate) fn setup_allow_fallback(
    executor_mode: ExecutorMode,
    aggregator_execution_mode: AggregatorMode,
    txns: usize,
) -> AggV2TestHarness {
    _setup(executor_mode, aggregator_execution_mode, txns, true)
}

#[cfg(test)]
mod test_cases {
    use super::*;

    #[test]
    fn test_snapshot_concat() {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, AggregatorMode::BothComparison, 1);
        let txn = h.verify_string_concat();
        h.run_block_in_parts_and_check(BlockSplit::Whole, vec![(SUCCESS, txn)]);
    }

    #[test]
    fn test_aggregators_e2e() {
        println!("Testing test_aggregators_e2e");
        let element_type = ElementType::U64;
        let use_type = UseType::UseTableType;

        let mut h = setup(DEFAULT_EXECUTOR_MODE, AggregatorMode::BothComparison, 100);

        let init_txn = h.init(None, use_type, element_type, StructType::Aggregator);
        h.run_block_in_parts_and_check(BlockSplit::Whole, vec![(SUCCESS, init_txn)]);

        let addr = *h.account.address();
        let loc = |i| AggregatorLocation::new(addr, element_type, use_type, i);

        let block_size = 30;

        // Create many aggregators with deterministic limit.
        let txns = (0..block_size)
            .map(|i| (SUCCESS, h.new(&loc(i), (i as u128) * 100000)))
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

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
        h.run_block_in_parts_and_check(BlockSplit::Whole, failed_txns);

        // Now test all operations. To do that, make sure aggregator have values large enough.
        let txns = (0..block_size)
            .map(|i| (SUCCESS, h.add(&loc(i), (i as u128) * 1000)))
            .collect();

        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

        // TODO[agg_v2](test): proptests with random transaction generator might be useful here.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (
                    SUCCESS,
                    h.sub_add(&loc(i), (i as u128) * 1000, (i as u128) * 3000),
                ),
                1 => (SUCCESS, h.materialize_and_add(&loc(i), (i as u128) * 1000)),
                2 => (SUCCESS, h.sub_and_materialize(&loc(i), (i as u128) * 1000)),
                _ => (SUCCESS, h.add(&loc(i), i as u128)),
            })
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

        // Finally, check values.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (SUCCESS, h.check(&loc(i), (i as u128) * 3000)),
                1 => (SUCCESS, h.check(&loc(i), (i as u128) * 2000)),
                2 => (SUCCESS, h.check(&loc(i), 0)),
                _ => (SUCCESS, h.check(&loc(i), (i as u128) * 1000 + (i as u128))),
            })
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);
    }
}

pub fn arb_block_split(len: usize) -> BoxedStrategy<BlockSplit> {
    (0..3)
        .prop_flat_map(move |enum_type| {
            // making running a test with a full block likely
            if enum_type == 0 {
                Just(BlockSplit::Whole).boxed()
            } else if enum_type == 1 {
                Just(BlockSplit::SingleTxnPerBlock).boxed()
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
pub enum AggregatorMode {
    EnabledOnly,
    DisabledOnly,
    BothComparison,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestEnvConfig {
    pub executor_mode: ExecutorMode,
    pub aggregator_execution_mode: AggregatorMode,
    pub block_split: BlockSplit,
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_mode: AggregatorMode::BothComparison,
            block_split
        }),
    ]
    .boxed()
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env_non_equivalent(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_mode: AggregatorMode::DisabledOnly,
            block_split
        }),
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_mode: AggregatorMode::EnabledOnly,
            block_split
        }),
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
        Just(UseType::UseResourceGroupType),
    ]
    .boxed()
}

fn arb_droppable_use_type() -> BoxedStrategy<UseType> {
    prop_oneof![
        Just(UseType::UseResourceType),
        // Just(UseType::UseTableType),
        Just(UseType::UseResourceGroupType),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        // We will test a few more comprehensive tests more times, and the rest even fewer.
        cases: if STRESSTEST_MODE { 1000 } else { 20 },
        result_cache: if STRESSTEST_MODE { prop::test_runner::noop_result_cache } else {prop::test_runner::basic_result_cache },
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime(test_env in arb_test_env(14), element_type in arb_agg_type(), use_type in arb_use_type()) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 14);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new(&agg_loc, 1500)),
            (SUCCESS, h.add(&agg_loc, 400)), // 400
            (SUCCESS, h.materialize(&agg_loc)),
            (SUCCESS, h.add(&agg_loc, 500)), // 900
            (SUCCESS, h.check(&agg_loc, 900)),
            (SUCCESS, h.materialize_and_add(&agg_loc, 600)), // 1500
            (SUCCESS, h.materialize_and_sub(&agg_loc, 600)), // 900
            (SUCCESS, h.check(&agg_loc, 900)),
            (SUCCESS, h.sub_add(&agg_loc, 200, 300)), // 1000
            (SUCCESS, h.check(&agg_loc, 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (EAGGREGATOR_OVERFLOW, h.add_and_materialize(&agg_loc, 501)),
            (EAGGREGATOR_UNDERFLOW, h.sub_and_materialize(&agg_loc, 1001)),
            (SUCCESS, h.check(&agg_loc, 1000)),
        ];
        h.run_block_in_parts_and_check(
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
        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 24);
        let acc_2 = h.new_account_with_key_pair();
        let acc_3 = h.new_account_with_key_pair();

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
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.init(Some(&acc_2), use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.init(Some(&acc_3), use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_1_loc, 10, 5)),
            (SUCCESS, h.new_add(&agg_2_loc, 10, 5)),
            (SUCCESS, h.new_add(&agg_3_loc, 10, 5)),  // 5, 5, 5
            (SUCCESS, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // 6, 6, 5
            (SUCCESS, h.add_2(&agg_1_loc, &agg_3_loc, 1, 1)), // 7, 6, 6
            (EAGGREGATOR_OVERFLOW, h.add(&agg_1_loc, 5)), // X
            (SUCCESS, h.add_sub(&agg_1_loc, 3, 3)), // 7, 6, 6
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 3, 5)), // X
            (SUCCESS, h.add_2(&agg_1_loc, &agg_2_loc, 3, 1)), // 10, 7, 6
            (EAGGREGATOR_OVERFLOW, h.add_sub(&agg_1_loc, 3, 3)), // X
            (SUCCESS, h.sub(&agg_1_loc, 3)), // 7, 7, 6
            (SUCCESS, h.add_2(&agg_2_loc, &agg_3_loc, 2, 2)), // 7, 9, 8
            (SUCCESS, h.check(&agg_2_loc, 9)),
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 1, 2)), // X
            (SUCCESS, h.add_2(&agg_2_loc, &agg_3_loc, 1, 2)), // 7, 10, 10
            (EAGGREGATOR_OVERFLOW, h.add(&agg_2_loc, 1)), // X
            (EAGGREGATOR_OVERFLOW, h.add_and_materialize(&agg_3_loc, 1)), // X
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // X
            (SUCCESS, h.check(&agg_1_loc, 7)),
            (SUCCESS, h.check(&agg_2_loc, 10)),
            (SUCCESS, h.check(&agg_3_loc, 10)),
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough for these
        cases: if STRESSTEST_MODE { 1000 } else { 10 },
        // TODO: result cache breaks with proptest v1.1 and above because of this change: https://github.com/proptest-rs/proptest/pull/295.
        // result_cache: if STRESSTEST_MODE { prop::test_runner::noop_result_cache } else {prop::test_runner::basic_result_cache },
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_underflow(test_env in arb_test_env(4)) {
        println!("Testing test_aggregator_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 4);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new(&agg_loc, 600)),
            (SUCCESS, h.add(&agg_loc, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            (EAGGREGATOR_UNDERFLOW, h.sub(&agg_loc, 500))
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_underflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new(&agg_loc, 600)),
            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            (EAGGREGATOR_UNDERFLOW, h.materialize_and_sub(&agg_loc, 400)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_loc, 600, 400)),
            // Limit exceeded - abort with EAGGREGATOR_OVERFLOW.
            (EAGGREGATOR_OVERFLOW, h.add(&agg_loc, 201))
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h= setup(test_env.executor_mode, test_env.aggregator_execution_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new(&agg_loc, 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            (EAGGREGATOR_OVERFLOW, h.materialize_and_add(&agg_loc, 400)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_with_republish(test_env in arb_test_env(6), element_type in arb_agg_type(), use_type in arb_use_type()) {
        println!("Testing test_aggregator_with_republish {:?}", test_env);
        let mut h = setup_allow_fallback(test_env.executor_mode, test_env.aggregator_execution_mode, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_loc, 600, 400)),
            (SUCCESS, h.add(&agg_loc, 1)),
            (SUCCESS, h.republish()),
            (EAGGREGATOR_OVERFLOW, h.add(&agg_loc, 200)),
            (SUCCESS, h.add(&agg_loc, 1)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_recreate(test_env in arb_test_env(13), element_type in arb_agg_type(), use_type in arb_droppable_use_type()) {
        println!("Testing test_aggregator_recreate {:?}", test_env);
        let mut h = setup_allow_fallback(test_env.executor_mode, test_env.aggregator_execution_mode, 13);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_loc, 10, 3)),
            (SUCCESS, h.add(&agg_loc, 4)),
            (SUCCESS, h.new_add(&agg_loc, 10, 3)),
            (SUCCESS, h.add(&agg_loc, 4)),
            (SUCCESS, h.delete(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_loc, 10, 3)),
            (SUCCESS, h.add_delete(&agg_loc, 4)),
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.new_add(&agg_loc, 10, 5)),
            (EAGGREGATOR_OVERFLOW, h.add_delete(&agg_loc, 7)),
            (SUCCESS, h.add_delete(&agg_loc, 3)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_snapshot(test_env in arb_test_env_non_equivalent(10)) {
        println!("Testing test_aggregator_snapshot {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 10);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let snap_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let derived_snap_loc = AggregatorLocation::new(*h.account.address(), ElementType::String, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.init(None, use_type, element_type, StructType::Snapshot)),
            (SUCCESS, h.init(None, use_type, ElementType::String, StructType::DerivedString)),
            (SUCCESS, h.new_add(&agg_loc, 400, 100)),
            (SUCCESS, h.snapshot(&agg_loc, &snap_loc)),
            (SUCCESS, h.check_snapshot(&snap_loc, 100)),
            (SUCCESS, h.read_snapshot(&agg_loc)),
            (SUCCESS, h.add_and_read_snapshot_u128(&agg_loc, 100)),
            (SUCCESS, h.concat(&snap_loc, &derived_snap_loc, "12", "13")),
            (SUCCESS, h.check_derived(&derived_snap_loc, 1210013)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_is_at_least(test_env in arb_test_env_non_equivalent(10)) {
        println!("Testing test_aggregator_is_at_least {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_mode, 10);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, StructType::Aggregator)),
            (SUCCESS, h.init(None, use_type, element_type, StructType::Snapshot)),
            (SUCCESS, h.init(None, use_type, ElementType::String, StructType::DerivedString)),
            (SUCCESS, h.new_add(&agg_loc, 400, 100)),
            (SUCCESS, h.add(&agg_loc, 50)),
            (SUCCESS, h.add(&agg_loc, 50)),
            (SUCCESS, h.add_if_at_least(&agg_loc, 180, 50)),
            (SUCCESS, h.sub(&agg_loc, 50)),
            (SUCCESS, h.add_if_at_least(&agg_loc, 220, 50)),
            (SUCCESS, h.check(&agg_loc, 200)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }
}

#[test]
fn test_aggregator_snapshot_equivalent_gas() {
    let test_env = TestEnvConfig {
        executor_mode: ExecutorMode::BothComparison,
        aggregator_execution_mode: AggregatorMode::BothComparison,
        block_split: BlockSplit::Whole,
    };

    println!("Testing test_aggregator_snapshot {:?}", test_env);
    let element_type = ElementType::U64;
    let use_type = UseType::UseResourceType;

    let mut h = setup(
        test_env.executor_mode,
        test_env.aggregator_execution_mode,
        6,
    );

    let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
    let snap_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
    let derived_snap_loc =
        AggregatorLocation::new(*h.account.address(), ElementType::String, use_type, 0);

    let txns = vec![
        (
            0,
            h.init(None, use_type, element_type, StructType::Aggregator),
        ),
        (
            0,
            h.init(None, use_type, element_type, StructType::Snapshot),
        ),
        (
            0,
            h.init(
                None,
                use_type,
                ElementType::String,
                StructType::DerivedString,
            ),
        ),
        (0, h.new_add(&agg_loc, 400, 100)),
        (0, h.snapshot(&agg_loc, &snap_loc)),
        // string needs to be large, for gas rounding to be different
        (
            0,
            h.concat(
                &snap_loc,
                &derived_snap_loc,
                &String::from_utf8(vec![b'A'; 1000]).unwrap(),
                "13",
            ),
        ),
    ];

    h.run_block_in_parts_and_check(test_env.block_split, txns);
}

// Table splits into multiple resources, so test is not as straightforward
#[test_case(UseType::UseResourceGroupType)]
#[test_case(UseType::UseResourceType)]
fn test_too_many_aggregators_in_a_resource(use_type: UseType) {
    let test_env = TestEnvConfig {
        executor_mode: ExecutorMode::BothComparison,
        aggregator_execution_mode: AggregatorMode::EnabledOnly,
        block_split: BlockSplit::Whole,
    };
    println!(
        "Testing test_too_many_aggregators_in_a_resource {:?}",
        test_env
    );

    let element_type = ElementType::U64;

    let mut h = setup(
        test_env.executor_mode,
        test_env.aggregator_execution_mode,
        12,
    );

    let agg_locs = (0..15)
        .map(|i| AggregatorLocation::new(*h.account.address(), element_type, use_type, i))
        .collect::<Vec<_>>();

    let mut txns = vec![(
        SUCCESS,
        h.init(None, use_type, element_type, StructType::Aggregator),
    )];
    for i in 0..10 {
        txns.push((SUCCESS, h.new(agg_locs.get(i).unwrap(), 10)));
    }
    h.run_block_in_parts_and_check(test_env.block_split, txns);

    let failed_txns = vec![h.new(agg_locs.get(10).unwrap(), 10)];
    let output = h.run_block(failed_txns);
    assert_eq!(output.len(), 1);
    assert_ok_eq!(
        output[0].status().status(),
        ExecutionStatus::MiscellaneousError(Some(StatusCode::TOO_MANY_DELAYED_FIELDS))
    );
}
