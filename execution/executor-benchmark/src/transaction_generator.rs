// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_generator::{AccountCache, AccountGenerator},
    metrics::{NUM_TXNS, TIMER},
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_logger::info;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::LocalAccount,
};
use aptos_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReader, DbReaderWriter,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{aptos_test_root_address, AccountResource},
    chain_id::ChainId,
    state_store::MoveResourceExt,
    transaction::{EntryFunction, Transaction, TransactionPayload},
};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use move_core_types::{ident_str, language_storage::ModuleId};
#[cfg(test)]
use rand::SeedableRng;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    ThreadPool, ThreadPoolBuilder,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::HashSet;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
};
use thread_local::ThreadLocal;

const META_FILENAME: &str = "metadata.toml";
pub const MAX_ACCOUNTS_INVOLVED_IN_P2P: usize = 1_000_000;

pub(crate) fn get_progress_bar(num_accounts: usize) -> ProgressBar {
    let bar = ProgressBar::new(num_accounts as u64);
    bar.set_style(ProgressStyle::default_bar().template(
        "[{elapsed_precise} {per_sec}] {bar:100.cyan/blue} {percent}% ETA {eta_precise}",
    ));
    bar
}

fn get_sequence_number(address: AccountAddress, reader: Arc<dyn DbReader>) -> u64 {
    let db_state_view = reader.latest_state_checkpoint_view().unwrap();

    match AccountResource::fetch_move_resource(&db_state_view, &address).unwrap() {
        Some(account_resource) => account_resource.sequence_number(),
        None => 0,
    }
}

macro_rules! now_fmt {
    () => {
        Local::now().format("%m-%d %H:%M:%S")
    };
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "args")]
enum TestCase {
    P2p(P2pTestCase),
}

#[derive(Serialize, Deserialize)]
struct P2pTestCase {
    num_accounts: usize,
}

pub struct TransactionGenerator {
    /// The current state of the accounts. The main purpose is to keep track of the sequence number
    /// so generated transactions are guaranteed to be successfully executed.
    main_signer_accounts: Option<AccountCache>,

    /// The current state of seed accounts. The purpose of the seed accounts to parallelize the
    /// account creation and minting process so that they are not blocked on sequence number of
    /// a single root account.
    seed_accounts_cache: Option<AccountCache>,

    /// Total # of existing (non-seed) accounts in the DB at the time of TransactionGenerator
    /// creation.
    num_existing_accounts: usize,

    /// Each generated block of transactions are sent to this channel. Using `SyncSender` to make
    /// sure if execution is slow to consume the transactions, we do not run out of memory.
    block_sender: Option<mpsc::SyncSender<Vec<Transaction>>>,

    /// Transaction Factory
    transaction_factory: TransactionFactory,

    /// root account is used across creating and minting.
    root_account: LocalAccount,

    /// # of workers used to generate transactions.
    num_workers: usize,
    // TODO(grao): Use a different pool, and pin threads to dedicate cores to avoid affecting the
    // rest parts of benchmark.
    worker_pool: ThreadPool,
}

impl TransactionGenerator {
    fn gen_account_cache(
        generator: AccountGenerator,
        num_accounts: usize,
        name: &str,
    ) -> AccountCache {
        println!(
            "[{}] Generating cache of {} {} accounts.",
            now_fmt!(),
            num_accounts,
            name,
        );
        AccountCache::new(generator, num_accounts)
    }

    pub fn resync_sequence_numbers(
        reader: Arc<dyn DbReader>,
        mut accounts: AccountCache,
        name: &str,
    ) -> AccountCache {
        let mut updated = 0;
        for account in &mut accounts.accounts {
            let seq_num = get_sequence_number(account.address(), reader.clone());
            if seq_num > 0 {
                updated += 1;
                account.set_sequence_number(seq_num);
            }
        }
        if updated > 0 {
            println!(
                "Updated {} seq numbers out of {} {} accounts",
                updated,
                accounts.accounts.len(),
                name
            );
        }
        accounts
    }

    pub fn gen_user_account_cache(
        reader: Arc<dyn DbReader>,
        num_accounts: usize,
        num_to_skip: usize,
        is_keyless: bool,
    ) -> AccountCache {
        Self::resync_sequence_numbers(
            reader,
            Self::gen_account_cache(
                AccountGenerator::new_for_user_accounts(num_to_skip as u64, is_keyless),
                num_accounts,
                "user",
            ),
            "user",
        )
    }

    fn gen_seed_account_cache(
        reader: Arc<dyn DbReader>,
        num_accounts: usize,
        is_keyless: bool,
    ) -> AccountCache {
        Self::resync_sequence_numbers(
            reader,
            Self::gen_account_cache(
                AccountGenerator::new_for_seed_accounts(is_keyless),
                num_accounts,
                "seed",
            ),
            "seed",
        )
    }

    pub fn new_with_existing_db<P: AsRef<Path>>(
        db: DbReaderWriter,
        root_account: LocalAccount,
        block_sender: mpsc::SyncSender<Vec<Transaction>>,
        db_dir: P,
        num_main_signer_accounts: Option<usize>,
        num_workers: usize,
        is_keyless: bool,
    ) -> Self {
        let num_existing_accounts = TransactionGenerator::read_meta(&db_dir);

        Self {
            seed_accounts_cache: None,
            root_account,
            main_signer_accounts: num_main_signer_accounts.map(|num_main_signer_accounts| {
                let num_cached_accounts =
                    std::cmp::min(num_existing_accounts, num_main_signer_accounts);
                Self::gen_user_account_cache(db.reader.clone(), num_cached_accounts, 0, is_keyless)
            }),
            num_existing_accounts,
            block_sender: Some(block_sender),
            transaction_factory: Self::create_transaction_factory(),
            num_workers,
            worker_pool: ThreadPoolBuilder::new()
                .num_threads(num_workers)
                .build()
                .unwrap(),
        }
    }

    pub fn create_transaction_factory() -> TransactionFactory {
        TransactionFactory::new(ChainId::test())
            .with_transaction_expiration_time(300)
            .with_gas_unit_price(100)
    }

    // Write metadata
    pub fn write_meta<P: AsRef<Path>>(self, path: &P, num_new_accounts: usize) {
        let metadata = TestCase::P2p(P2pTestCase {
            num_accounts: self.num_existing_accounts + num_new_accounts,
        });
        let serialized = toml::ser::to_string(&metadata).unwrap();
        let meta_file = path.as_ref().join(META_FILENAME);
        let mut file = File::create(meta_file).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }

    pub fn read_meta<P: AsRef<Path>>(path: &P) -> usize {
        let filename = path.as_ref().join(META_FILENAME);
        File::open(filename).map_or(0, |mut file| {
            let mut contents = vec![];
            file.read_to_end(&mut contents).unwrap();
            let test_case: TestCase =
                toml::from_str(&String::from_utf8(contents).expect("Must be UTF8"))
                    .expect("Must exist.");
            let TestCase::P2p(P2pTestCase { num_accounts }) = test_case;
            num_accounts
        })
    }

    pub fn read_root_account(genesis_key: Ed25519PrivateKey, db: &DbReaderWriter) -> LocalAccount {
        LocalAccount::new(
            aptos_test_root_address(),
            genesis_key,
            get_sequence_number(aptos_test_root_address(), db.reader.clone()),
        )
    }

    pub fn num_existing_accounts(&self) -> usize {
        self.num_existing_accounts
    }

    pub fn run_mint(
        &mut self,
        reader: Arc<dyn DbReader>,
        num_existing_accounts: usize,
        num_new_accounts: usize,
        init_account_balance: u64,
        block_size: usize,
        is_keyless: bool,
    ) {
        assert!(self.block_sender.is_some());
        // Ensure that seed accounts have enough balance to transfer money to at least 10000 account with
        // balance init_account_balance.
        self.create_seed_accounts(
            reader,
            num_new_accounts,
            block_size,
            init_account_balance * 10_000,
            is_keyless,
        );
        self.create_and_fund_accounts(
            num_existing_accounts,
            num_new_accounts,
            init_account_balance,
            block_size,
            is_keyless,
        );
    }

    pub fn run_transfer(
        &mut self,
        block_size: usize,
        num_transfer_blocks: usize,
        transactions_per_sender: usize,
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
        hotspot_probability: Option<f32>,
    ) -> usize {
        assert!(self.block_sender.is_some());
        self.gen_transfer_transactions(
            block_size,
            num_transfer_blocks,
            transactions_per_sender,
            connected_tx_grps,
            shuffle_connected_txns,
            hotspot_probability,
        );
        num_transfer_blocks
    }

    pub fn run_workload(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        transaction_generators: Vec<Box<dyn aptos_transaction_generator_lib::TransactionGenerator>>,
        phase: Arc<AtomicUsize>,
        transactions_per_sender: usize,
    ) -> usize {
        let last_non_empty_phase = Arc::new(AtomicUsize::new(0));
        let transaction_generators = Mutex::new(transaction_generators);
        assert!(self.block_sender.is_some());
        let num_senders_per_block = block_size.div_ceil(transactions_per_sender);
        let account_pool_size = self.main_signer_accounts.as_ref().unwrap().accounts.len();
        let transaction_generator = ThreadLocal::with_capacity(self.num_workers);
        for i in 0..num_blocks {
            let sender_indices = rand::seq::index::sample(
                &mut thread_rng(),
                account_pool_size,
                num_senders_per_block,
            )
            .into_iter()
            .flat_map(|sender_idx| vec![sender_idx; transactions_per_sender])
            .collect();
            let terminate = self.generate_and_send_block(
                self.main_signer_accounts.as_ref().unwrap(),
                sender_indices,
                phase.clone(),
                last_non_empty_phase.clone(),
                |sender_idx, _| {
                    let sender = &self.main_signer_accounts.as_ref().unwrap().accounts[sender_idx];
                    let mut transaction_generator = transaction_generator
                        .get_or(|| {
                            RefCell::new(transaction_generators.lock().unwrap().pop().unwrap())
                        })
                        .borrow_mut();
                    transaction_generator
                        .generate_transactions(sender, 1)
                        .pop()
                        .map(Transaction::UserTransaction)
                },
                |sender_idx| *sender_idx,
            );
            if terminate {
                return i + 1;
            }
        }
        num_blocks
    }

    pub fn create_seed_accounts(
        &mut self,
        reader: Arc<dyn DbReader>,
        num_new_accounts: usize,
        block_size: usize,
        seed_account_balance: u64,
        is_keyless: bool,
    ) {
        // We don't store the # of existing seed accounts now. Thus here we just blindly re-create
        // and re-mint seed accounts here.
        let num_seed_accounts = (num_new_accounts / 1000).clamp(1, 100000);
        let seed_accounts_cache =
            Self::gen_seed_account_cache(reader, num_seed_accounts, is_keyless);

        println!(
            "[{}] Generating {} seed account creation txns, with {} coins.",
            now_fmt!(),
            num_seed_accounts,
            seed_account_balance,
        );
        let bar = get_progress_bar(num_seed_accounts);

        for chunk in seed_accounts_cache
            .accounts
            .iter()
            .collect::<Vec<_>>()
            .chunks(block_size)
        {
            let transactions: Vec<_> = chunk
                .iter()
                .map(|new_account| {
                    let payload = aptos_stdlib::aptos_account_transfer(
                        new_account.authentication_key().account_address(),
                        seed_account_balance,
                    );
                    let builder = self.transaction_factory.payload(payload);
                    let txn = self.root_account.sign_with_transaction_builder(builder);
                    Transaction::UserTransaction(txn)
                })
                .collect();
            bar.inc(transactions.len() as u64 - 1);
            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            }
        }
        bar.finish();
        println!("[{}] done.", now_fmt!());
        self.seed_accounts_cache = Some(seed_accounts_cache);
    }

    /// Generates transactions that creates a set of accounts and fund them from the seed accounts.
    pub fn create_and_fund_accounts(
        &mut self,
        num_existing_accounts: usize,
        num_new_accounts: usize,
        init_account_balance: u64,
        block_size: usize,
        is_keyless: bool,
    ) {
        println!(
            "[{}] Generating {} account creation txns.",
            now_fmt!(),
            num_new_accounts
        );
        let mut generator =
            AccountGenerator::new_for_user_accounts(num_existing_accounts as u64, is_keyless);
        println!("Skipped first {} existing accounts.", num_existing_accounts);

        let bar = get_progress_bar(num_new_accounts);

        for chunk in &(0..num_new_accounts).chunks(block_size) {
            let input: Vec<_> = chunk
                .map(|_| {
                    (
                        self.seed_accounts_cache
                            .as_mut()
                            .unwrap()
                            .get_random_index(),
                        generator.generate(),
                    )
                })
                .collect();
            self.generate_and_send_block(
                self.seed_accounts_cache.as_ref().unwrap(),
                input,
                Arc::new(AtomicUsize::new(0)),
                Arc::new(AtomicUsize::new(0)),
                |(sender_idx, new_account), account_cache| {
                    let sender = &account_cache.accounts[sender_idx];
                    // Use special function to both transfer, and create account resource.
                    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                        ModuleId::new(
                            AccountAddress::SEVEN,
                            ident_str!("benchmark_utils").to_owned(),
                        ),
                        ident_str!("transfer_and_create_account").to_owned(),
                        vec![],
                        vec![
                            bcs::to_bytes(&new_account.authentication_key().account_address())
                                .unwrap(),
                            bcs::to_bytes(&init_account_balance).unwrap(),
                        ],
                    ));
                    let txn = sender
                        .sign_with_transaction_builder(self.transaction_factory.payload(payload));
                    Some(Transaction::UserTransaction(txn))
                },
                |(sender_idx, _)| *sender_idx,
            );
            bar.inc(block_size as u64);
        }
        bar.finish();
        println!("[{}] done.", now_fmt!());
    }

    /// Generates transactions for random pairs of accounts.
    pub fn gen_random_transfer_transactions(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        transactions_per_sender: usize,
    ) {
        for _ in 0..num_blocks {
            let transfer_indices =
                self.get_random_transfer_indices(block_size, transactions_per_sender);
            self.generate_and_send_transfer_block(
                self.main_signer_accounts.as_ref().unwrap(),
                transfer_indices,
            );
        }
    }

    fn get_random_transfer_indices(
        &mut self,
        block_size: usize,
        transactions_per_sender: usize,
    ) -> Vec<(usize, usize)> {
        // TODO: handle when block_size isn't divisible by transactions_per_sender
        (0..(block_size / transactions_per_sender))
            .flat_map(|_| {
                let (sender, receivers) = self
                    .main_signer_accounts
                    .as_mut()
                    .unwrap()
                    .get_random_transfer_batch(transactions_per_sender);
                receivers
                    .into_iter()
                    .map(|receiver| (sender, receiver))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// Generates random P2P transfer transactions, with `1-hotspot_probability` of the accounts used `hotspot_probability` of the time.
    ///
    /// Example 1. If `hotspot_probability` is 0.5, all accounts have the same probability of being sampled.
    ///
    /// Example 2. Say there are 10 accounts A0, ..., A9 and `hotspot_probability` is 0.8.
    /// Whenever we need to sample an account, with probability 0.8 we sample from {A0, A1} uniformly at random;
    /// with probability 0.2 we sample from {A2, ..., A9} uniformly at random.
    pub fn gen_random_transfers_with_hotspot(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        hotspot_probability: f32,
    ) {
        assert!((0.5..1.0).contains(&hotspot_probability));
        for _ in 0..num_blocks {
            let transfer_indices =
                self.get_random_with_hotspot_transfer_indices(block_size, hotspot_probability);
            self.generate_and_send_transfer_block(
                self.main_signer_accounts.as_ref().unwrap(),
                transfer_indices,
            );
        }
    }

    fn get_random_with_hotspot_transfer_indices(
        &mut self,
        block_size: usize,
        hotspot_probability: f32,
    ) -> Vec<(usize, usize)> {
        let num_accounts = self.main_signer_accounts.as_ref().unwrap().len();
        let num_hotspot_accounts =
            ((1.0 - hotspot_probability) * num_accounts as f32).ceil() as usize;
        let mut rng = thread_rng();
        (0..block_size)
            .map(|_| {
                (
                    rand_with_hotspot(&mut rng, num_accounts, num_hotspot_accounts),
                    rand_with_hotspot(&mut rng, num_accounts, num_hotspot_accounts),
                )
            })
            .collect()
    }

    /// 'Conflicting groups of txns' are a type of 'connected groups of txns'.
    /// Here we generate conflicts completely on one particular address (which can be sender or
    /// receiver).
    /// To generate 'n' conflicting groups, we divide the signer accounts into 'n' pools, and
    /// create 'block_size / n' transactions in each group. In each group, we randomly pick
    /// an address from the pool belonging to that group, and create all txns with that address as
    /// a sender or receiver (thereby generating conflicts around that address). In other words,
    /// all txns in a group would have to be executed in serial order.
    /// Finally, after generating such groups of conflicting txns, we shuffle them to generate a
    /// more realistic workload (that is conflicting txns need not always be consecutive).
    fn get_conflicting_grps_transfer_indices(
        rng: &mut StdRng,
        num_signer_accounts: usize,
        block_size: usize,
        conflicting_tx_grps: usize,
        shuffle_indices: bool,
    ) -> Vec<(usize, usize)> {
        let num_accounts_per_grp = num_signer_accounts / conflicting_tx_grps;
        // TODO: handle when block_size isn't divisible by connected_tx_grps; an easy
        //       way to do this is to just generate a few more transactions in the last group
        let num_txns_per_grp = block_size / conflicting_tx_grps;

        if 2 * conflicting_tx_grps >= num_signer_accounts {
            panic!(
                "For the desired workload we want num_signer_accounts ({}) > 2 * num_txns_per_grp ({})",
                num_signer_accounts, num_txns_per_grp);
        } else if conflicting_tx_grps > block_size {
            panic!(
                "connected_tx_grps ({}) > block_size ({}) cannot guarantee at least 1 txn per grp",
                conflicting_tx_grps, block_size
            );
        }

        let mut signer_account_indices: Vec<_> = (0..num_signer_accounts).collect();
        signer_account_indices.shuffle(rng);

        let mut transfer_indices: Vec<_> = (0..conflicting_tx_grps)
            .flat_map(|grp_idx| {
                let accounts_start_idx = grp_idx * num_accounts_per_grp;
                let accounts_end_idx = accounts_start_idx + num_accounts_per_grp - 1;
                let mut accounts_pool: Vec<_> =
                    signer_account_indices[accounts_start_idx..=accounts_end_idx].to_vec();
                let index1 = accounts_pool.pop().unwrap();

                let conflicting_indices: Vec<_> = (0..num_txns_per_grp)
                    .map(|_| {
                        let index2 = accounts_pool[rng.gen_range(0, accounts_pool.len())];
                        if rng.gen::<bool>() {
                            (index1, index2)
                        } else {
                            (index2, index1)
                        }
                    })
                    .collect();
                conflicting_indices
            })
            .collect();
        if shuffle_indices {
            transfer_indices.shuffle(rng);
        }
        transfer_indices
    }

    /// A 'connected transaction group' is a group of transactions where all the transactions are
    /// connected to each other. For now we generate connected groups of txns as conflicting, but
    /// real world workloads can be more complex (and we can generate them as needed in the future).
    pub fn gen_connected_grps_transfer_transactions(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
    ) {
        for _ in 0..num_blocks {
            let num_signer_accounts = self.main_signer_accounts.as_ref().unwrap().accounts.len();
            let rng = &mut self.main_signer_accounts.as_mut().unwrap().rng;
            let transfer_indices: Vec<_> =
                TransactionGenerator::get_conflicting_grps_transfer_indices(
                    rng,
                    num_signer_accounts,
                    block_size,
                    connected_tx_grps,
                    shuffle_connected_txns,
                );
            self.generate_and_send_transfer_block(
                self.main_signer_accounts.as_ref().unwrap(),
                transfer_indices,
            );
        }
    }

    fn generate_and_send_transfer_block(
        &self,
        account_cache: &AccountCache,
        transfer_indices: Vec<(usize, usize)>,
    ) {
        self.generate_and_send_block(
            account_cache,
            transfer_indices,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            |(sender_idx, receiver_idx), account_cache| {
                let txn = account_cache.accounts[sender_idx].sign_with_transaction_builder(
                    self.transaction_factory
                        .transfer(account_cache.accounts[receiver_idx].address(), 1),
                );
                Some(Transaction::UserTransaction(txn))
            },
            |(sender_idx, _)| *sender_idx,
        );
    }

    fn generate_and_send_block<T, F, S>(
        &self,
        account_cache: &AccountCache,
        inputs: Vec<T>,
        phase: Arc<AtomicUsize>,
        last_non_empty_phase: Arc<AtomicUsize>,
        func: F,
        sender_func: S,
    ) -> bool
    where
        T: Send,
        F: Fn(T, &AccountCache) -> Option<Transaction> + Send + Sync,
        S: Fn(&T) -> usize,
    {
        let _timer = TIMER.with_label_values(&["generate_block"]).start_timer();
        let block_size = inputs.len();
        let mut jobs = Vec::new();
        jobs.resize_with(self.num_workers, BTreeMap::new);
        inputs.into_iter().enumerate().for_each(|(i, input)| {
            let sender_idx = sender_func(&input);
            jobs[sender_idx % self.num_workers].insert(i, || func(input, account_cache));
        });
        let (tx, rx) = std::sync::mpsc::channel();
        self.worker_pool.scope(move |scope| {
            for per_worker_jobs in jobs.into_iter() {
                let tx = tx.clone();
                scope.spawn(move |_| {
                    for (index, job) in per_worker_jobs {
                        if let Some(txn) = job() {
                            tx.send((index, txn)).unwrap();
                        }
                    }
                });
            }
        });

        let mut transactions_by_index = HashMap::new();
        while let Ok((index, txn)) = rx.recv() {
            transactions_by_index.insert(index, txn);
        }

        let mut transactions = Vec::new();
        for i in 0..block_size {
            if let Some(txn) = transactions_by_index.get(&i) {
                transactions.push(txn.clone());
            }
        }

        if transactions.is_empty() {
            let val = phase.fetch_add(1, Ordering::Relaxed);
            let last_generated_at = last_non_empty_phase.load(Ordering::Relaxed);
            if val > last_generated_at + 2 {
                info!(
                    "Block generation: no transactions generated in phase {}, and since {}, ending execution",
                    val, last_generated_at
                );
                return true;
            }
            info!(
                "Block generation: no transactions generated in phase {}, moving to next phase",
                val
            );
        } else {
            let val = phase.load(Ordering::Relaxed);
            last_non_empty_phase.fetch_max(val, Ordering::Relaxed);
            info!(
                "Block generation: {} transactions generated in phase {}",
                transactions.len(),
                val
            );
        }

        NUM_TXNS
            .with_label_values(&["generation_done"])
            .inc_by(transactions.len() as u64);

        if let Some(sender) = &self.block_sender {
            sender.send(transactions).unwrap();
        }
        false
    }

    pub fn gen_transfer_transactions(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        transactions_per_sender: usize,
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
        hotspot_probability: Option<f32>,
    ) {
        info!("Starting block generation.");
        info!("block_size={block_size}");
        info!("num_blocks={num_blocks}");
        if connected_tx_grps > 0 {
            info!("block_generation_mode=connected_tx_grps");
            info!("connected_tx_grps={connected_tx_grps}");
            info!("shuffle_connected_txns={shuffle_connected_txns}");
            self.gen_connected_grps_transfer_transactions(
                block_size,
                num_blocks,
                connected_tx_grps,
                shuffle_connected_txns,
            );
        } else if hotspot_probability.is_some() {
            info!("block_generation_mode=sample_from_pool_with_hotspot");
            info!("hotspot_ratio={hotspot_probability:?}");
            self.gen_random_transfers_with_hotspot(
                block_size,
                num_blocks,
                hotspot_probability.unwrap(),
            );
        } else {
            info!("block_generation_mode=default_sample");
            info!("transactions_per_sender={transactions_per_sender}");
            self.gen_random_transfer_transactions(block_size, num_blocks, transactions_per_sender);
        }
    }

    /// Verifies the sequence numbers in storage match what we have locally.
    pub fn verify_sequence_numbers(&self, db: Arc<dyn DbReader>) {
        if self.main_signer_accounts.is_none() {
            println!("Cannot verify account sequence numbers.");
            return;
        }

        let num_accounts_in_cache = self.main_signer_accounts.as_ref().unwrap().len();
        println!(
            "[{}] verify {} account sequence numbers.",
            now_fmt!(),
            num_accounts_in_cache,
        );
        let bar = get_progress_bar(num_accounts_in_cache);
        self.main_signer_accounts
            .as_ref()
            .unwrap()
            .accounts()
            .par_iter()
            .for_each(|account| {
                let address = account.address();
                let db_state_view = db.latest_state_checkpoint_view().unwrap();
                assert_eq!(
                    AccountResource::fetch_move_resource(&db_state_view, &address)
                        .unwrap()
                        .map(|acct| acct.sequence_number)
                        .unwrap_or(0),
                    account.sequence_number()
                );
                bar.inc(1);
            });
        bar.finish();
        println!("[{}] done.", now_fmt!());
    }

    /// Drops the sender to notify the receiving end of the channel.
    pub fn drop_sender(&mut self) {
        self.block_sender.take().unwrap();
    }
}

/// With probability `1-h/n`, pick an integer in [0, h) uniformly at random;
/// with probability `h/n`, pick an integer in [h, n) uniformly at random.
fn rand_with_hotspot<R: Rng>(rng: &mut R, n: usize, h: usize) -> usize {
    let from_hotspot = rng.gen_range(0, n) > h;
    if from_hotspot {
        rng.gen_range(0, h)
    } else {
        rng.gen_range(h, n)
    }
}

#[test]
fn test_get_conflicting_grps_transfer_indices() {
    let mut rng = StdRng::from_entropy();

    fn dfs(node: usize, adj_list: &HashMap<usize, HashSet<usize>>, visited: &mut HashSet<usize>) {
        visited.insert(node);
        for &n in adj_list.get(&node).unwrap() {
            if !visited.contains(&n) {
                dfs(n, adj_list, visited);
            }
        }
    }

    fn get_num_connected_components(adj_list: &HashMap<usize, HashSet<usize>>) -> usize {
        let mut visited = HashSet::new();
        let mut num_connected_components = 0;
        for node in adj_list.keys() {
            if !visited.contains(node) {
                dfs(*node, adj_list, &mut visited);
                num_connected_components += 1;
            }
        }
        num_connected_components
    }

    {
        let block_size = 100;
        let num_signer_accounts = 1000;
        // we check for (i) block_size not divisible by connected_txn_grps (ii) when divisible
        // (iii) when all txns in the block are independent (iv) all txns are dependent
        for connected_txn_grps in [3, block_size / 10, block_size, 1] {
            let transfer_indices = TransactionGenerator::get_conflicting_grps_transfer_indices(
                &mut rng,
                num_signer_accounts,
                block_size,
                connected_txn_grps,
                true,
            );

            let mut adj_list: HashMap<usize, HashSet<usize>> = HashMap::new();
            assert_eq!(
                transfer_indices.len(),
                (block_size / connected_txn_grps) * connected_txn_grps
            );
            for (sender_idx, receiver_idx) in transfer_indices {
                assert!(sender_idx < num_signer_accounts);
                assert!(receiver_idx < num_signer_accounts);
                adj_list.entry(sender_idx).or_default().insert(receiver_idx);
                adj_list.entry(receiver_idx).or_default().insert(sender_idx);
            }

            assert_eq!(get_num_connected_components(&adj_list), connected_txn_grps);
        }
    }
}
