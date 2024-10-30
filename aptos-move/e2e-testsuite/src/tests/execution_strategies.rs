// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_language_e2e_tests::{
    account::Account,
    common_transactions::create_account_txn,
    execution_strategies::{
        basic_strategy::BasicExecutor,
        guided_strategy::{
            AnnotatedTransaction, GuidedExecutor, PartitionedGuidedStrategy,
            UnPartitionedGuidedStrategy,
        },
        multi_strategy::MultiExecutor,
        random_strategy::RandomExecutor,
        types::Executor,
    },
    feature_flags_for_orderless,
};
use aptos_types::{
    transaction::{ExecutionStatus, SignedTransaction, TransactionStatus},
    vm_status::VMStatus,
};
use rstest::rstest;

fn txn(
    sender: &Account,
    seq_num: Option<u64>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let account = Account::new();
    create_account_txn(
        sender,
        &account,
        seq_num,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
}

fn execute_and_assert_success<T>(
    exec: &mut impl Executor<Txn = T>,
    block: Vec<T>,
    num_txns: usize,
) {
    let output = exec.execute_block(block).unwrap();
    output.iter().for_each(|txn_output| {
        assert_eq!(
            txn_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );
    });
    assert_eq!(output.len(), num_txns);
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_execution_strategies(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    {
        println!("===========================================================================");
        println!("TESTING BASIC STRATEGY");
        println!("===========================================================================");
        let mut exec = BasicExecutor::new();
        exec.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let sender = exec
            .executor
            .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
        exec.executor.add_account_data(&sender);
        let big_block = (0..10)
            .map(|seq| {
                txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(seq)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )
            })
            .collect();
        execute_and_assert_success(&mut exec, big_block, 10);
    }

    {
        println!("===========================================================================");
        println!("TESTING RANDOM STRATEGY");
        println!("===========================================================================");
        let mut exec = RandomExecutor::from_os_rng();
        exec.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let sender = exec
            .executor
            .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
        exec.executor.add_account_data(&sender);
        let big_block = (0..10)
            .map(|seq| {
                txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(seq)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )
            })
            .collect();
        execute_and_assert_success(&mut exec, big_block, 10);
    }

    {
        println!("===========================================================================");
        println!("TESTING GUIDED STRATEGY");
        println!("===========================================================================");
        let mut exec = GuidedExecutor::new(PartitionedGuidedStrategy);
        exec.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let sender = exec
            .executor
            .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
        exec.executor.add_account_data(&sender);

        let mut block1: Vec<_> = (0..10)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 10)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 15)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 22)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);

        execute_and_assert_success(&mut exec, block1, 42);
    }

    {
        println!("===========================================================================");
        println!("TESTING COMPOSED STRATEGY 1");
        println!("===========================================================================");
        let mut exec = MultiExecutor::<AnnotatedTransaction, VMStatus>::new();
        let mut exec_1 = GuidedExecutor::new(PartitionedGuidedStrategy);
        let mut exec_2 = GuidedExecutor::new(UnPartitionedGuidedStrategy);
        exec_1.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        exec_2.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let sender = exec_1
            .executor
            .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
        exec_1.executor.add_account_data(&sender);
        exec_2.executor.add_account_data(&sender);
        exec.add_executor(exec_1);
        exec.add_executor(exec_2);

        let mut block1: Vec<_> = (0..10)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 10)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 15)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(i + 22)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);

        execute_and_assert_success(&mut exec, block1, 42);
    }

    {
        println!("===========================================================================");
        println!("TESTING COMPOSED STRATEGY 2");
        println!("===========================================================================");
        let mut exec = MultiExecutor::<SignedTransaction, VMStatus>::new();
        let mut exec_1 = RandomExecutor::from_os_rng();
        let mut exec_2 = RandomExecutor::from_os_rng();
        let mut exec_3 = RandomExecutor::from_os_rng();
        exec_1.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        exec_2.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        exec_3.executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let sender = exec_1
            .executor
            .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
        exec_1.executor.add_account_data(&sender);
        exec_2.executor.add_account_data(&sender);
        exec_3.executor.add_account_data(&sender);
        exec.add_executor(exec_1);
        exec.add_executor(exec_2);
        exec.add_executor(exec_3);

        let block = (0..10)
            .map(|seq| {
                txn(
                    sender.account(),
                    if use_orderless_transactions {
                        None
                    } else {
                        Some(seq)
                    },
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )
            })
            .collect();
        execute_and_assert_success(&mut exec, block, 10);
    }
}
