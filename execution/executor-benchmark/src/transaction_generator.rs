// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::account_generator::{AccountCache, AccountGenerator};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_storage_interface::{state_view::LatestDbStateCheckpointView, DbReader, DbReaderWriter};
use aptos_transaction_generator_lib::TransactionGeneratorCreator;
use aptos_types::{
    account_address::AccountAddress,
    account_config::aptos_test_root_address,
    account_view::AccountView,
    chain_id::ChainId,
    transaction::{Transaction, Version},
};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
#[cfg(test)]
use rand::SeedableRng;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::{HashMap, HashSet};
use std::{
    fs::File,
    io::{Read, Write},
    iter::once,
    path::Path,
    sync::{mpsc, Arc},
};

const META_FILENAME: &str = "metadata.toml";
pub const MAX_ACCOUNTS_INVOLVED_IN_P2P: usize = 1_000_000;

fn get_progress_bar(num_accounts: usize) -> ProgressBar {
    let bar = ProgressBar::new(num_accounts as u64);
    bar.set_style(ProgressStyle::default_bar().template(
        "[{elapsed_precise} {per_sec}] {bar:100.cyan/blue} {percent}% ETA {eta_precise}",
    ));
    bar
}

fn get_sequence_number(address: AccountAddress, reader: Arc<dyn DbReader>) -> u64 {
    let db_state_view = reader.latest_state_checkpoint_view().unwrap();

    let account_state_view = db_state_view.as_account_with_state_view(&address);

    match account_state_view.get_account_resource().unwrap() {
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

    /// Record the number of txns generated.
    version: Version,

    /// Each generated block of transactions are sent to this channel. Using `SyncSender` to make
    /// sure if execution is slow to consume the transactions, we do not run out of memory.
    block_sender: Option<mpsc::SyncSender<Vec<Transaction>>>,

    /// Transaction Factory
    transaction_factory: TransactionFactory,

    /// root account is used across creating and minting.
    root_account: LocalAccount,
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
        let mut accounts = AccountCache::new(generator);
        let bar = get_progress_bar(num_accounts);
        for _ in 0..num_accounts {
            accounts.grow(1);
            bar.inc(1);
        }
        bar.finish();
        accounts
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
                *account.sequence_number_mut() = seq_num;
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
    ) -> AccountCache {
        Self::resync_sequence_numbers(
            reader,
            Self::gen_account_cache(
                AccountGenerator::new_for_user_accounts(num_to_skip as u64),
                num_accounts,
                "user",
            ),
            "user",
        )
    }

    fn gen_seed_account_cache(reader: Arc<dyn DbReader>, num_accounts: usize) -> AccountCache {
        Self::resync_sequence_numbers(
            reader,
            Self::gen_account_cache(
                AccountGenerator::new_for_seed_accounts(),
                num_accounts,
                "seed",
            ),
            "seed",
        )
    }

    pub fn new_with_existing_db<P: AsRef<Path>>(
        db: DbReaderWriter,
        genesis_key: Ed25519PrivateKey,
        block_sender: mpsc::SyncSender<Vec<Transaction>>,
        db_dir: P,
        version: Version,
        num_main_signer_accounts: Option<usize>,
    ) -> Self {
        let num_existing_accounts = TransactionGenerator::read_meta(&db_dir);

        Self {
            seed_accounts_cache: None,
            root_account: LocalAccount::new(
                aptos_test_root_address(),
                genesis_key,
                get_sequence_number(aptos_test_root_address(), db.reader.clone()),
            ),
            main_signer_accounts: num_main_signer_accounts.map(|num_main_signer_accounts| {
                let num_cached_accounts =
                    std::cmp::min(num_existing_accounts, num_main_signer_accounts);
                Self::gen_user_account_cache(db.reader.clone(), num_cached_accounts, 0)
            }),
            num_existing_accounts,
            version,
            block_sender: Some(block_sender),
            transaction_factory: Self::create_transaction_factory(),
        }
    }

    pub fn create_transaction_factory() -> TransactionFactory {
        TransactionFactory::new(ChainId::test())
            .with_transaction_expiration_time(300)
            .with_gas_unit_price(100)
            // TODO(Gas): double check if this is correct
            .with_max_gas_amount(100_000)
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

    pub fn num_existing_accounts(&self) -> usize {
        self.num_existing_accounts
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn run_mint(
        &mut self,
        reader: Arc<dyn DbReader>,
        num_existing_accounts: usize,
        num_new_accounts: usize,
        init_account_balance: u64,
        block_size: usize,
    ) {
        assert!(self.block_sender.is_some());
        // Ensure that seed accounts have enough balance to transfer money to at least 10000 account with
        // balance init_account_balance.
        self.create_seed_accounts(
            reader,
            num_new_accounts,
            block_size,
            init_account_balance * 10_000,
        );
        self.create_and_fund_accounts(
            num_existing_accounts,
            num_new_accounts,
            init_account_balance,
            block_size,
        );
    }

    pub fn run_transfer(
        &mut self,
        block_size: usize,
        num_transfer_blocks: usize,
        transactions_per_sender: usize,
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
    ) {
        assert!(self.block_sender.is_some());
        self.gen_transfer_transactions(
            block_size,
            num_transfer_blocks,
            transactions_per_sender,
            connected_tx_grps,
            shuffle_connected_txns,
        );
    }

    pub fn run_workload(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        mut transaction_generator_creator: Box<dyn TransactionGeneratorCreator>,
        transactions_per_sender: usize,
    ) {
        assert!(self.block_sender.is_some());
        let num_senders_per_block =
            (block_size + transactions_per_sender - 1) / transactions_per_sender;
        let account_pool_size = self.main_signer_accounts.as_ref().unwrap().accounts.len();
        let mut transaction_generator =
            transaction_generator_creator.create_transaction_generator();
        for _ in 0..num_blocks {
            let transactions: Vec<_> = rand::seq::index::sample(
                &mut thread_rng(),
                account_pool_size,
                num_senders_per_block,
            )
            .into_iter()
            .flat_map(|idx| {
                let sender = &mut self.main_signer_accounts.as_mut().unwrap().accounts[idx];
                transaction_generator.generate_transactions(sender, transactions_per_sender)
            })
            .map(Transaction::UserTransaction)
            .chain(once(Transaction::StateCheckpoint(HashValue::random())))
            .collect();
            self.version += transactions.len() as Version;

            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            }
        }
    }

    pub fn create_seed_accounts(
        &mut self,
        reader: Arc<dyn DbReader>,
        num_new_accounts: usize,
        block_size: usize,
        seed_account_balance: u64,
    ) {
        // We don't store the # of existing seed accounts now. Thus here we just blindly re-create
        // and re-mint seed accounts here.
        let num_seed_accounts = (num_new_accounts / 1000).clamp(1, 100000);
        let seed_accounts_cache = Self::gen_seed_account_cache(reader, num_seed_accounts);

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
                    let txn = self.root_account.sign_with_transaction_builder(
                        self.transaction_factory
                            .implicitly_create_user_account_and_transfer(
                                new_account.public_key(),
                                seed_account_balance,
                            ),
                    );
                    Transaction::UserTransaction(txn)
                })
                .chain(once(Transaction::StateCheckpoint(HashValue::random())))
                .collect();
            self.version += transactions.len() as Version;
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
    ) {
        println!(
            "[{}] Generating {} account creation txns.",
            now_fmt!(),
            num_new_accounts
        );
        let mut generator = AccountGenerator::new_for_user_accounts(num_existing_accounts as u64);
        println!("Skipped first {} existing accounts.", num_existing_accounts);

        let bar = get_progress_bar(num_new_accounts);

        for chunk in &(0..num_new_accounts).chunks(block_size) {
            let transactions: Vec<_> = chunk
                .map(|_| {
                    let sender = self.seed_accounts_cache.as_mut().unwrap().get_random();
                    let new_account = generator.generate();
                    let txn = sender.sign_with_transaction_builder(
                        self.transaction_factory
                            .implicitly_create_user_account_and_transfer(
                                new_account.public_key(),
                                init_account_balance,
                            ),
                    );
                    Transaction::UserTransaction(txn)
                })
                .chain(once(Transaction::StateCheckpoint(HashValue::random())))
                .collect();
            self.version += transactions.len() as Version;
            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            }
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
            // TODO: handle when block_size isn't divisible by transactions_per_sender
            let transactions: Vec<_> = (0..(block_size / transactions_per_sender))
                .flat_map(|_| {
                    let (sender, receivers) = self
                        .main_signer_accounts
                        .as_mut()
                        .unwrap()
                        .get_random_transfer_batch(transactions_per_sender);
                    receivers
                        .into_iter()
                        .map(|receiver| {
                            let amount = 1;
                            let txn = sender.sign_with_transaction_builder(
                                self.transaction_factory.transfer(receiver, amount),
                            );
                            Transaction::UserTransaction(txn)
                        })
                        .collect::<Vec<_>>()
                })
                .chain(once(Transaction::StateCheckpoint(HashValue::random())))
                .collect();
            self.version += transactions.len() as Version;

            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            }
        }
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

            let mut transactions: Vec<_> = transfer_indices
                .iter()
                .map(|transfer_idx| {
                    let receiver = self.main_signer_accounts.as_mut().unwrap().accounts
                        [transfer_idx.1]
                        .address();
                    let sender =
                        &mut self.main_signer_accounts.as_mut().unwrap().accounts[transfer_idx.0];
                    let txn = sender.sign_with_transaction_builder(
                        self.transaction_factory.transfer(receiver, 1),
                    );
                    Transaction::UserTransaction(txn)
                })
                .collect();
            transactions.push(Transaction::StateCheckpoint(HashValue::random()));
            self.version += transactions.len() as Version;

            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            }
        }
    }

    pub fn gen_transfer_transactions(
        &mut self,
        block_size: usize,
        num_blocks: usize,
        transactions_per_sender: usize,
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
    ) {
        if connected_tx_grps > 0 {
            self.gen_connected_grps_transfer_transactions(
                block_size,
                num_blocks,
                connected_tx_grps,
                shuffle_connected_txns,
            );
        } else {
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
                let address_account_view = db_state_view.as_account_with_state_view(&address);
                assert_eq!(
                    address_account_view
                        .get_account_resource()
                        .unwrap()
                        .unwrap()
                        .sequence_number(),
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
                adj_list
                    .entry(sender_idx)
                    .or_insert(HashSet::new())
                    .insert(receiver_idx);
                adj_list
                    .entry(receiver_idx)
                    .or_insert(HashSet::new())
                    .insert(sender_idx);
            }

            assert_eq!(get_num_connected_components(&adj_list), connected_txn_grps);
        }
    }
}
