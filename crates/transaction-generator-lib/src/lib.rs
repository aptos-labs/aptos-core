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
use rand::{rngs::StdRng, seq::SliceRandom, Rng};
#[cfg(test)]
use rand_core::SeedableRng;
#[cfg(test)]
use std::collections::HashSet;
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
mod publishing;
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
        account: &mut LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction>;
}

pub trait TxnPatternGenerator: Send + Sync {
    /*
        1. The class implementing the trait should ensure that the pool is large enough.
        2. The trait is independent of the contents of the pool. In case the contents of the pool
           matter, then the class implementing the trait should handle it.
        3. The trait returns: array of [senders, [multi_sig_signers], [receivers]]
    */
    fn generate_tx_pattern(
        &mut self,
        rng: &mut StdRng,
        pool_start: usize,
        pool_size: usize,
        num_txns: usize,
        num_multi_sig_signers_per_txn: usize,
        num_receivers_per_txn: usize,
    ) -> Vec<(usize, Vec<usize>, Vec<usize>)>;
}

/// A sampler that samples a random subset of the pool. Samples are replaced immediately.
#[derive(Default)]
pub struct RandomTxnPatternGenerator {}

impl TxnPatternGenerator for RandomTxnPatternGenerator {
    fn generate_tx_pattern(
        &mut self,
        rng: &mut StdRng,
        pool_start: usize,
        pool_size: usize,
        num_txns: usize,
        num_multi_sig_signers_per_txn: usize,
        num_receivers_per_txn: usize,
    ) -> Vec<(usize, Vec<usize>, Vec<usize>)> {
        let pool_end = pool_start + pool_size;
        let mut result = Vec::new();
        for _ in 0..num_txns {
            let sender = rng.gen_range(pool_start, pool_end);
            let mut multi_sig_signers = Vec::new();
            for _ in 0..num_multi_sig_signers_per_txn {
                multi_sig_signers.push(rng.gen_range(pool_start, pool_end));
            }
            let mut receivers = Vec::new();
            for _ in 0..num_receivers_per_txn {
                receivers.push(rng.gen_range(pool_start, pool_end));
            }
            result.push((sender, multi_sig_signers, receivers));
        }
        result
    }
}

#[test]
fn test_random_txn_pattern_generator() {
    let mut rng = StdRng::from_entropy();
    let mut generator = RandomTxnPatternGenerator::default();
    let pool_size = 100;
    let num_txns = 10;
    let num_multi_sig_signers_per_txn = 2;
    let num_receivers_per_txn = 3;
    let result = generator.generate_tx_pattern(
        &mut rng,
        0,
        pool_size,
        num_txns,
        num_multi_sig_signers_per_txn,
        num_receivers_per_txn,
    );

    assert_eq!(result.len(), num_txns);
    for (sender, multi_sig_signers, receivers) in result {
        assert!(sender < pool_size);
        assert_eq!(multi_sig_signers.len(), num_multi_sig_signers_per_txn);
        assert_eq!(receivers.len(), num_receivers_per_txn);
        for signer in multi_sig_signers {
            assert!(signer < pool_size);
        }
        for receiver in receivers {
            assert!(receiver < pool_size);
        }
    }
}

pub struct ConnectedTxnPatternGenerator {
    /*
        A 'connected transaction group' is a group of transactions where all the transactions are
        connected to each other, that is they cannot be executed in parallel.
        Transactions across different groups can be executed in parallel.
    */
    num_connected_txn_grps: usize,
}

impl ConnectedTxnPatternGenerator {
    pub fn new(num_connected_txn_grps: usize) -> Self {
        Self {
            num_connected_txn_grps,
        }
    }

    fn get_connected_random_transfers(
        &mut self,
        rng: &mut StdRng,
        num_transfers: usize,
        accounts_st_idx: usize,
        accounts_end_idx: usize,
    ) -> Vec<(usize, Vec<usize>, Vec<usize>)> {
        let mut unused_indices: Vec<_> = (accounts_st_idx..=accounts_end_idx).collect();
        unused_indices.shuffle(rng);
        let mut used_indices: Vec<_> =
            vec![unused_indices.pop().unwrap(), unused_indices.pop().unwrap()];
        let mut transfer_indices: Vec<(usize, Vec<usize>, Vec<usize>)> =
            vec![(used_indices[0], vec![], vec![used_indices[1]])];

        for _ in 1..num_transfers {
            // index1 is from used_indices, so that all the txns are connected
            let mut index1 = used_indices[rng.gen_range(0, used_indices.len())];

            // index2 is either from used_indices or unused_indices
            let mut index2;
            let rnd = rng.gen_range(0, used_indices.len() + unused_indices.len());
            if rnd < used_indices.len() {
                index2 = used_indices[rnd];
            } else {
                // unused_indices is shuffled already, so last element is random
                index2 = unused_indices.pop().unwrap();
                used_indices.push(index2);
            }

            if rng.gen_range(0, 2) == 0 {
                // with 50% probability, swap the indices of sender and receiver
                (index1, index2) = (index2, index1);
            }
            transfer_indices.push((index1, vec![], vec![index2]));
        }
        transfer_indices
    }
}

impl TxnPatternGenerator for ConnectedTxnPatternGenerator {
    fn generate_tx_pattern(
        &mut self,
        rng: &mut StdRng,
        pool_start: usize,
        pool_size: usize,
        num_txns: usize,
        _: usize, // num_multi_sig_signers_per_txn == 0
        _: usize, // num_receivers_per_txn == 1
    ) -> Vec<(usize, Vec<usize>, Vec<usize>)> {
        let num_accounts_per_grp = pool_size / self.num_connected_txn_grps;

        // TODO: handle when block_size isn't divisible by num_connected_txn_grps; an easy
        //       way to do this is to just generate a few more transactions in the last group
        let num_txns_per_grp = num_txns / self.num_connected_txn_grps;

        if num_txns_per_grp >= num_accounts_per_grp {
            panic!("For the desired workload we want num_accounts_per_grp ({}) > num_txns_per_grp ({})", num_accounts_per_grp, num_txns_per_grp);
        }

        let result: Vec<_> = (0..self.num_connected_txn_grps)
            .flat_map(|grp_idx| {
                self.get_connected_random_transfers(
                    rng,
                    num_txns_per_grp,
                    pool_start + grp_idx * num_accounts_per_grp,
                    (grp_idx + 1) * num_accounts_per_grp - 1,
                )
            })
            .collect();
        result
    }
}

#[test]
fn test_connected_txn_pattern_generator() {
    let mut rng = StdRng::from_entropy();
    let num_connected_txn_grps = 3;
    let mut generator = ConnectedTxnPatternGenerator::new(num_connected_txn_grps);
    let pool_size = 100;
    let num_txns = 10;
    let result = generator.generate_tx_pattern(&mut rng, 0, pool_size, num_txns, 0, 1);

    {
        let mut adj_list: HashMap<usize, HashSet<usize>> = HashMap::new();
        assert_eq!(
            result.len(),
            num_txns / num_connected_txn_grps * num_connected_txn_grps
        );
        for (sender, multi_sig_signers, receivers) in result {
            assert!(sender < pool_size);
            assert_eq!(multi_sig_signers.len(), 0);
            assert_eq!(receivers.len(), 1);
            assert!(receivers[0] < pool_size);
            adj_list
                .entry(sender)
                .or_insert(HashSet::new())
                .insert(receivers[0]);
            adj_list
                .entry(receivers[0])
                .or_insert(HashSet::new())
                .insert(sender);
        }
        // check the number of connected components in the graph
        fn dfs(
            node: usize,
            adj_list: &HashMap<usize, HashSet<usize>>,
            visited: &mut HashSet<usize>,
        ) {
            visited.insert(node);
            for &next_node in adj_list.get(&node).unwrap() {
                if !visited.contains(&next_node) {
                    dfs(next_node, adj_list, visited);
                }
            }
        }

        let mut visited = HashSet::new();
        let mut num_connected_components = 0;
        for node in adj_list.keys() {
            if !visited.contains(node) {
                dfs(*node, &adj_list, &mut visited);
                num_connected_components += 1;
            }
        }
        assert_eq!(num_connected_components, num_connected_txn_grps);
    }
}

#[async_trait]
pub trait TransactionGeneratorCreator: Sync + Send {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator>;
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
    from: &mut LocalAccount,
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
