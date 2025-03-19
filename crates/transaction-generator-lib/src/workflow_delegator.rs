// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ObjectPool, ReliableTransactionSubmitter, RootAccountHandle, TransactionGenerator,
    TransactionGeneratorCreator, WorkflowProgress,
};
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[async_trait::async_trait]
pub trait WorkflowKind: std::fmt::Debug + Sync + Send + CloneWorkflowKind {
    async fn construct_workflow(
        &self,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        stage_tracking: StageTracking,
    ) -> WorkflowTxnGeneratorCreator;
}

pub trait CloneWorkflowKind {
    fn clone_workflow_kind(&self) -> Box<dyn WorkflowKind>;
}

impl<T> CloneWorkflowKind for T
where
    T: WorkflowKind + Clone + 'static,
{
    fn clone_workflow_kind(&self) -> Box<dyn WorkflowKind> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn WorkflowKind> {
    fn clone(&self) -> Box<dyn WorkflowKind> {
        self.clone_workflow_kind()
    }
}

#[derive(Clone)]
pub enum StageTracking {
    // Stage is externally modified. This is used by executor benchmark tests
    ExternallySet(Arc<AtomicUsize>),
    // We move to a next stage when all accounts have finished with the current stage
    // This is used by transaction emitter (forge and tests on mainnet, devnet, testnet)
    WhenDone {
        stage_counter: Arc<AtomicUsize>,
        stage_start_time: Arc<AtomicU64>,
        delay_between_stages: Duration,
    },
}

impl StageTracking {
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn load_current_stage(&self) -> Option<usize> {
        match self {
            StageTracking::ExternallySet(stage_counter) => {
                Some(stage_counter.load(Ordering::Relaxed))
            },
            StageTracking::WhenDone {
                stage_counter,
                stage_start_time,
                ..
            } => {
                if stage_start_time.load(Ordering::Relaxed) > Self::current_timestamp() {
                    None
                } else {
                    Some(stage_counter.load(Ordering::Relaxed))
                }
            },
        }
    }
}

/// Generator allowing for multi-stage workflows.
/// List of generators are passed:
/// gen_0, gen_1, ... gen_n
/// and on list of account pools, each representing accounts in between two stages:
/// pool_0, pool_1, ... pool_n-1
///
/// pool_i is filled by gen_i, and consumed by gen_i+1, and so there is one less pools than generators.
///
/// We start with stage 0, which calls gen_0 stage_switch_conditions[0].len() times, which populates pool_0 with accounts.
///
/// After that, in stage 1, we call gen_1, which consumes accounts from pool_0, and moves them to pool_1.
/// We do this until pool_0 is empty.
///
/// We proceed, until in the last stage - stage n - calls gen_n, which consumes accounts from pool_n-1.
///
/// There are two modes on when to move to the next stage:
/// - WhenDone means as soon as pool_i is empty, we move to stage i+1
/// - ExternallySet means we wait for external signal to move to next stage, and we stop creating transactions
///   until we receive it (or will move early if pool hasn't been consumed yet)
///
/// Use WorkflowTxnGeneratorCreator::create_workload to create this generator.
struct WorkflowTxnGenerator {
    stage: StageTracking,
    generators: Vec<Box<dyn TransactionGenerator>>,
    stage_switch_conditions: Vec<StageSwitchCondition>,
}

impl WorkflowTxnGenerator {
    fn new(
        stage: StageTracking,
        generators: Vec<Box<dyn TransactionGenerator>>,
        stage_switch_conditions: Vec<StageSwitchCondition>,
    ) -> Self {
        Self {
            stage,
            generators,
            stage_switch_conditions,
        }
    }
}

impl TransactionGenerator for WorkflowTxnGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        assert_ne!(num_to_create, 0);
        let stage = match self.stage.load_current_stage() {
            Some(stage) => stage,
            None => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(2)),
                    info!("Waiting for delay before next stage");
                );
                return Vec::new();
            },
        };

        match &self.stage {
            StageTracking::WhenDone {
                stage_counter,
                stage_start_time,
                delay_between_stages,
            } => {
                if stage < self.stage_switch_conditions.len()
                    && self
                        .stage_switch_conditions
                        .get(stage)
                        .unwrap()
                        .should_switch()
                {
                    info!("TransactionGenerator Workflow: Stage {} has consumed all accounts, moving to stage {}", stage, stage + 1);
                    stage_start_time.store(
                        StageTracking::current_timestamp() + delay_between_stages.as_secs(),
                        Ordering::Relaxed,
                    );
                    let _ = stage_counter.compare_exchange(
                        stage,
                        stage + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
                    return Vec::new();
                }
            },
            StageTracking::ExternallySet(_) => {
                if stage >= self.stage_switch_conditions.len()
                    || (stage < self.stage_switch_conditions.len()
                        && self
                            .stage_switch_conditions
                            .get(stage)
                            .unwrap()
                            .should_switch())
                {
                    info!("TransactionGenerator Workflow: Stage {} has consumed all accounts, moving to stage {}", stage, stage + 1);
                    return Vec::new();
                }
            },
        }

        sample!(
            SampleRate::Duration(Duration::from_secs(2)),
            info!("Cur stage: {}, stage switch conditions: {:?}", stage, self.stage_switch_conditions);
        );

        let result = if let Some(generator) = self.generators.get_mut(stage) {
            generator.generate_transactions(account, num_to_create)
        } else {
            Vec::new()
        };
        if let Some(switch_condition) = self.stage_switch_conditions.get_mut(stage) {
            switch_condition.reduce_txn_count(result.len());
        }
        result
    }
}

#[derive(Clone)]
pub enum StageSwitchCondition {
    WhenPoolBecomesEmpty(Arc<ObjectPool<LocalAccount>>),
    MaxTransactions(Arc<AtomicUsize>),
}

impl StageSwitchCondition {
    pub fn new_max_transactions(max_transactions: usize) -> Self {
        Self::MaxTransactions(Arc::new(AtomicUsize::new(max_transactions)))
    }

    fn should_switch(&self) -> bool {
        match self {
            StageSwitchCondition::WhenPoolBecomesEmpty(pool) => pool.len() == 0,
            StageSwitchCondition::MaxTransactions(max) => max.load(Ordering::Relaxed) == 0,
        }
    }

    fn reduce_txn_count(&mut self, count: usize) {
        match self {
            StageSwitchCondition::WhenPoolBecomesEmpty(_) => {},
            StageSwitchCondition::MaxTransactions(max) => {
                let current = max.load(Ordering::Relaxed);
                if count > current {
                    max.store(0, Ordering::Relaxed);
                } else {
                    max.fetch_sub(count, Ordering::Relaxed);
                }
            },
        }
    }
}
impl Debug for StageSwitchCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageSwitchCondition::WhenPoolBecomesEmpty(pool) => {
                write!(f, "WhenPoolBecomesEmpty({})", pool.len())
            },
            StageSwitchCondition::MaxTransactions(max) => {
                write!(f, "MaxTransactions({})", max.load(Ordering::Relaxed))
            },
        }
    }
}

pub struct WorkflowTxnGeneratorCreator {
    stage: StageTracking,
    creators: Vec<Box<dyn TransactionGeneratorCreator>>,
    stage_switch_conditions: Vec<StageSwitchCondition>,
}

impl WorkflowTxnGeneratorCreator {
    pub fn new(
        stage: StageTracking,
        creators: Vec<Box<dyn TransactionGeneratorCreator>>,
        stage_switch_conditions: Vec<StageSwitchCondition>,
    ) -> Self {
        Self {
            stage,
            creators,
            stage_switch_conditions,
        }
    }

    pub async fn create_workload(
        workflow_kind: Box<dyn WorkflowKind>,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        _initial_account_pool: Option<Arc<ObjectPool<LocalAccount>>>,
        cur_phase: Arc<AtomicUsize>,
        progress_type: WorkflowProgress,
    ) -> Self {
        assert_eq!(num_modules, 1, "Only one module is supported for now");

        let stage_tracking = match progress_type {
            WorkflowProgress::MoveByPhases => StageTracking::ExternallySet(cur_phase),
            WorkflowProgress::WhenDone {
                delay_between_stages_s,
            } => StageTracking::WhenDone {
                stage_counter: Arc::new(AtomicUsize::new(0)),
                stage_start_time: Arc::new(AtomicU64::new(0)),
                delay_between_stages: Duration::from_secs(delay_between_stages_s),
            },
        };
        println!(
            "Creating workload with stage tracking: {:?}",
            match &stage_tracking {
                StageTracking::ExternallySet(_) => "ExternallySet",
                StageTracking::WhenDone { .. } => "WhenDone",
            }
        );
        workflow_kind
            .construct_workflow(
                txn_factory,
                init_txn_factory,
                root_account,
                txn_executor,
                num_modules,
                stage_tracking,
            )
            .await
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
            self.stage_switch_conditions.clone(),
        ))
    }
}
