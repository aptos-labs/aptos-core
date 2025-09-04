// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::EntryPoints;
use velor_sdk::transaction_builder::TransactionFactory;
use velor_transaction_generator_lib::{
    call_custom_modules::{
        CustomModulesDelegationGeneratorCreator, UserModuleTransactionGenerator,
    },
    entry_point_trait::EntryPointTrait,
    entry_points::EntryPointTransactionGenerator,
    workflow_delegator::{StageTracking, WorkflowKind, WorkflowTxnGeneratorCreator},
    ReliableTransactionSubmitter, RootAccountHandle,
};
use async_trait::async_trait;
use std::sync::Arc;

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
                let mint_entry_point = EntryPoints::TokenV2AmbassadorMint { numbered: false };
                let burn_entry_point = EntryPoints::TokenV2AmbassadorBurn;

                let packages = Arc::new(
                    CustomModulesDelegationGeneratorCreator::publish_package(
                        init_txn_factory.clone(),
                        root_account,
                        txn_executor,
                        num_modules,
                        mint_entry_point.pre_built_packages(),
                        mint_entry_point.package_name(),
                        Some(40_0000_0000),
                    )
                    .await,
                );

                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(EntryPointTransactionGenerator::new_singleton(Box::new(
                        mint_entry_point,
                    ))),
                    Box::new(EntryPointTransactionGenerator::new_singleton(Box::new(
                        burn_entry_point,
                    ))),
                ];

                WorkflowTxnGeneratorCreator::new_staged_with_account_pool(
                    *count,
                    *creation_balance,
                    workers,
                    None,
                    packages,
                    txn_factory,
                    init_txn_factory,
                    root_account,
                    txn_executor,
                    stage_tracking,
                )
                .await
            },
        }
    }
}
