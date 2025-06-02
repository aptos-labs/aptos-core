// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_infallible::{RwLock, RwLockWriteGuard};
use aptos_logger::{sample, sample::SampleRate};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use clap::{Parser, ValueEnum};
use log::{info, warn};
use publishing::{
    entry_point_trait::{EntryPointTrait, PreBuiltPackages},
    publish_util::PackageHandler,
};
use rand::{rngs::StdRng, seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use workflow_delegator::WorkflowKind;

pub mod account_generator;
pub mod accounts_pool_wrapper;
mod batch_transfer;
mod bounded_batch_wrapper;
pub mod call_custom_modules;
pub mod entry_points;
mod p2p_transaction_generator;
pub mod publish_modules;
pub mod publishing;
mod transaction_mix_generator;
pub mod workflow_delegator;
use self::{
    account_generator::AccountGeneratorCreator,
    call_custom_modules::CustomModulesDelegationGeneratorCreator,
    p2p_transaction_generator::P2PTransactionGeneratorCreator,
    publish_modules::PublishPackageCreator,
    transaction_mix_generator::PhasedTxnMixGeneratorCreator,
};
use crate::{
    accounts_pool_wrapper::AccountsPoolWrapperCreator,
    batch_transfer::BatchTransferTransactionGeneratorCreator,
    entry_points::EntryPointTransactionGenerator, p2p_transaction_generator::SamplingMode,
    workflow_delegator::WorkflowTxnGeneratorCreator,
};
pub use publishing::{entry_point_trait, prebuild_packages::create_prebuilt_packages_rs_file};

pub const SEND_AMOUNT: u64 = 1;

#[derive(Debug, Clone)]
pub enum TransactionType {
    CoinTransfer {
        invalid_transaction_ratio: usize,
        sender_use_account_pool: bool,
        non_conflicting: bool,
        use_fa_transfer: bool,
    },
    AccountGeneration {
        add_created_accounts_to_pool: bool,
        max_account_working_set: usize,
        creation_balance: u64,
    },
    PublishPackage {
        use_account_pool: bool,
        pre_built: &'static dyn PreBuiltPackages,
        package_name: String,
    },
    CallCustomModules {
        entry_point: Box<dyn EntryPointTrait>,
        num_modules: usize,
        use_account_pool: bool,
    },
    CallCustomModulesMix {
        entry_points: Vec<(Box<dyn EntryPointTrait>, usize)>,
        num_modules: usize,
        use_account_pool: bool,
    },
    BatchTransfer {
        batch_size: usize,
    },
    Workflow {
        workflow_kind: Box<dyn WorkflowKind>,
        num_modules: usize,
        use_account_pool: bool,
        progress_type: WorkflowProgress,
    },
}

#[derive(Debug, Copy, Clone, ValueEnum, Default, Deserialize, Parser, Serialize)]
pub enum AccountType {
    #[default]
    Local,
    Keyless,
}

#[derive(Debug, Copy, Clone)]
pub enum WorkflowProgress {
    MoveByPhases,
    WhenDone { delay_between_stages_s: u64 },
}

impl WorkflowProgress {
    pub fn when_done_default() -> Self {
        Self::WhenDone {
            delay_between_stages_s: 10,
        }
    }
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionType::CoinTransfer {
            invalid_transaction_ratio: 0,
            sender_use_account_pool: false,
            non_conflicting: false,
            use_fa_transfer: true,
        }
    }
}

pub trait TransactionGenerator: Sync + Send {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction>;
}

#[async_trait]
pub trait TransactionGeneratorCreator: Sync + Send {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator>;
}

pub struct CounterState {
    pub submit_failures: Vec<AtomicUsize>,
    pub wait_failures: Vec<AtomicUsize>,
    pub successes: AtomicUsize,
    // (success, submit_fail, wait_fail)
    pub by_client: HashMap<String, (AtomicUsize, AtomicUsize, AtomicUsize)>,
}

#[async_trait]
pub trait ReliableTransactionSubmitter: Sync + Send {
    async fn get_account_balance(&self, account_address: AccountAddress) -> Result<u64>;

    async fn query_sequence_number(&self, account_address: AccountAddress) -> Result<u64>;

    async fn execute_transactions(&self, txns: &[SignedTransaction]) -> Result<()> {
        self.execute_transactions_with_counter(txns, &CounterState {
            submit_failures: vec![AtomicUsize::new(0)],
            wait_failures: vec![AtomicUsize::new(0)],
            successes: AtomicUsize::new(0),
            by_client: HashMap::new(),
        })
        .await
    }

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        state: &CounterState,
    ) -> Result<()>;

    fn create_counter_state(&self) -> CounterState;
}

fn failed_requests_to_trimmed_vec(failed_requests: &[AtomicUsize]) -> Vec<usize> {
    let mut result = failed_requests
        .iter()
        .map(|c| c.load(Ordering::Relaxed))
        .collect::<Vec<_>>();
    while result.len() > 1 && *result.last().unwrap() == 0 {
        result.pop();
    }
    result
}

impl CounterState {
    pub fn show_simple(&self) -> String {
        format!(
            "success {}, failed submit {:?}, failed wait {:?}",
            self.successes.load(Ordering::Relaxed),
            failed_requests_to_trimmed_vec(&self.submit_failures),
            failed_requests_to_trimmed_vec(&self.wait_failures)
        )
    }

    pub fn show_detailed(&self) -> String {
        format!(
            "{}, by client: {}",
            self.show_simple(),
            self.by_client
                .iter()
                .flat_map(|(name, (success, fail_submit, fail_wait))| {
                    let num_s = success.load(Ordering::Relaxed);
                    let num_fs = fail_submit.load(Ordering::Relaxed);
                    let num_fw = fail_wait.load(Ordering::Relaxed);
                    if num_fs + num_fw > 0 {
                        Some(format!("[({}, {}, {}): {}]", num_s, num_fs, num_fw, name))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[async_trait::async_trait]
pub trait RootAccountHandle: Send + Sync {
    async fn approve_funds(&self, amount: u64, reason: &str);

    fn get_root_account(&self) -> Arc<LocalAccount>;
}

pub struct AlwaysApproveRootAccountHandle {
    pub root_account: Arc<LocalAccount>,
}

#[async_trait::async_trait]
impl RootAccountHandle for AlwaysApproveRootAccountHandle {
    async fn approve_funds(&self, amount: u64, reason: &str) {
        println!(
            "Consuming funds from root/source account: up to {} for {}",
            amount, reason
        );
    }

    fn get_root_account(&self) -> Arc<LocalAccount> {
        self.root_account.clone()
    }
}

pub async fn create_txn_generator_creator(
    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
    root_account: impl RootAccountHandle,
    source_accounts: &mut [LocalAccount],
    initial_burner_accounts: Vec<LocalAccount>,
    txn_executor: &dyn ReliableTransactionSubmitter,
    txn_factory: &TransactionFactory,
    init_txn_factory: &TransactionFactory,
    cur_phase: Arc<AtomicUsize>,
) -> (
    Box<dyn TransactionGeneratorCreator>,
    Arc<ObjectPool<AccountAddress>>,
    Arc<ObjectPool<LocalAccount>>,
) {
    let addresses_pool = Arc::new(ObjectPool::new_initial(
        source_accounts
            .iter()
            .chain(initial_burner_accounts.iter())
            .map(|d| d.address())
            .collect(),
    ));
    let accounts_pool = Arc::new(ObjectPool::new_initial(initial_burner_accounts));

    let mut txn_generator_creator_mix_per_phase: Vec<
        Vec<(Box<dyn TransactionGeneratorCreator>, usize)>,
    > = Vec::new();

    fn wrap_accounts_pool(
        inner: Box<dyn TransactionGeneratorCreator>,
        use_account_pool: bool,
        accounts_pool: &Arc<ObjectPool<LocalAccount>>,
    ) -> Box<dyn TransactionGeneratorCreator> {
        if use_account_pool {
            Box::new(AccountsPoolWrapperCreator::new(
                inner,
                accounts_pool.clone(),
                None,
            ))
        } else {
            inner
        }
    }

    for transaction_mix in transaction_mix_per_phase {
        let mut txn_generator_creator_mix: Vec<(Box<dyn TransactionGeneratorCreator>, usize)> =
            Vec::new();
        for (transaction_type, weight) in transaction_mix {
            let txn_generator_creator: Box<dyn TransactionGeneratorCreator> = match transaction_type
            {
                TransactionType::CoinTransfer {
                    invalid_transaction_ratio,
                    sender_use_account_pool,
                    non_conflicting,
                    use_fa_transfer,
                } => wrap_accounts_pool(
                    Box::new(P2PTransactionGeneratorCreator::new(
                        txn_factory.clone(),
                        SEND_AMOUNT,
                        addresses_pool.clone(),
                        invalid_transaction_ratio,
                        use_fa_transfer,
                        if non_conflicting {
                            SamplingMode::BurnAndRecycle(addresses_pool.len() / 2)
                        } else {
                            SamplingMode::Basic
                        },
                    )),
                    sender_use_account_pool,
                    &accounts_pool,
                ),
                TransactionType::AccountGeneration {
                    add_created_accounts_to_pool,
                    max_account_working_set,
                    creation_balance,
                } => Box::new(AccountGeneratorCreator::new(
                    txn_factory.clone(),
                    add_created_accounts_to_pool.then(|| {
                        addresses_pool.reserve(max_account_working_set);
                        addresses_pool.clone()
                    }),
                    add_created_accounts_to_pool.then(|| {
                        addresses_pool.reserve(max_account_working_set);
                        accounts_pool.clone()
                    }),
                    max_account_working_set,
                    creation_balance,
                )),
                TransactionType::PublishPackage {
                    use_account_pool,
                    pre_built,
                    package_name,
                } => wrap_accounts_pool(
                    Box::new(PublishPackageCreator::new(
                        txn_factory.clone(),
                        PackageHandler::new(pre_built, &package_name),
                    )),
                    use_account_pool,
                    &accounts_pool,
                ),
                TransactionType::CallCustomModules {
                    entry_point,
                    num_modules,
                    use_account_pool,
                } => wrap_accounts_pool(
                    Box::new(
                        CustomModulesDelegationGeneratorCreator::new(
                            txn_factory.clone(),
                            init_txn_factory.clone(),
                            &root_account,
                            txn_executor,
                            num_modules,
                            entry_point.pre_built_packages(),
                            entry_point.package_name(),
                            &mut EntryPointTransactionGenerator::new_singleton(entry_point),
                        )
                        .await,
                    ),
                    use_account_pool,
                    &accounts_pool,
                ),
                TransactionType::CallCustomModulesMix {
                    entry_points,
                    num_modules,
                    use_account_pool,
                } => wrap_accounts_pool(
                    Box::new(
                        CustomModulesDelegationGeneratorCreator::new(
                            txn_factory.clone(),
                            init_txn_factory.clone(),
                            &root_account,
                            txn_executor,
                            num_modules,
                            entry_points[0].0.pre_built_packages(),
                            entry_points[0].0.package_name(),
                            &mut EntryPointTransactionGenerator::new(entry_points),
                        )
                        .await,
                    ),
                    use_account_pool,
                    &accounts_pool,
                ),
                TransactionType::BatchTransfer { batch_size } => {
                    Box::new(BatchTransferTransactionGeneratorCreator::new(
                        txn_factory.clone(),
                        SEND_AMOUNT,
                        addresses_pool.clone(),
                        batch_size,
                    ))
                },
                TransactionType::Workflow {
                    num_modules,
                    use_account_pool,
                    workflow_kind,
                    progress_type,
                } => Box::new(
                    WorkflowTxnGeneratorCreator::create_workload(
                        workflow_kind,
                        txn_factory.clone(),
                        init_txn_factory.clone(),
                        &root_account,
                        txn_executor,
                        num_modules,
                        use_account_pool.then(|| accounts_pool.clone()),
                        cur_phase.clone(),
                        progress_type,
                    )
                    .await,
                ),
            };
            txn_generator_creator_mix.push((txn_generator_creator, weight));
        }
        txn_generator_creator_mix_per_phase.push(txn_generator_creator_mix)
    }

    (
        Box::new(PhasedTxnMixGeneratorCreator::new(
            txn_generator_creator_mix_per_phase,
            cur_phase,
        )),
        addresses_pool,
        accounts_pool,
    )
}

/// Simple object pool structure, that you can add and remove from multiple threads.
/// Taking is done at random positions, but sequentially.
/// Overflow replaces at random positions as well.
///
/// It's efficient to lock the objects for short time - and replace
/// in place, but its not a concurrent datastructure.
pub struct ObjectPool<T> {
    pool: RwLock<Vec<T>>,
}

impl<T> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ObjectPool<T> {
    pub fn new_initial(initial: Vec<T>) -> Self {
        Self {
            pool: RwLock::new(initial),
        }
    }

    pub fn new() -> Self {
        Self::new_initial(Vec::new())
    }

    pub(crate) fn reserve(&self, additional: usize) {
        self.pool.write().reserve(additional);
    }

    pub fn add_to_pool(&self, mut addition: Vec<T>) {
        assert!(!addition.is_empty());
        let mut current = self.pool.write();
        current.append(&mut addition);
        sample!(
            SampleRate::Duration(Duration::from_secs(120)),
            info!("Pool working set increased to {}", current.len())
        );
    }

    pub fn add_to_pool_bounded(&self, mut addition: Vec<T>, max_size: usize, rng: &mut StdRng) {
        assert!(!addition.is_empty());
        assert!(addition.len() <= max_size);

        let mut current = self.pool.write();
        if current.len() < max_size {
            if current.len() + addition.len() > max_size {
                addition.truncate(max_size - current.len());
            }
            current.append(&mut addition);
            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                info!("Pool working set increased to {}", current.len())
            );
        } else {
            // no underflow as: addition.len() <= max_size < current.len()
            let start = rng.gen_range(0, current.len() - addition.len());
            current[start..start + addition.len()].swap_with_slice(&mut addition);

            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                info!(
                    "Already at limit {} > {}, so exchanged objects in working set",
                    current.len(),
                    max_size
                )
            );
        }
    }

    pub fn take_from_pool(&self, needed: usize, return_partial: bool, rng: &mut StdRng) -> Vec<T> {
        let mut current = self.pool.write();
        let num_in_pool = current.len();
        if !return_partial && num_in_pool < needed {
            sample!(
                SampleRate::Duration(Duration::from_secs(10)),
                warn!("Cannot fetch enough from shared pool, left in pool {}, needed {}", num_in_pool, needed);
            );
            return Vec::new();
        }
        let num_to_return = std::cmp::min(num_in_pool, needed);
        let mut result = current
            .drain((num_in_pool - num_to_return)..)
            .collect::<Vec<_>>();

        if current.len() > num_to_return {
            let start = rng.gen_range(0, current.len() - num_to_return);
            current[start..start + num_to_return].swap_with_slice(&mut result);
        }
        result
    }

    pub(crate) fn shuffle(&self, rng: &mut StdRng) {
        self.pool.write().shuffle(rng);
    }

    pub(crate) fn write_view(&self) -> RwLockWriteGuard<'_, Vec<T>> {
        self.pool.write()
    }

    pub(crate) fn len(&self) -> usize {
        self.pool.read().len()
    }
}

impl<T: Clone> ObjectPool<T> {
    pub(crate) fn clone_from_pool(&self, num_to_copy: usize, rng: &mut StdRng) -> Vec<T> {
        self.pool
            .read()
            .choose_multiple(rng, num_to_copy)
            .cloned()
            .collect::<Vec<_>>()
    }
}

pub fn create_account_transaction(
    from: &LocalAccount,
    to: AccountAddress,
    txn_factory: &TransactionFactory,
    creation_balance: u64,
) -> SignedTransaction {
    from.sign_with_transaction_builder(txn_factory.payload(
        if creation_balance > 0 {
            aptos_stdlib::aptos_account_transfer(to, creation_balance)
        } else {
            aptos_stdlib::aptos_account_create_account(to)
        },
    ))
}
