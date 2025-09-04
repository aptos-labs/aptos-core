// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::{
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
use velor_types::{
    transaction::{ExecutionStatus, SignedTransaction, TransactionStatus},
    vm_status::VMStatus,
};

fn txn(seq_num: u64) -> SignedTransaction {
    let account = Account::new();
    let velor_root = Account::new_velor_root();
    create_account_txn(&velor_root, &account, seq_num)
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

#[test]
fn test_execution_strategies() {
    {
        println!("===========================================================================");
        println!("TESTING BASIC STRATEGY");
        println!("===========================================================================");
        let big_block = (0..10).map(txn).collect();
        let mut exec = BasicExecutor::new();
        execute_and_assert_success(&mut exec, big_block, 10);
    }

    {
        println!("===========================================================================");
        println!("TESTING RANDOM STRATEGY");
        println!("===========================================================================");
        let big_block = (0..10).map(txn).collect();
        let mut exec = RandomExecutor::from_os_rng();
        execute_and_assert_success(&mut exec, big_block, 10);
    }

    {
        println!("===========================================================================");
        println!("TESTING GUIDED STRATEGY");
        println!("===========================================================================");
        let mut block1: Vec<_> = (0..10)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i))))
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 10))))
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 15))))
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 22))))
            .collect();
        block1.append(&mut block);

        let mut exec = GuidedExecutor::new(PartitionedGuidedStrategy);
        execute_and_assert_success(&mut exec, block1, 42);
    }

    {
        println!("===========================================================================");
        println!("TESTING COMPOSED STRATEGY 1");
        println!("===========================================================================");
        let mut block1: Vec<_> = (0..10)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i))))
            .collect();
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..5)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 10))))
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block: Vec<_> = (0..7)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 15))))
            .collect();
        block1.append(&mut block);
        block1.push(AnnotatedTransaction::Block);
        let mut block = (0..20)
            .map(|i| AnnotatedTransaction::Txn(Box::new(txn(i + 22))))
            .collect();
        block1.append(&mut block);

        let mut exec = MultiExecutor::<AnnotatedTransaction, VMStatus>::new();
        exec.add_executor(GuidedExecutor::new(PartitionedGuidedStrategy));
        exec.add_executor(GuidedExecutor::new(UnPartitionedGuidedStrategy));
        execute_and_assert_success(&mut exec, block1, 42);
    }

    {
        println!("===========================================================================");
        println!("TESTING COMPOSED STRATEGY 2");
        println!("===========================================================================");
        let block = (0..10).map(txn).collect();

        let mut exec = MultiExecutor::<SignedTransaction, VMStatus>::new();
        exec.add_executor(RandomExecutor::from_os_rng());
        exec.add_executor(RandomExecutor::from_os_rng());
        exec.add_executor(RandomExecutor::from_os_rng());
        execute_and_assert_success(&mut exec, block, 10);
    }
}
