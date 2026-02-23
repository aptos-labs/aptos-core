// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::transaction_generator::get_progress_bar;
use aptos_sdk::types::LocalAccount;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{collections::VecDeque, sync::mpsc};

type Seed = [u8; 32];

pub struct AccountGenerator {
    receiver: mpsc::Receiver<LocalAccount>,
}

impl AccountGenerator {
    pub(crate) const MAX_ACCOUNT_GEN_PER_RNG: u64 = 40000;
    pub(crate) const SEED_ACCOUNTS_ROOT_SEED: u64 = u64::MAX;
    pub(crate) const USER_ACCOUNTS_ROOT_SEED: u64 = 0;

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

    /// Generates `num_accounts` accounts in parallel, producing the exact same
    /// accounts as the sequential `AccountGenerator` + `AccountCache::new` path.
    ///
    /// Each 40k-account chunk uses an independent RNG, so chunks are generated
    /// concurrently via rayon.
    pub fn generate_parallel(
        root_seed: u64,
        num_to_skip: u64,
        num_accounts: usize,
        is_keyless: bool,
    ) -> Self {
        let bar = get_progress_bar(num_accounts);

        // Walk the root RNG forward to the first chunk we need.
        let mut root_rng = StdRng::seed_from_u64(root_seed);
        let num_rngs_to_skip = num_to_skip / AccountGenerator::MAX_ACCOUNT_GEN_PER_RNG;
        for _ in 0..num_rngs_to_skip {
            root_rng.next_u64();
        }
        let first_chunk_skip = num_to_skip % AccountGenerator::MAX_ACCOUNT_GEN_PER_RNG;

        // Build work items: (rng_seed, accounts_to_skip, accounts_to_generate).
        let mut work_items: Vec<(u64, u64, usize)> = Vec::new();
        let mut remaining = num_accounts;

        // First chunk may be partial (some accounts already skipped within it).
        let first_seed = root_rng.next_u64();
        let first_capacity =
            (AccountGenerator::MAX_ACCOUNT_GEN_PER_RNG - first_chunk_skip) as usize;
        let first_count = remaining.min(first_capacity);
        work_items.push((first_seed, first_chunk_skip, first_count));
        remaining -= first_count;

        while remaining > 0 {
            let seed = root_rng.next_u64();
            let count = remaining.min(AccountGenerator::MAX_ACCOUNT_GEN_PER_RNG as usize);
            work_items.push((seed, 0, count));
            remaining -= count;
        }

        // Generate all chunks in parallel — one rayon task per 40k chunk.
        let accounts: Vec<LocalAccount> = work_items
            .into_par_iter()
            .flat_map(|(seed, skip, count)| {
                let mut rng = StdRng::seed_from_u64(seed);
                for _ in 0..skip {
                    LocalAccount::generate_for_testing(&mut rng, is_keyless);
                }
                let chunk: Vec<LocalAccount> = (0..count)
                    .map(|_| {
                        let account = LocalAccount::generate_for_testing(&mut rng, is_keyless);
                        bar.inc(1);
                        account
                    })
                    .collect();
                chunk
            })
            .collect();

        bar.finish();

        Self {
            accounts: VecDeque::from(accounts),
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
