// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::{Mutex, RwLock};
use aptos_sdk::{move_types::account_address::AccountAddress, types::LocalAccount};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use std::{collections::VecDeque, sync::mpsc};

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

pub struct AccountCache {
    generator: Mutex<AccountGenerator>,
    pub accounts: VecDeque<RwLock<LocalAccount>>,
    rng: RwLock<StdRng>,
}

impl AccountCache {
    const SEED: Seed = [1; 32];

    pub fn new(generator: AccountGenerator) -> Self {
        Self {
            generator: Mutex::new(generator),
            accounts: VecDeque::new(),
            rng: RwLock::new(StdRng::from_seed(Self::SEED)),
        }
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn accounts(&self) -> &VecDeque<RwLock<LocalAccount>> {
        &self.accounts
    }

    pub fn grow(&mut self, n: usize) {
        let accounts: Vec<_> = (0..n)
            .map(|_| RwLock::new(self.generator.lock().generate()))
            .collect();
        self.accounts.extend(accounts);
    }

    pub fn get_random(&self) -> &RwLock<LocalAccount> {
        //let x = self.rng.write().next_u64();
        let indices = rand::seq::index::sample(&mut *self.rng.write(), self.accounts.len(), 1);
        let index = indices.index(0);
        &self.accounts[index]
    }

    pub fn get_random_transfer_batch(
        &self,
        batch_size: usize,
    ) -> (&RwLock<LocalAccount>, Vec<AccountAddress>) {
        let indices =
            rand::seq::index::sample(&mut *self.rng.write(), self.accounts.len(), batch_size + 1);
        let sender_idx = indices.index(0);
        let receivers = indices
            .iter()
            .skip(1)
            .map(|i| self.accounts[i].read().address())
            .collect();
        let sender = &self.accounts[sender_idx];

        (sender, receivers)
    }
}
