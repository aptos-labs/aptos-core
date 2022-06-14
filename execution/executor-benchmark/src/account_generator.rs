// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::{move_types::account_address::AccountAddress, types::LocalAccount};
use rand::{rngs::StdRng, SeedableRng};
use std::{collections::VecDeque, sync::mpsc};

type Seed = [u8; 32];

pub struct AccountGenerator {
    receiver: mpsc::Receiver<LocalAccount>,
}

impl AccountGenerator {
    const USER_ACCOUNTS_SEED: Seed = [1; 32];
    const SEED_ACCOUNTS_SEED: Seed = [2; 32];

    pub fn new_for_seed_accounts() -> Self {
        Self::new(Self::SEED_ACCOUNTS_SEED)
    }

    pub fn new_for_user_accounts() -> Self {
        Self::new(Self::USER_ACCOUNTS_SEED)
    }

    fn new(seed: Seed) -> Self {
        let (sender, receiver) = mpsc::sync_channel(100 /* bound */);

        std::thread::Builder::new()
            .name("account_generator".to_string())
            .spawn(move || {
                let mut rng = StdRng::from_seed(seed);
                while sender.send(LocalAccount::generate(&mut rng)).is_ok() {}
            })
            .expect("Failed to spawn transaction generator thread.");

        Self { receiver }
    }

    pub fn generate(&mut self) -> LocalAccount {
        self.receiver.recv().unwrap()
    }
}

pub struct AccountCache {
    generator: AccountGenerator,
    pub accounts: VecDeque<LocalAccount>,
    rng: StdRng,
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

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn accounts(&self) -> &VecDeque<LocalAccount> {
        &self.accounts
    }

    pub fn grow(&mut self, n: usize) {
        let accounts: Vec<_> = (0..n).map(|_| self.generator.generate()).collect();
        self.accounts.extend(accounts);
    }

    pub fn get_random(&mut self) -> &mut LocalAccount {
        let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), 1);
        let index = indices.index(0);

        &mut self.accounts[index]
    }

    pub fn get_random_transfer(&mut self) -> (&mut LocalAccount, AccountAddress) {
        let indices = rand::seq::index::sample(&mut self.rng, self.accounts.len(), 2);
        let sender_idx = indices.index(0);
        let receiver_idx = indices.index(1);

        let receiver = self.accounts[receiver_idx].address();
        let sender = &mut self.accounts[sender_idx];

        (sender, receiver)
    }
}
