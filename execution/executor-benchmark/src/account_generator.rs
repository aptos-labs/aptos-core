// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_generator::get_progress_bar;
use aptos_sdk::types::LocalAccount;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use std::{collections::VecDeque, sync::mpsc};

type Seed = [u8; 32];

pub struct AccountGenerator {
    receiver: mpsc::Receiver<LocalAccount>,
}

impl AccountGenerator {
    const MAX_ACCOUNT_GEN_PER_RNG: u64 = 40000;
    const SEED_ACCOUNTS_ROOT_SEED: u64 = u64::MAX;
    const USER_ACCOUNTS_ROOT_SEED: u64 = 0;

    pub fn new_for_seed_accounts(is_keyless: bool) -> Self {
        Self::new(Self::SEED_ACCOUNTS_ROOT_SEED, 0, is_keyless)
    }

    pub fn new_for_user_accounts(num_to_skip: u64, is_keyless: bool) -> Self {
        Self::new(Self::USER_ACCOUNTS_ROOT_SEED, num_to_skip, is_keyless)
    }

    fn new(root_seed: u64, num_to_skip: u64, is_keyless: bool) -> Self {
        let mut root_rng = StdRng::seed_from_u64(root_seed);
        let num_rngs_to_skip = num_to_skip / Self::MAX_ACCOUNT_GEN_PER_RNG;
        for _ in 0..num_rngs_to_skip {
            root_rng.next_u64();
        }
        let active_rng_to_skip = num_to_skip % Self::MAX_ACCOUNT_GEN_PER_RNG;
        let mut active_rng_quota = Self::MAX_ACCOUNT_GEN_PER_RNG - active_rng_to_skip;
        let mut active_rng = StdRng::seed_from_u64(root_rng.next_u64());
        for _ in 0..active_rng_to_skip {
            LocalAccount::generate_for_testing(&mut active_rng, is_keyless);
        }
        let (sender, receiver) = mpsc::sync_channel(100 /* bound */);

        std::thread::Builder::new()
            .name("account_generator".to_string())
            .spawn(move || {
                while sender
                    .send(LocalAccount::generate_for_testing(
                        &mut active_rng,
                        is_keyless,
                    ))
                    .is_ok()
                {
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
    pub accounts: VecDeque<LocalAccount>,
    pub rng: StdRng,
}

impl AccountCache {
    const SEED: Seed = [1; 32];

    pub fn new(mut generator: AccountGenerator, num_accounts: usize) -> Self {
        let bar = get_progress_bar(num_accounts);
        let accounts = (0..num_accounts)
            .map(|_| {
                let account = generator.generate();
                bar.inc(1);
                account
            })
            .collect();
        bar.finish();
        Self {
            accounts,
            rng: StdRng::from_seed(Self::SEED),
        }
    }

    pub fn split(mut self, index: usize) -> (Vec<LocalAccount>, Vec<LocalAccount>) {
        let other = self.accounts.split_off(index);
        (self.accounts.into(), other.into())
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn accounts(&self) -> &VecDeque<LocalAccount> {
        &self.accounts
    }

    pub fn get_random(&mut self) -> &mut LocalAccount {
        let index = self.get_random_index();
        &mut self.accounts[index]
    }

    pub fn get_random_index(&mut self) -> usize {
        rand::seq::index::sample(&mut self.rng, self.accounts.len(), 1).index(0)
    }

    pub fn get_random_transfer_batch(&mut self, batch_size: usize) -> (usize, Vec<usize>) {
        let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), batch_size + 1);
        let sender_idx = indices.index(0);
        let receivers = indices.iter().skip(1).collect();
        (sender_idx, receivers)
    }
}
