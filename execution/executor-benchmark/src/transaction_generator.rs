// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_generator::{AccountCache, AccountGenerator};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_types::{
    account_config::aptos_root_address,
    account_view::AccountView,
    chain_id::ChainId,
    transaction::{Transaction, Version},
};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    iter::once,
    path::Path,
    sync::{mpsc, Arc},
};
use storage_interface::{state_view::LatestDbStateCheckpointView, DbReader};

const META_FILENAME: &str = "metadata.toml";
const MAX_ACCOUNTS_INVOLVED_IN_P2P: usize = 1_000_000;

fn get_progress_bar(num_accounts: usize) -> ProgressBar {
    let bar = ProgressBar::new(num_accounts as u64);
    bar.set_style(ProgressStyle::default_bar().template(
        "[{elapsed_precise} {per_sec}] {bar:100.cyan/blue} {percent}% ETA {eta_precise}",
    ));
    bar
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
    accounts_cache: Option<AccountCache>,

    /// The current state of seed accounts. The purpose of the seed accounts to parallelize the
    /// account creation and minting process so that they are not blocked on sequence number of
    /// a single root account.
    seed_accounts_cache: Option<AccountCache>,

    /// Total number of accounts in the DB
    num_accounts: usize,

    /// Record the number of txns generated.
    version: Version,

    /// Each generated block of transactions are sent to this channel. Using `SyncSender` to make
    /// sure if execution is slow to consume the transactions, we do not run out of memory.
    block_sender: Option<mpsc::SyncSender<Vec<Transaction>>>,

    /// Transaction Factory
    transaction_factory: TransactionFactory,

    /// root account is used across creating and minting.
    root_account: Option<LocalAccount>,
}

impl TransactionGenerator {
    pub fn new(genesis_key: Ed25519PrivateKey, num_accounts: usize) -> Self {
        Self::new_impl(genesis_key, num_accounts, None)
    }

    pub fn new_with_sender(
        genesis_key: Ed25519PrivateKey,
        num_accounts: usize,
        block_sender: mpsc::SyncSender<Vec<Transaction>>,
    ) -> Self {
        Self::new_impl(genesis_key, num_accounts, Some(block_sender))
    }

    fn new_impl(
        genesis_key: Ed25519PrivateKey,
        num_accounts: usize,
        block_sender: Option<mpsc::SyncSender<Vec<Transaction>>>,
    ) -> Self {
        Self {
            seed_accounts_cache: None,
            root_account: Some(LocalAccount::new(aptos_root_address(), genesis_key, 0)),
            accounts_cache: None,
            num_accounts,
            version: 0,
            block_sender,
            transaction_factory: Self::create_transaction_factory(),
        }
    }

    fn gen_account_cache(num_accounts: usize, seed_account: bool) -> AccountCache {
        let (name, generator) = if seed_account {
            ("seed", AccountGenerator::new_for_seed_accounts())
        } else {
            ("user", AccountGenerator::new_for_user_accounts())
        };

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
        println!("[{}] done.", now_fmt!());

        accounts
    }

    pub fn new_with_existing_db<P: AsRef<Path>>(
        block_sender: mpsc::SyncSender<Vec<Transaction>>,
        db_dir: P,
        version: Version,
    ) -> Self {
        let path = db_dir.as_ref().join(META_FILENAME);
        let mut file = File::open(&path).unwrap();
        let mut contents = vec![];
        file.read_to_end(&mut contents).unwrap();
        let test_case: TestCase = toml::from_slice(&contents).expect("Must exist.");
        let TestCase::P2p(P2pTestCase { num_accounts }) = test_case;

        let num_cached_accounts = std::cmp::min(num_accounts, MAX_ACCOUNTS_INVOLVED_IN_P2P);
        let accounts_cache = Some(Self::gen_account_cache(num_cached_accounts, false));

        Self {
            seed_accounts_cache: None,
            root_account: None,
            accounts_cache,
            num_accounts,
            version,
            block_sender: Some(block_sender),
            transaction_factory: Self::create_transaction_factory(),
        }
    }

    fn create_transaction_factory() -> TransactionFactory {
        TransactionFactory::new(ChainId::test())
            .with_transaction_expiration_time(300)
            .with_gas_unit_price(1)
            .with_max_gas_amount(1000)
    }

    // Write metadata
    pub fn write_meta<P: AsRef<Path>>(self, path: &P) {
        let metadata = TestCase::P2p(P2pTestCase {
            num_accounts: self.num_accounts,
        });
        let serialized = toml::to_vec(&metadata).unwrap();
        let meta_file = path.as_ref().join(META_FILENAME);
        let mut file = File::create(meta_file).unwrap();
        file.write_all(&serialized).unwrap();
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn run_mint(&mut self, init_account_balance: u64, block_size: usize) {
        assert!(self.block_sender.is_some());
        // Ensure that seed accounts have enough balance to transfer money to at least 1000 account with
        // balance init_account_balance.
        self.create_seed_accounts(block_size, init_account_balance * 1_000_000_000);
        self.create_and_fund_accounts(init_account_balance, block_size);
    }

    pub fn run_transfer(&mut self, block_size: usize, num_transfer_blocks: usize) {
        assert!(self.block_sender.is_some());
        self.gen_transfer_transactions(block_size, num_transfer_blocks);
    }

    pub fn create_seed_accounts(
        &mut self,
        block_size: usize,
        seed_account_balance: u64,
    ) -> Vec<Vec<Transaction>> {
        let mut txn_block = Vec::new();

        let num_seed_accounts = (self.num_accounts / 1000).max(1).min(1000);
        let seed_accounts_cache = Self::gen_account_cache(num_seed_accounts, true);

        println!(
            "[{}] Generating {} seed account creation txns.",
            now_fmt!(),
            num_seed_accounts,
        );
        let bar = get_progress_bar(num_seed_accounts);

        for chunk in seed_accounts_cache
            .accounts
            .iter()
            .collect::<Vec<_>>()
            .chunks(block_size / 2)
        {
            let transactions: Vec<_> = chunk
                .iter()
                .flat_map(|account| {
                    let create = self
                        .root_account
                        .as_mut()
                        .unwrap()
                        .sign_with_transaction_builder(
                            self.transaction_factory
                                .create_user_account(account.public_key()),
                        );
                    let mint = self
                        .root_account
                        .as_mut()
                        .unwrap()
                        .sign_with_transaction_builder(
                            self.transaction_factory
                                .mint(account.address(), seed_account_balance),
                        );
                    vec![create, mint]
                })
                .map(Transaction::UserTransaction)
                .chain(once(Transaction::StateCheckpoint))
                .collect();
            self.version += transactions.len() as Version;
            bar.inc(transactions.len() as u64 - 1);
            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            } else {
                txn_block.push(transactions);
            }
        }
        bar.finish();
        println!("[{}] done.", now_fmt!());
        self.seed_accounts_cache = Some(seed_accounts_cache);

        txn_block
    }

    /// Generates transactions that creates a set of accounts and fund them from the seed accounts.
    pub fn create_and_fund_accounts(
        &mut self,
        init_account_balance: u64,
        block_size: usize,
    ) -> Vec<Vec<Transaction>> {
        let mut txn_block = vec![];

        println!(
            "[{}] Generating {} account creation txns.",
            now_fmt!(),
            self.num_accounts,
        );
        let mut generator = AccountGenerator::new_for_user_accounts();
        let bar = get_progress_bar(self.num_accounts);

        for chunk in &(0..self.num_accounts).chunks(block_size) {
            let transactions: Vec<_> = chunk
                .map(|_| {
                    self.seed_accounts_cache
                        .as_mut()
                        .unwrap()
                        .get_random()
                        .sign_with_transaction_builder(
                            self.transaction_factory.create_and_fund_user_account(
                                generator.generate().public_key(),
                                init_account_balance,
                            ),
                        )
                })
                .map(Transaction::UserTransaction)
                .chain(once(Transaction::StateCheckpoint))
                .collect();
            self.version += transactions.len() as Version;
            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            } else {
                txn_block.push(transactions);
            }
            bar.inc(block_size as u64);
        }
        bar.finish();
        println!("[{}] done.", now_fmt!());

        txn_block
    }

    /// Generates transactions for random pairs of accounts.
    pub fn gen_transfer_transactions(
        &mut self,
        block_size: usize,
        num_blocks: usize,
    ) -> Vec<Vec<Transaction>> {
        let mut txn_block = vec![];

        for _ in 0..num_blocks {
            let transactions: Vec<_> = (0..block_size)
                .into_iter()
                .map(|_| {
                    let (sender, receiver) =
                        self.accounts_cache.as_mut().unwrap().get_random_transfer();
                    sender.sign_with_transaction_builder(
                        self.transaction_factory.transfer(receiver, 1),
                    )
                })
                .map(Transaction::UserTransaction)
                .chain(once(Transaction::StateCheckpoint))
                .collect();
            self.version += transactions.len() as Version;

            if let Some(sender) = &self.block_sender {
                sender.send(transactions).unwrap();
            } else {
                txn_block.push(transactions);
            }
        }
        txn_block
    }

    /// Verifies the sequence numbers in storage match what we have locally.
    pub fn verify_sequence_numbers(&self, db: Arc<dyn DbReader>) {
        if self.accounts_cache.is_none() {
            println!("Cannot verify account sequence numbers.");
            return;
        }

        let num_accounts_in_cache = self.accounts_cache.as_ref().unwrap().len();
        println!(
            "[{}] verify {} account sequence numbers.",
            now_fmt!(),
            num_accounts_in_cache,
        );
        let bar = get_progress_bar(num_accounts_in_cache);
        self.accounts_cache
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
