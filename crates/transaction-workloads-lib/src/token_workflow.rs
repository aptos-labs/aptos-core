// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::EntryPoints;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_transaction_generator_lib::{
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
    MarketplaceWorkflow { count: usize, creation_balance: u64 },
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
            TokenWorkflowKind::MarketplaceWorkflow { count, creation_balance } => {
                use aptos_transaction_generator_lib::marketplace_generator::{
                    MintNftTransactionGenerator, CreateFeeScheduleTransactionGenerator, PlaceListingTransactionGenerator
                };
                
                // Publish the on-chain-nft-marketplace package
                let marketplace_entry_point = EntryPoints::CreateNftCollection;
                let packages = Arc::new(
                    CustomModulesDelegationGeneratorCreator::publish_package(
                        init_txn_factory.clone(),
                        root_account,
                        txn_executor,
                        num_modules,
                        marketplace_entry_point.pre_built_packages(),
                        marketplace_entry_point.package_name(),
                        Some(40_0000_0000),
                    )
                    .await,
                );

                // Create the shared object pools for cross-stage communication
                let minted_token_objects = Arc::new(std::sync::RwLock::new(Vec::new()));
                let fee_schedule_objects = Arc::new(std::sync::RwLock::new(Vec::new()));

                // Create stage-specific generators with only the pools they need
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(MintNftTransactionGenerator::new(minted_token_objects.clone())),
                    Box::new(CreateFeeScheduleTransactionGenerator::new(fee_schedule_objects.clone())),
                    Box::new(PlaceListingTransactionGenerator::new(
                        minted_token_objects.clone(),
                        fee_schedule_objects.clone(),
                    )),
                ];

                // Note: Since new_staged_with_account_pool only supports account pools,
                // we'll need a custom approach for object pools. For now, this compiles
                // but we may need to create a custom workflow delegator later.
                WorkflowTxnGeneratorCreator::new_staged_with_account_pool(
                    *count,
                    *creation_balance,
                    workers,
                    None, // No loop_last_num_times
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
