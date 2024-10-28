// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_generator::AccountGeneratorCreator, accounts_pool_wrapper::{AccountsPoolWrapperCreator, BucketedAccountsPoolWrapperCreator}, call_custom_modules::CustomModulesDelegationGeneratorCreator, entry_points::EntryPointTransactionGenerator, stable_coin_minter::{
        StableCoinConfigureControllerGenerator, StableCoinMinterGenerator,
        StableCoinSetMinterAllowanceGenerator,
    }, BucketedAccountPool, EntryPoints, ObjectPool, ReliableTransactionSubmitter, RootAccountHandle, TransactionGenerator, TransactionGeneratorCreator, WorkflowKind, WorkflowProgress
};
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
    move_types::account_address::AccountAddress,
};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
enum StageTracking {
    // stage is externally modified
    ExternallySet(Arc<AtomicUsize>),
    // we move to a next stage when all accounts have finished with the current stage
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
/// We start with stage 0, which calls gen_0 stop_condition_per_stage times, which populates pool_0 with accounts.
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
    stop_condition_per_stage: Vec<StageStopCondition>,
    // Internal counter, so multiple workers (WorkflowTxnGenerator) can coordinate how many times to execute the first stage
}

impl WorkflowTxnGenerator {
    fn new(
        stage: StageTracking,
        generators: Vec<Box<dyn TransactionGenerator>>,
        stop_condition_per_stage: Vec<StageStopCondition>,
    ) -> Self {
        Self {
            stage,
            generators,
            stop_condition_per_stage,
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
                if stage < self.stop_condition_per_stage.len()
                    && self
                        .stop_condition_per_stage
                        .get(stage)
                        .unwrap()
                        .should_stop()
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
                if stage >= self.stop_condition_per_stage.len()
                    || (stage < self.stop_condition_per_stage.len()
                        && self
                            .stop_condition_per_stage
                            .get(stage)
                            .unwrap()
                            .should_stop())
                {
                    info!("TransactionGenerator Workflow: Stage {} has consumed all accounts, moving to stage {}", stage, stage + 1);
                    return Vec::new();
                }
            },
        }

        sample!(
            SampleRate::Duration(Duration::from_millis(250)),
            info!("Cur stage: {}, stop conditions per stage: {:?}", stage, self.stop_condition_per_stage);
        );

        let result = if let Some(generator) = self.generators.get_mut(stage) {
            generator.generate_transactions(account, num_to_create)
        } else {
            Vec::new()
        };
        self.stop_condition_per_stage
            .get_mut(stage)
            .map(|stop_condition| stop_condition.reduce_txn_count(result.len()));

        result
    }
}

#[derive(Clone)]
enum StageStopCondition {
    WhenPoolBecomesEmpty(Arc<ObjectPool<LocalAccount>>),
    MaxTransactions(Arc<AtomicUsize>),
}

impl StageStopCondition {
    fn should_stop(&self) -> bool {
        match self {
            StageStopCondition::WhenPoolBecomesEmpty(pool) => pool.len() == 0,
            StageStopCondition::MaxTransactions(max) => max.load(Ordering::Relaxed) == 0,
        }
    }

    fn reduce_txn_count(&mut self, count: usize) {
        match self {
            StageStopCondition::WhenPoolBecomesEmpty(_) => {},
            StageStopCondition::MaxTransactions(max) => {
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
impl Debug for StageStopCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageStopCondition::WhenPoolBecomesEmpty(pool) => {
                write!(f, "WhenPoolBecomesEmpty({})", pool.len())
            },
            StageStopCondition::MaxTransactions(max) => {
                write!(f, "MaxTransactions({})", max.load(Ordering::Relaxed))
            },
        }
    }
}

pub struct WorkflowTxnGeneratorCreator {
    stage: StageTracking,
    creators: Vec<Box<dyn TransactionGeneratorCreator>>,
    stop_condition_per_stage: Vec<StageStopCondition>,
}

impl WorkflowTxnGeneratorCreator {
    fn new(
        stage: StageTracking,
        creators: Vec<Box<dyn TransactionGeneratorCreator>>,
        stop_condition_per_stage: Vec<StageStopCondition>,
    ) -> Self {
        Self {
            stage,
            creators,
            stop_condition_per_stage,
        }
    }

    pub async fn create_workload(
        workflow_kind: WorkflowKind,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        txn_emitter_account_pool: Option<Arc<Vec<AccountAddress>>>,
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
        match workflow_kind {
            WorkflowKind::CreateMintBurn {
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
                    mint_entry_point.package_name(),
                    Some(40_0000_0000),
                )
                .await;

                let (mint_worker, mint_sequence_number_update_worker) = CustomModulesDelegationGeneratorCreator::create_worker(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    &mut packages,
                    &mut EntryPointTransactionGenerator {
                        entry_point: mint_entry_point,
                    },
                )
                .await;
                let (burn_worker, burn_sequence_number_update_worker) = CustomModulesDelegationGeneratorCreator::create_worker(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    &mut packages,
                    &mut EntryPointTransactionGenerator {
                        entry_point: burn_entry_point,
                    },
                )
                .await;

                let packages = Arc::new(packages);

                let creators: Vec<Box<dyn TransactionGeneratorCreator>> = vec![
                    Box::new(AccountGeneratorCreator::new(
                        txn_factory.clone(),
                        None,
                        Some(created_pool.clone()),
                        count,
                        creation_balance,
                    )),
                    Box::new(AccountsPoolWrapperCreator::new(
                        Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                            txn_factory.clone(),
                            packages.clone(),
                            mint_worker,
                            mint_sequence_number_update_worker,
                        )),
                        created_pool.clone(),
                        Some(minted_pool.clone()),
                    )),
                    Box::new(AccountsPoolWrapperCreator::new(
                        Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                            txn_factory.clone(),
                            packages.clone(),
                            burn_worker,
                            burn_sequence_number_update_worker,
                        )),
                        minted_pool.clone(),
                        Some(burnt_pool.clone()),
                    )),
                ];
                Self::new(stage_tracking, creators, vec![
                    StageStopCondition::MaxTransactions(Arc::new(AtomicUsize::new(count))),
                    StageStopCondition::WhenPoolBecomesEmpty(created_pool),
                    StageStopCondition::WhenPoolBecomesEmpty(minted_pool),
                ])
            },
            WorkflowKind::StableCoinMint {
                num_minter_accounts,
                num_user_accounts,
                batch_size,
                num_mint_transactions,
            } => {
                // Stages:
                // 0. Create minter accounts
                // 1. Create user accounts
                        // // 2. For each minter account, add controller in the stablecoin module
                // 2. For each minter account, set minter allowance in the stablecoin module
                // 3. Let minter accounts mint transactions for the users
                let created_minter_pool = Arc::new(ObjectPool::new());
                let destination_pool = Arc::new(ObjectPool::new());
                // let configured_minter_pool = Arc::new(ObjectPool::new());
                let minters_with_allowance_pool = Arc::new(BucketedAccountPool::new(txn_emitter_account_pool.unwrap()));

                let mut packages = CustomModulesDelegationGeneratorCreator::publish_package(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    num_modules,
                    "stablecoin",
                    Some(40_0000_0000),
                )
                .await;

                // Stage 0: Create minter accounts
                let minter_account_creation_stage = Box::new(AccountGeneratorCreator::new(
                    txn_factory.clone(),
                    None,
                    Some(created_minter_pool.clone()),
                    num_minter_accounts,
                    300_0000_0000,
                ));

                // Stage 1: Create user accounts
                let destination_account_creation_stage = Box::new(AccountGeneratorCreator::new(
                    txn_factory.clone(),
                    None,
                    Some(destination_pool.clone()),
                    num_user_accounts,
                    10_0000,
                ));

                // // Stage 2: For each minter account, add controller in the stablecoin module
                // let (configure_controllers_worker, configure_controller_stage_seq_num_updater) =
                //     CustomModulesDelegationGeneratorCreator::create_worker(
                //         init_txn_factory.clone(),
                //         root_account,
                //         txn_executor,
                //         &mut packages,
                //         &mut StableCoinConfigureControllerGenerator::default(),
                //     )
                //     .await;

                // Stage 2: For each minter account, set minter allowance in the stablecoin module
                let (set_minter_allowance_worker, set_minter_allowance_stage_seq_num_updater) =
                    CustomModulesDelegationGeneratorCreator::create_worker(
                        init_txn_factory.clone(),
                        root_account,
                        txn_executor,
                        &mut packages,
                        &mut StableCoinSetMinterAllowanceGenerator::default(),
                    )
                    .await;

                // Stage 3: Let minter accounts mint transactions for the users
                let (mint_stage_worker, mint_stage_seq_number_updater) = CustomModulesDelegationGeneratorCreator::create_worker(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    &mut packages,
                    &mut StableCoinMinterGenerator::new(
                        20,
                        batch_size,
                        minters_with_allowance_pool.clone(),
                        destination_pool.clone(),
                    ),
                )
                .await;

                let packages = Arc::new(packages);

                // let configure_controllers_stage = Box::new(AccountsPoolWrapperCreator::new(
                //     Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                //         txn_factory.clone(),
                //         packages.clone(),
                //         configure_controllers_worker,
                //         configure_controller_stage_seq_num_updater,
                //     )),
                //     created_minter_pool.clone(),
                //     Some(configured_minter_pool.clone()),
                // ));

                let set_minter_allowance_stage = Box::new(BucketedAccountsPoolWrapperCreator::new(
                    Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                        txn_factory.clone(),
                        packages.clone(),
                        set_minter_allowance_worker,
                        set_minter_allowance_stage_seq_num_updater,
                    )),
                    created_minter_pool.clone(),
                    Some(minters_with_allowance_pool.clone()),
                ));

                let mint_stage = Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                    txn_factory.clone(),
                    packages.clone(),
                    mint_stage_worker,
                    mint_stage_seq_number_updater,
                ));

                let stages: Vec<Box<dyn TransactionGeneratorCreator>> = vec![
                    minter_account_creation_stage,
                    destination_account_creation_stage,
                    // configure_controllers_stage,
                    set_minter_allowance_stage,
                    mint_stage,
                ];

                Self::new(stage_tracking, stages, vec![
                    StageStopCondition::MaxTransactions(Arc::new(AtomicUsize::new(
                        num_minter_accounts,
                    ))),
                    StageStopCondition::MaxTransactions(Arc::new(AtomicUsize::new(
                        num_user_accounts,
                    ))),
                    StageStopCondition::WhenPoolBecomesEmpty(created_minter_pool),
                    StageStopCondition::MaxTransactions(Arc::new(AtomicUsize::new(
                        num_mint_transactions,
                    ))),
                ])
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
            self.stop_condition_per_stage.clone(),
        ))
    }
}
