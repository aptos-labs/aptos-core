// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_infallible::RwLock;
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use args::TransactionTypeArg;
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

mod account_generator;
mod accounts_pool_wrapper;
pub mod args;
mod batch_transfer;
mod call_custom_modules;
mod entry_points;
mod p2p_transaction_generator;
pub mod publish_modules;
pub mod publishing;
mod transaction_mix_generator;
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
};
pub use publishing::module_simple::EntryPoints;

pub const SEND_AMOUNT: u64 = 1;

#[derive(Debug, Copy, Clone)]
pub enum TransactionType {
    NonConflictingCoinTransfer {
        invalid_transaction_ratio: usize,
        sender_use_account_pool: bool,
    },
    CoinTransfer {
        invalid_transaction_ratio: usize,
        sender_use_account_pool: bool,
    },
    AccountGeneration {
        add_created_accounts_to_pool: bool,
        max_account_working_set: usize,
        creation_balance: u64,
    },
    PublishPackage {
        use_account_pool: bool,
    },
    CallCustomModules {
        entry_point: EntryPoints,
        num_modules: usize,
        use_account_pool: bool,
    },
    BatchTransfer {
        batch_size: usize,
    },
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionTypeArg::CoinTransfer.materialize(1, false)
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

pub async fn create_txn_generator_creator(
    transaction_mix_per_phase: &[Vec<(TransactionType, usize)>],
    source_accounts: &mut [LocalAccount],
    initial_burner_accounts: Vec<LocalAccount>,
    txn_executor: &dyn ReliableTransactionSubmitter,
    txn_factory: &TransactionFactory,
    init_txn_factory: &TransactionFactory,
    cur_phase: Arc<AtomicUsize>,
) -> (
    Box<dyn TransactionGeneratorCreator>,
    Arc<RwLock<Vec<AccountAddress>>>,
    Arc<RwLock<Vec<LocalAccount>>>,
) {
    let addresses_pool = Arc::new(RwLock::new(
        source_accounts
            .iter()
            .chain(initial_burner_accounts.iter())
            .map(|d| d.address())
            .collect::<Vec<_>>(),
    ));
    let accounts_pool = Arc::new(RwLock::new(initial_burner_accounts));

    let mut txn_generator_creator_mix_per_phase: Vec<
        Vec<(Box<dyn TransactionGeneratorCreator>, usize)>,
    > = Vec::new();

    fn wrap_accounts_pool(
        inner: Box<dyn TransactionGeneratorCreator>,
        use_account_pool: bool,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    ) -> Box<dyn TransactionGeneratorCreator> {
        if use_account_pool {
            Box::new(AccountsPoolWrapperCreator::new(inner, accounts_pool))
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
                TransactionType::NonConflictingCoinTransfer {
                    invalid_transaction_ratio,
                    sender_use_account_pool,
                } => wrap_accounts_pool(
                    Box::new(P2PTransactionGeneratorCreator::new(
                        txn_factory.clone(),
                        SEND_AMOUNT,
                        addresses_pool.clone(),
                        *invalid_transaction_ratio,
                        SamplingMode::BurnAndRecycle(addresses_pool.read().len() / 2),
                    )),
                    *sender_use_account_pool,
                    accounts_pool.clone(),
                ),
                TransactionType::CoinTransfer {
                    invalid_transaction_ratio,
                    sender_use_account_pool,
                } => wrap_accounts_pool(
                    Box::new(P2PTransactionGeneratorCreator::new(
                        txn_factory.clone(),
                        SEND_AMOUNT,
                        addresses_pool.clone(),
                        *invalid_transaction_ratio,
                        SamplingMode::Basic,
                    )),
                    *sender_use_account_pool,
                    accounts_pool.clone(),
                ),
                TransactionType::AccountGeneration {
                    add_created_accounts_to_pool,
                    max_account_working_set,
                    creation_balance,
                } => Box::new(AccountGeneratorCreator::new(
                    txn_factory.clone(),
                    addresses_pool.clone(),
                    accounts_pool.clone(),
                    *add_created_accounts_to_pool,
                    *max_account_working_set,
                    *creation_balance,
                )),
                TransactionType::PublishPackage { use_account_pool } => wrap_accounts_pool(
                    Box::new(PublishPackageCreator::new(txn_factory.clone())),
                    *use_account_pool,
                    accounts_pool.clone(),
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
                            source_accounts,
                            txn_executor,
                            *num_modules,
                            entry_point.package_name(),
                            &mut EntryPointTransactionGenerator {
                                entry_point: *entry_point,
                            },
                        )
                        .await,
                    ),
                    *use_account_pool,
                    accounts_pool.clone(),
                ),
                TransactionType::BatchTransfer { batch_size } => {
                    Box::new(BatchTransferTransactionGeneratorCreator::new(
                        txn_factory.clone(),
                        SEND_AMOUNT,
                        addresses_pool.clone(),
                        *batch_size,
                    ))
                },
            };
            txn_generator_creator_mix.push((txn_generator_creator, *weight));
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

fn get_account_to_burn_from_pool(
    accounts_pool: &Arc<RwLock<Vec<LocalAccount>>>,
    needed: usize,
) -> Vec<LocalAccount> {
    let mut accounts_pool = accounts_pool.write();
    let num_in_pool = accounts_pool.len();
    if num_in_pool < needed {
        sample!(
            SampleRate::Duration(Duration::from_secs(10)),
            warn!("Cannot fetch enough accounts from pool, left in pool {}, needed {}", num_in_pool, needed);
        );
        return Vec::new();
    }
    accounts_pool
        .drain((num_in_pool - needed)..)
        .collect::<Vec<_>>()
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
