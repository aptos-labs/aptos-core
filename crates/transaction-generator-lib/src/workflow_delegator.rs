// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_generator::AccountGeneratorCreator, accounts_pool_wrapper::AccountsPoolWrapperCreator,
    call_custom_modules::CustomModulesDelegationGeneratorCreator,
    entry_points::EntryPointTransactionGenerator, EntryPoints, ObjectPool,
    ReliableTransactionSubmitter, TransactionGenerator, TransactionGeneratorCreator, WorkflowKind,
    tournament_generator::{TournamentStartNewRoundTransactionGenerator, TournamentMovePlayersToRoundTransactionGenerator},
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
    time::Duration, cmp,
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
    completed_for_first_stage: Arc<AtomicUsize>,
}

impl WorkflowTxnGenerator {
    fn new(
        stage: StageTracking,
        generators: Vec<Box<dyn TransactionGenerator>>,
        pool_per_stage: Vec<Arc<ObjectPool<LocalAccount>>>,
        num_for_first_stage: usize,
        completed_for_first_stage: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            stage,
            generators,
            pool_per_stage,
            num_for_first_stage,
            completed_for_first_stage,
        }
    }
}

impl TransactionGenerator for WorkflowTxnGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        mut num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        assert_ne!(num_to_create, 0);
        let mut stage = self.stage.load_current_stage();

        match &self.stage {
            StageTracking::WhenDone(stage_counter) => {
                if stage == 0 {
                    let prev = self.completed_for_first_stage.fetch_add(num_to_create, Ordering::Relaxed);
                    num_to_create = cmp::min(num_to_create, self.num_for_first_stage.saturating_sub(prev));

                    println!("TransactionGenerator Workflow: Stage 0: prev: {prev}, num_to_create: {num_to_create}");
                    if num_to_create == 0 {
                        info!("TransactionGenerator Workflow: Stage 0 is full with {} accounts, moving to stage 1", self.pool_per_stage.get(0).unwrap().len());
                        let _ =
                            stage_counter.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed);
                        stage = 1;
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
                    stage = stage + 1;
                }
            },
            StageTracking::ExternallySet(_) => {
                if stage == 0 {
                    let prev = self.completed_for_first_stage.fetch_add(num_to_create, Ordering::Relaxed);
                    num_to_create = cmp::min(num_to_create, self.num_for_first_stage.saturating_sub(prev));

                    println!("TransactionGenerator Workflow: Stage 0: prev: {prev}, num_to_create: {num_to_create}");
                    if num_to_create == 0 {
                        return Vec::new();
                    }
                }
            },
        }

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
    completed_for_first_stage: Arc<AtomicUsize>,
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
            completed_for_first_stage: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn create_workload(
        workflow_kind: WorkflowKind,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        initial_account_pool: Option<Arc<ObjectPool<LocalAccount>>>,
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
            WorkflowKind::Tournament { num_players, join_batch } => {
                let create_accounts = initial_account_pool.is_none();
                let created_pool = initial_account_pool.unwrap_or(Arc::new(ObjectPool::new()));
                let player_setup_pool = Arc::new(ObjectPool::new());
                let round_created_pool = Arc::new(ObjectPool::new());
                let in_round_pool = Arc::new(ObjectPool::new());
                let finished_pool = Arc::new(ObjectPool::new());

                let mut packages = CustomModulesDelegationGeneratorCreator::publish_package(
                    init_txn_factory.clone(),
                    root_account,
                    txn_executor,
                    num_modules,
                    EntryPoints::TournamentSetupPlayer.package_name(),
                    Some(1000_000_00000000)
                ).await;

                let tournament_setup_player_worker =  CustomModulesDelegationGeneratorCreator::create_worker(init_txn_factory.clone(), root_account, txn_executor, &mut packages, &mut EntryPointTransactionGenerator {
                    entry_point: EntryPoints::TournamentSetupPlayer,
                }).await;
                let tournament_setup_round_worker =  CustomModulesDelegationGeneratorCreator::create_worker(init_txn_factory.clone(), root_account, txn_executor, &mut packages, &mut TournamentStartNewRoundTransactionGenerator::new(
                    player_setup_pool.clone(),
                    round_created_pool.clone(),
                    join_batch,
                )).await;
                let tournament_move_players_to_round_worker =  CustomModulesDelegationGeneratorCreator::create_worker(init_txn_factory.clone(), root_account, txn_executor, &mut packages, &mut TournamentMovePlayersToRoundTransactionGenerator::new(
                    round_created_pool.clone(),
                    in_round_pool.clone(),
                    join_batch,
                )).await;
                let tournament_game_play_worker =  CustomModulesDelegationGeneratorCreator::create_worker(init_txn_factory.clone(), root_account, txn_executor, &mut packages, &mut EntryPointTransactionGenerator {
                    entry_point: EntryPoints::TournamentGamePlay,
                }).await;

                let packages = Arc::new(packages);

                let mut creators: Vec<Box<dyn TransactionGeneratorCreator>> = vec![];
                if create_accounts {
                    creators.push(
                        Box::new(AccountGeneratorCreator::new(
                            txn_factory.clone(),
                            None,
                            Some(created_pool.clone()),
                            num_players,
                            // 2 APT
                            400_000_000,
                        ))
                    );
                }

                creators.push(
                    Box::new(
                        AccountsPoolWrapperCreator::new(
                            Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                                txn_factory.clone(),
                                packages.clone(),
                                tournament_setup_player_worker,
                            )),
                            created_pool.clone(),
                            Some(player_setup_pool.clone()),
                        )
                    ),
                );
                creators.push(
                    Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                        txn_factory.clone(),
                        packages.clone(),
                        tournament_setup_round_worker,
                    )),
                );
                creators.push(
                    Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                        txn_factory.clone(),
                        packages.clone(),
                        tournament_move_players_to_round_worker,
                    )),
                );
                creators.push(
                    Box::new(
                        AccountsPoolWrapperCreator::new(
                            Box::new(CustomModulesDelegationGeneratorCreator::new_raw(
                                txn_factory.clone(),
                                packages.clone(),
                                tournament_game_play_worker,
                            )),
                            in_round_pool.clone(),
                            Some(finished_pool.clone()),
                        )
                    ),
                );

                let pool_per_stage = if create_accounts {
                    vec![created_pool, player_setup_pool, round_created_pool, in_round_pool]
                } else {
                    vec![player_setup_pool, round_created_pool, in_round_pool]
                };

                Self::new(stage_tracking, creators, pool_per_stage, num_players)
            }
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
            self.completed_for_first_stage.clone(),
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
