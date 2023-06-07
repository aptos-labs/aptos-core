// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::RwLock;
use aptos_sdk::{move_types::account_address::AccountAddress, types::LocalAccount};
use rand::{rngs::StdRng, seq::SliceRandom, RngCore, SeedableRng};
use std::{
    collections::VecDeque,
    sync::{mpsc, Arc},
};

type Seed = [u8; 32];

pub struct AccountGenerator {
    receiver: mpsc::Receiver<LocalAccount>,
}

impl AccountGenerator {
    const MAX_ACCOUNT_GEN_PER_RNG: u64 = 40000;
    const SEED_ACCOUNTS_ROOT_SEED: u64 = u64::max_value();
    const USER_ACCOUNTS_ROOT_SEED: u64 = 0;

    pub fn new_for_seed_accounts() -> Self {
        Self::new(Self::SEED_ACCOUNTS_ROOT_SEED, 0)
    }

    pub fn new_for_user_accounts(num_to_skip: u64) -> Self {
        Self::new(Self::USER_ACCOUNTS_ROOT_SEED, num_to_skip)
    }

    fn new(root_seed: u64, num_to_skip: u64) -> Self {
        let mut root_rng = StdRng::seed_from_u64(root_seed);
        let num_rngs_to_skip = num_to_skip / Self::MAX_ACCOUNT_GEN_PER_RNG;
        for _ in 0..num_rngs_to_skip {
            root_rng.next_u64();
        }
        let active_rng_to_skip = num_to_skip % Self::MAX_ACCOUNT_GEN_PER_RNG;
        let mut active_rng_quota = Self::MAX_ACCOUNT_GEN_PER_RNG - active_rng_to_skip;
        let mut active_rng = StdRng::seed_from_u64(root_rng.next_u64());
        for _ in 0..active_rng_to_skip {
            LocalAccount::generate(&mut active_rng);
        }
        let (sender, receiver) = mpsc::sync_channel(100 /* bound */);

        std::thread::Builder::new()
            .name("account_generator".to_string())
            .spawn(move || {
                while sender.send(LocalAccount::generate(&mut active_rng)).is_ok() {
                    active_rng_quota -= 1;
                    if active_rng_quota == 0 {
                        active_rng = StdRng::seed_from_u64(root_rng.next_u64());
                        active_rng_quota = Self::MAX_ACCOUNT_GEN_PER_RNG;
                    }
                }
            })
            .expect("Failed to spawn transaction generator thread.");

        Self { receiver }
    }

    pub fn generate(&mut self) -> LocalAccount {
        self.receiver.recv().unwrap()
    }
}

pub trait RandomAccountGenerator {
    // Returns the number of accounts in the cache.
    fn len(&self) -> usize;
    // Returns a random account from the cache.
    fn get_random(&mut self) -> Arc<RwLock<LocalAccount>>;

    // Returns a random sender and a vector of random receivers.
    fn get_random_transfer_batch(
        &mut self,
        batch_size: usize,
    ) -> (Arc<RwLock<LocalAccount>>, Vec<AccountAddress>);
}

pub struct AccountCache {
    generator: AccountGenerator,
    pub accounts: VecDeque<Arc<RwLock<LocalAccount>>>,
    pub rng: StdRng,
}

impl AccountCache {
    const SEED: Seed = [1; 32];

    pub fn new(generator: AccountGenerator) -> Self {
        Self {
            generator,
            accounts: VecDeque::new(),
            rng: StdRng::from_seed(Self::SEED),
        }
    }

    pub fn split(mut self, index: usize) -> (Vec<LocalAccount>, Vec<LocalAccount>) {
        let other = self.accounts.split_off(index);
        let accounts: Vec<LocalAccount> = self
            .accounts
            .into_iter()
            .map(|a| Arc::try_unwrap(a).unwrap().into_inner())
            .collect();
        let other: Vec<LocalAccount> = other
            .into_iter()
            .map(|a| Arc::try_unwrap(a).unwrap().into_inner())
            .collect();
        (accounts, other)
    }

    pub fn accounts(&self) -> &VecDeque<Arc<RwLock<LocalAccount>>> {
        &self.accounts
    }

    pub fn grow(&mut self, n: usize) {
        let accounts: Vec<_> = (0..n)
            .map(|_| Arc::new(RwLock::new(self.generator.generate())))
            .collect();
        self.accounts.extend(accounts);
    }
}

impl RandomAccountGenerator for &mut AccountCache {
    fn get_random_transfer_batch(
        &mut self,
        batch_size: usize,
    ) -> (Arc<RwLock<LocalAccount>>, Vec<AccountAddress>) {
        let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), batch_size + 1);
        let sender_idx = indices.index(0);
        let receivers = indices
            .iter()
            .skip(1)
            .map(|i| self.accounts[i].read().address())
            .collect();
        let sender = self.accounts[sender_idx].clone();

        (sender, receivers)
    }

    fn get_random(&mut self) -> Arc<RwLock<LocalAccount>> {
        let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), 1);
        let index = indices.index(0);

        self.accounts[index].clone()
    }

    fn len(&self) -> usize {
        self.accounts.len()
    }
}

pub struct NoConflictsAccountCache {
    pub accounts: VecDeque<Arc<RwLock<LocalAccount>>>,
}

impl NoConflictsAccountCache {
    const SEED: Seed = [1; 32];

    pub fn new(account_cache: &AccountCache) -> Self {
        let mut rng = StdRng::from_seed(Self::SEED);
        let mut accounts = account_cache.accounts.clone();
        accounts.make_contiguous().shuffle(&mut rng);
        Self { accounts }
    }
}

impl RandomAccountGenerator for NoConflictsAccountCache {
    fn len(&self) -> usize {
        self.accounts.len()
    }

    fn get_random(&mut self) -> Arc<RwLock<LocalAccount>> {
        // Since the accounts are already shuffled, we can just pop the first one.
        self.accounts.pop_front().unwrap()
    }

    fn get_random_transfer_batch(
        &mut self,
        batch_size: usize,
    ) -> (Arc<RwLock<LocalAccount>>, Vec<AccountAddress>) {
        let sender = self
            .accounts
            .pop_front()
            .expect("Not enough accounts to create non-conflicting transactions");
        let mut receivers = Vec::new();
        for _ in 0..batch_size {
            receivers.push(
                self.accounts
                    .pop_front()
                    .expect("Not enough accounts to create non-conflicting transactions")
                    .read()
                    .address(),
            );
        }
        (sender, receivers)
    }
}
