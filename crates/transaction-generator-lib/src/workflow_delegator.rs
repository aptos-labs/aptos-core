// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_generator::AccountGeneratorCreator, accounts_pool_wrapper::AccountsPoolWrapperCreator,
    call_custom_modules::CustomModulesDelegationGeneratorCreator,
    entry_points::EntryPointTransactionGenerator, EntryPoints, ObjectPool,
    ReliableTransactionSubmitter, TransactionGenerator, TransactionGeneratorCreator, WorkflowKind,
};
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

/// Wrapper that allows inner transaction generator to have unique accounts
/// for all transactions (instead of having 5-20 transactions per account, as default)
/// This is achieved via using accounts from the pool that account creatin can fill,
/// and burning (removing accounts from the pool) them - basically using them only once.
/// (we cannot use more as sequence number is not updated on failure)
struct WorkflowTxnGenerator {
    stage: StageTracking,
    generators: Vec<Box<dyn TransactionGenerator>>,
    pool_per_stage: Vec<Arc<ObjectPool<LocalAccount>>>,
    num_for_first_stage: usize,
}

impl WorkflowTxnGenerator {
    fn new(
        stage: StageTracking,
        generators: Vec<Box<dyn TransactionGenerator>>,
        pool_per_stage: Vec<Arc<ObjectPool<LocalAccount>>>,
        num_for_first_stage: usize,
    ) -> Self {
        Self {
            stage,
            generators,
            pool_per_stage,
            num_for_first_stage,
        }
    }
}

impl TransactionGenerator for WorkflowTxnGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        if let StageTracking::WhenDone(stage_counter) = &self.stage {
            let stage = stage_counter.load(Ordering::Relaxed);
            if stage == 0 {
                if self.pool_per_stage.get(0).unwrap().len() >= self.num_for_first_stage {
                    info!("TransactionGenerator Workflow: Stage 0 is full with {} accounts, moving to stage 1", self.pool_per_stage.get(0).unwrap().len());
                    let _ =
                        stage_counter.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed);
                }
            } else if stage < self.pool_per_stage.len()
                && self.pool_per_stage.get(stage - 1).unwrap().len() == 0
            {
                info!("TransactionGenerator Workflow: Stage {} has consumed all accounts, moving to stage {}", stage, stage + 1);
                let _ = stage_counter.compare_exchange(
                    stage,
                    stage + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );
            }
        }

        let stage = self.stage.load_current_stage();

        sample!(
            SampleRate::Duration(Duration::from_millis(500)),
            info!("Cur stage: {}, pool sizes: {:?}", stage, self.pool_per_stage.iter().map(|p| p.len()).collect::<Vec<_>>());
        );

        let result = if let Some(generator) = self.generators.get_mut(stage) {
            generator.generate_transactions(account, num_to_create)
        } else {
            Vec::new()
        };

        result
    }
}

pub struct WorkflowTxnGeneratorCreator {
    stage: StageTracking,
    creators: Vec<Box<dyn TransactionGeneratorCreator>>,
    pool_per_stage: Vec<Arc<ObjectPool<LocalAccount>>>,
    num_for_first_stage: usize,
}

impl WorkflowTxnGeneratorCreator {
    fn new(
        stage: StageTracking,
        creators: Vec<Box<dyn TransactionGeneratorCreator>>,
        pool_per_stage: Vec<Arc<ObjectPool<LocalAccount>>>,
        num_for_first_stage: usize,
    ) -> Self {
        Self {
            stage,
            creators,
            pool_per_stage,
            num_for_first_stage,
        }
    }

    pub async fn create_workload(
        workflow_kind: WorkflowKind,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        cur_phase: Option<Arc<AtomicUsize>>,
    ) -> Self {
        let stage_tracking = cur_phase.map_or_else(
            || StageTracking::WhenDone(Arc::new(AtomicUsize::new(0))),
            StageTracking::ExternallySet,
        );
        match workflow_kind {
            WorkflowKind::CreateThenMint {
                count,
                creation_balance,
            } => {
                let created_pool = Arc::new(ObjectPool::new());
                let minted_pool = Arc::new(ObjectPool::new());
                let entry_point = EntryPoints::TokenV2AmbassadorMint;

                let creators: Vec<Box<dyn TransactionGeneratorCreator>> = vec![
                    Box::new(AccountGeneratorCreator::new(
                        txn_factory.clone(),
                        None,
                        Some(created_pool.clone()),
                        count,
                        creation_balance,
                    )),
                    Box::new(AccountsPoolWrapperCreator::new(
                        Box::new(
                            CustomModulesDelegationGeneratorCreator::new(
                                txn_factory.clone(),
                                init_txn_factory.clone(),
                                root_account,
                                txn_executor,
                                num_modules,
                                entry_point.package_name(),
                                &mut EntryPointTransactionGenerator { entry_point },
                            )
                            .await,
                        ),
                        created_pool.clone(),
                        Some(minted_pool.clone()),
                    )),
                ];
                Self::new(
                    stage_tracking,
                    creators,
                    vec![created_pool, minted_pool],
                    count,
                )
            },
        }
    }
}

impl TransactionGeneratorCreator for WorkflowTxnGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(WorkflowTxnGenerator::new(
            self.stage.clone(),
            self.creators
                .iter()
                .map(|c| c.create_transaction_generator())
                .collect(),
            self.pool_per_stage.clone(),
            self.num_for_first_stage,
        ))
    }
}

#[derive(Clone)]
enum StageTracking {
    // phase is externally modified
    ExternallySet(Arc<AtomicUsize>),
    WhenDone(Arc<AtomicUsize>),
}

impl StageTracking {
    fn load_current_stage(&self) -> usize {
        match self {
            StageTracking::ExternallySet(stage) | StageTracking::WhenDone(stage) => {
                stage.load(Ordering::Relaxed)
            },
        }
    }
}
