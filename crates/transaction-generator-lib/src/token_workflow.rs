// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_generator::AccountGeneratorCreator,
    accounts_pool_wrapper::AccountsPoolWrapperCreator,
    call_custom_modules::CustomModulesDelegationGeneratorCreator,
    entry_points::EntryPointTransactionGenerator,
    publishing::entry_point_trait::EntryPointTrait,
    workflow_delegator::{
        StageSwitchCondition, StageTracking, WorkflowKind, WorkflowTxnGeneratorCreator,
    },
    EntryPoints, ObjectPool, ReliableTransactionSubmitter, RootAccountHandle,
    TransactionGeneratorCreator,
};
use aptos_sdk::transaction_builder::TransactionFactory;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

#[derive(Debug, Copy, Clone)]
pub enum TokenWorkflowKind {
    CreateMintBurn { count: usize, creation_balance: u64 },
}

#[async_trait]
impl WorkflowKind for TokenWorkflowKind {
    async fn construct_workflow(
        &self,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        stage_tracking: StageTracking,
    ) -> WorkflowTxnGeneratorCreator {
        match self {
            TokenWorkflowKind::CreateMintBurn {
                count,
                creation_balance,
            } => {
                let created_pool = Arc::new(ObjectPool::new());
                let minted_pool = Arc::new(ObjectPool::new());
                let burnt_pool = Arc::new(ObjectPool::new());

                let mint_entry_point = EntryPoints::TokenV2AmbassadorMint { numbered: false };
                let burn_entry_point = EntryPoints::TokenV2AmbassadorBurn;

                let mut packages = CustomModulesDelegationGeneratorCreator::publish_package(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    num_modules,
                    mint_entry_point.pre_built_packages(),
                    mint_entry_point.package_name(),
                    Some(40_0000_0000),
                )
                .await;

                let mint_worker = CustomModulesDelegationGeneratorCreator::create_worker(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    &mut packages,
                    &mut EntryPointTransactionGenerator::new_singleton(Box::new(mint_entry_point)),
                )
                .await;
                let burn_worker = CustomModulesDelegationGeneratorCreator::create_worker(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    &mut packages,
                    &mut EntryPointTransactionGenerator::new_singleton(Box::new(burn_entry_point)),
                )
                .await;

                let packages = Arc::new(packages);

                let creators: Vec<Box<dyn TransactionGeneratorCreator>> = vec![
                    Box::new(AccountGeneratorCreator::new(
                        txn_factory.clone(),
                        None,
                        Some(created_pool.clone()),
                        *count,
                        *creation_balance,
                    )),
                    Box::new(AccountsPoolWrapperCreator::new(
                        Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                            txn_factory.clone(),
                            packages.clone(),
                            mint_worker,
                        )),
                        created_pool.clone(),
                        Some(minted_pool.clone()),
                    )),
                    Box::new(AccountsPoolWrapperCreator::new(
                        Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                            txn_factory.clone(),
                            packages.clone(),
                            burn_worker,
                        )),
                        minted_pool.clone(),
                        Some(burnt_pool.clone()),
                    )),
                ];
                WorkflowTxnGeneratorCreator::new(stage_tracking, creators, vec![
                    StageSwitchCondition::MaxTransactions(Arc::new(AtomicUsize::new(*count))),
                    StageSwitchCondition::WhenPoolBecomesEmpty(created_pool),
                    StageSwitchCondition::WhenPoolBecomesEmpty(minted_pool),
                ])
            },
        }
    }
}
