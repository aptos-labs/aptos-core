// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::{
        add, add_2, add_and_materialize, add_and_read_snapshot_u128, add_sub, check,
        check_snapshot, concat, init, initialize, materialize, materialize_and_add,
        materialize_and_sub, new, new_add, read_snapshot, snapshot, sub, sub_add,
        sub_and_materialize, verify_copy_snapshot, verify_copy_string_snapshot,
        verify_string_concat, verify_string_snapshot_concat, AggLocation, ElementType,
        ExecutorMode, UseType,
    },
    assert_abort, assert_success,
    tests::common,
    MoveHarness,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::transaction::SignedTransaction;
use proptest::prelude::*;

const DEFAULT_EXECUTOR_MODE: ExecutorMode = ExecutorMode::Sequential;

fn setup(
    executor_mode: ExecutorMode,
    aggregator_execution_enabled: bool,
) -> (MoveHarness, Account) {
    initialize(
        common::test_dir_path("aggregator_v2.data/pack"),
        executor_mode,
        aggregator_execution_enabled,
    )
}

#[cfg(test)]
mod test_cases {
    use super::*;
    use test_case::test_case;

    #[test_case(true)]
    #[test_case(false)]
    fn test_copy_snapshot(execution_enabled: bool) {
        let (mut h, acc) = setup(DEFAULT_EXECUTOR_MODE, execution_enabled);
        let txn = verify_copy_snapshot(&mut h, &acc);
        assert_abort!(h.run(txn), 0x03_0009);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_copy_string_snapshot(execution_enabled: bool) {
        let (mut h, acc) = setup(DEFAULT_EXECUTOR_MODE, execution_enabled);
        let txn = verify_copy_string_snapshot(&mut h, &acc);
        assert_abort!(h.run(txn), 0x03_0009);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_snapshot_concat(execution_enabled: bool) {
        let (mut h, acc) = setup(DEFAULT_EXECUTOR_MODE, execution_enabled);
        let txn = verify_string_concat(&mut h, &acc);
        assert_success!(h.run(txn));
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_string_snapshot_concat(execution_enabled: bool) {
        let (mut h, acc) = setup(DEFAULT_EXECUTOR_MODE, execution_enabled);
        let txn = verify_string_snapshot_concat(&mut h, &acc);
        assert_abort!(h.run(txn), 0x03_0005);
    }

    // This tests uses multuple blocks, so requires exchange to be done to work.
    // #[test_case(true)]
    #[test_case(false)]
    fn test_aggregators_e2e(execution_enabled: bool) {
        let element_type = ElementType::U64;
        let use_type = UseType::UseTableType;

        let (mut h, acc) = setup(DEFAULT_EXECUTOR_MODE, execution_enabled);

        let init_txn = init(&mut h, &acc, use_type, element_type, true);
        h.run(init_txn);

        let loc = |i| AggLocation::new(&acc, element_type, use_type, i);

        let block_size = 300;

        // Create many aggregators with deterministic limit.
        let txns: Vec<SignedTransaction> = (0..block_size)
            .map(|i| new(&mut h, &loc(i), (i as u128) * 100000))
            .collect();
        h.run_block(txns);

        // All transactions in block must fail, so values of aggregators are still 0.
        let failed_txns: Vec<SignedTransaction> = (0..block_size)
            .map(|i| match i % 2 {
                0 => materialize_and_add(&mut h, &loc(i), (i as u128) * 100000 + 1),
                _ => materialize_and_sub(&mut h, &loc(i), (i as u128) * 100000 + 1),
            })
            .collect();
        h.run_block(failed_txns);

        // Now test all operations. To do that, make sure aggregator have values large enough.
        let txns: Vec<SignedTransaction> = (0..block_size)
            .map(|i| add(&mut h, &loc(i), (i as u128) * 1000))
            .collect();
        h.run_block(txns);

        // TODO: proptests with random transaction generator might be useful here.
        let txns: Vec<SignedTransaction> = (0..block_size)
            .map(|i| match i % 4 {
                0 => sub_add(&mut h, &loc(i), (i as u128) * 1000, (i as u128) * 3000),
                1 => materialize_and_add(&mut h, &loc(i), (i as u128) * 1000),
                2 => sub_and_materialize(&mut h, &loc(i), (i as u128) * 1000),
                _ => add(&mut h, &loc(i), i as u128),
            })
            .collect();
        h.run_block(txns);

        // Finally, check values.
        let txns: Vec<SignedTransaction> = (0..block_size)
            .map(|i| match i % 4 {
                0 => check(&mut h, &loc(i), (i as u128) * 3000),
                1 => check(&mut h, &loc(i), (i as u128) * 2000),
                2 => check(&mut h, &loc(i), 0),
                _ => check(&mut h, &loc(i), (i as u128) * 1000 + (i as u128)),
            })
            .collect();
        let outputs = h.run_block(txns);
        for status in outputs {
            assert_success!(status);
        }
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
    pub aggregator_execution_enabled: bool,
    pub block_split: BlockSplit,
}

#[allow(unused_variables)]
fn arb_test_env(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        // For execution enabled, use only whole blocks and txn-per-block for block splits, as it block split shouldn't matter there.
        Just(TestEnvConfig {
            executor_mode: ExecutorMode::Both,
            aggregator_execution_enabled: false,
            block_split: BlockSplit::Whole
        }),
        Just(TestEnvConfig {
            executor_mode: ExecutorMode::Both,
            aggregator_execution_enabled: false,
            block_split: BlockSplit::TxnPerBlock
        }),
        // For now only test whole blocks with execution enabled.
        // Just(TestEnvConfig {
        //     executor_mode: DEFAULT_EXECUTOR_MODE,
        //     aggregator_execution_enabled: true,
        //     block_split: BlockSplit::Whole
        // }),
        Just(TestEnvConfig {
            executor_mode: ExecutorMode::Both,
            aggregator_execution_enabled: true,
            block_split: BlockSplit::Whole
        }),
        // arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig{ use_parallel: true, aggregator_execution_enabled: true, block_split }),
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

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        // We will test a few more comprehensive tests more times, and the rest even fewer.
        cases: 10,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime(test_env in arb_test_env(14), element_type in arb_agg_type(), use_type in arb_use_type()) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, new(&mut h, &agg_loc, 1500)),
            (0, add(&mut h, &agg_loc, 400)),
            (0, materialize(&mut h, &agg_loc)),
            (0, add(&mut h, &agg_loc, 500)),
            (0, check(&mut h, &agg_loc, 900)),
            (0, materialize_and_add(&mut h, &agg_loc, 600)),
            (0, materialize_and_sub(&mut h, &agg_loc, 600)),
            (0, check(&mut h, &agg_loc, 900)),
            (0, sub_add(&mut h, &agg_loc, 200, 300)),
            (0, check(&mut h, &agg_loc, 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (0x02_0001, add_and_materialize(&mut h, &agg_loc, 501)),
            (0x02_0002, sub_and_materialize(&mut h, &agg_loc, 1001)),
            (0, check(&mut h, &agg_loc, 1000)),
        ];
        run_block_in_parts(
            &mut h,
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
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);
        let acc_2 = h.new_account_with_key_pair();
        let acc_3 = h.new_account_with_key_pair();

        let mut idx_1 = 0;
        let agg_1_loc = AggLocation::new(&acc, element_type, use_type, 0);
        let agg_2_loc = {
            let (cur_acc, idx_2) = if is_2_collocated { idx_1 += 1; (&acc, idx_1) } else { (&acc_2, 0)};
            AggLocation::new(cur_acc, element_type, use_type, idx_2)
        };
        let agg_3_loc = {
            let (cur_acc, idx_3) = if is_3_collocated { idx_1 += 1; (&acc, idx_1) } else { (&acc_3, 0)};
            AggLocation::new(cur_acc, element_type, use_type, idx_3)
        };
        println!("agg_1_loc: {:?}", agg_1_loc);
        println!("agg_2_loc: {:?}", agg_2_loc);
        println!("agg_3_loc: {:?}", agg_3_loc);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, init(&mut h, &acc_2, use_type, element_type, true)),
            (0, init(&mut h, &acc_3, use_type, element_type, true)),
            (0, new_add(&mut h, &agg_1_loc, 10, 5)),
            (0, new_add(&mut h, &agg_2_loc, 10, 5)),
            (0, new_add(&mut h, &agg_3_loc, 10, 5)),  // 5, 5, 5
            (0, add_2(&mut h, &agg_1_loc, &agg_2_loc, 1, 1)), // 6, 6, 5
            (0, add_2(&mut h, &agg_1_loc, &agg_3_loc, 1, 1)), // 7, 6, 6
            (0x02_0001, add(&mut h, &agg_1_loc, 5)), // X
            (0, add_sub(&mut h, &agg_1_loc, 3, 3)), // 7, 6, 6
            (0x02_0001, add_2(&mut h, &agg_1_loc, &agg_2_loc, 3, 5)), // X
            (0, add_2(&mut h, &agg_1_loc, &agg_2_loc, 3, 1)), // 10, 7, 6
            (0x02_0001, add_sub(&mut h, &agg_1_loc, 3, 3)), // X
            (0, sub(&mut h, &agg_1_loc, 3)), // 7, 7, 6
            (0, add_2(&mut h, &agg_2_loc, &agg_3_loc, 2, 2)), // 7, 9, 8
            (0, check(&mut h, &agg_2_loc, 9)),
            (0x02_0001, add_2(&mut h, &agg_1_loc, &agg_2_loc, 1, 2)), // X
            (0, add_2(&mut h, &agg_2_loc, &agg_3_loc, 1, 2)), // 7, 10, 10
            (0x02_0001, add(&mut h, &agg_2_loc, 1)), // X
            (0x02_0001, add_and_materialize(&mut h, &agg_3_loc, 1)), // X
            (0x02_0001, add_2(&mut h, &agg_1_loc, &agg_2_loc, 1, 1)), // X
            (0, check(&mut h, &agg_1_loc, 7)),
            (0, check(&mut h, &agg_2_loc, 10)),
            (0, check(&mut h, &agg_3_loc, 10)),
        ];
        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough for these
        cases: 5,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_underflow(test_env in arb_test_env(4)) {
        println!("Testing test_aggregator_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, new(&mut h, &agg_loc, 600)),
            (0, add(&mut h, &agg_loc, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            (0x02_0002, sub(&mut h, &agg_loc, 500))
        ];
        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_underflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, new(&mut h, &agg_loc, 600)),
            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            (0x02_0002, materialize_and_sub(&mut h, &agg_loc, 400)),
        ];

        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, new_add(&mut h, &agg_loc, 600, 400)),
            // Limit exceeded - abort with EAGGREGATOR_OVERFLOW.
            (0x02_0001, add(&mut h, &agg_loc, 201))
        ];

        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, new(&mut h, &agg_loc, 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            (0x02_0001, materialize_and_add(&mut h, &agg_loc, 400)),
        ];

        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_snapshot(test_env in arb_test_env(9)) {
        println!("Testing test_aggregator_snapshot {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let (mut h, acc) = setup(test_env.executor_mode, test_env.aggregator_execution_enabled);

        let agg_loc = AggLocation::new(&acc, element_type, use_type, 0);
        let snap_loc = AggLocation::new(&acc, element_type, use_type, 0);
        let derived_snap_loc = AggLocation::new(&acc, ElementType::String, use_type, 0);

        let txns = vec![
            (0, init(&mut h, &acc, use_type, element_type, true)),
            (0, init(&mut h, &acc, use_type, element_type, false)),
            (0, init(&mut h, &acc, use_type, ElementType::String, false)),
            (0, new_add(&mut h, &agg_loc, 400, 100)),
            (0, snapshot(&mut h, &agg_loc, &snap_loc)),
            (0, check_snapshot(&mut h, &snap_loc, 100)),
            (0, read_snapshot(&mut h, &agg_loc)),
            (0, add_and_read_snapshot_u128(&mut h, &agg_loc, 100)),
            (0, concat(&mut h, &snap_loc, &derived_snap_loc, "12", "13")),
            (0, check_snapshot(&mut h, &derived_snap_loc, 1210013)),
        ];

        run_block_in_parts(
            &mut h,
            test_env.block_split,
            txns,
        );
    }
}
