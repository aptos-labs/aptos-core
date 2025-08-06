// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
};
use aptos_types::{
    transaction::{ExecutionStatus, SignedTransaction, TransactionStatus},
    vm_status::VMStatus,
};
use rstest::rstest;

fn txn(seq_num: u64, current_time: u64, use_orderless_transactions: bool) -> SignedTransaction {
    let account = Account::new();
    let aptos_root = Account::new_aptos_root();
    create_account_txn(
        &aptos_root,
        &account,
        Some(seq_num),
        current_time,
        true,
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

#[rstest(use_orderless_transactions, case(false), case(true))]
fn test_execution_strategies(use_orderless_transactions: bool) {
    {
        println!("===========================================================================");
        println!("TESTING BASIC STRATEGY");
        println!("===========================================================================");
        let mut exec = BasicExecutor::new();
        let big_block = (0..10)
            .map(|i| {
                txn(
                    i,
                    exec.executor.get_block_time_seconds(),
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
        let big_block = (0..10)
            .map(|i| {
                txn(
                    i,
                    exec.executor.get_block_time_seconds(),
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
        let mut block1: Vec<_> = (0..10)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    i,
                    exec.executor.get_block_time_seconds(),
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    i + 10,
                    exec.executor.get_block_time_seconds(),
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    i + 15,
                    exec.executor.get_block_time_seconds(),
                    use_orderless_transactions,
                )))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(
                    i + 22,
                    exec.executor.get_block_time_seconds(),
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
        let mut block1: Vec<_> = (0..10)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i, 0, use_orderless_transactions))))
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(i + 10, 0, use_orderless_transactions)))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(i + 15, 0, use_orderless_transactions)))
            })
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| {
                AnnotatedTransaction::Txn(Box::new(txn(i + 22, 0, use_orderless_transactions)))
            })
            .collect();
        block1.append(&mut block);

        exec.add_executor(GuidedExecutor::new(PartitionedGuidedStrategy));
        exec.add_executor(GuidedExecutor::new(UnPartitionedGuidedStrategy));
        execute_and_assert_success(&mut exec, block1, 42);
    }

    {
        println!("===========================================================================");
        println!("TESTING COMPOSED STRATEGY 2");
        println!("===========================================================================");
        let mut exec = MultiExecutor::<SignedTransaction, VMStatus>::new();
        let block = (0..10)
            .map(|i| txn(i, 0, use_orderless_transactions))
            .collect();
        exec.add_executor(RandomExecutor::from_os_rng());
        exec.add_executor(RandomExecutor::from_os_rng());
        exec.add_executor(RandomExecutor::from_os_rng());
        execute_and_assert_success(&mut exec, block, 10);
    }
}
