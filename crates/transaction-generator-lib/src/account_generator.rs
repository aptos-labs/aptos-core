// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{create_account_transaction, TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{sync::Arc, time::Duration};

pub struct AccountGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    addresses_pool: Arc<RwLock<Vec<AccountAddress>>>,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    add_created_accounts_to_pool: bool,
    max_working_set: usize,
    creation_balance: u64,
}

impl AccountGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        addresses_pool: Arc<RwLock<Vec<AccountAddress>>>,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
        add_created_accounts_to_pool: bool,
        max_working_set: usize,
        creation_balance: u64,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            addresses_pool,
            accounts_pool,
            add_created_accounts_to_pool,
            max_working_set,
            creation_balance,
        }
    }
}

fn add_to_sized_pool<T>(
    pool: &RwLock<Vec<T>>,
    mut addition: Vec<T>,
    max_working_set: usize,
    rng: &mut StdRng,
) {
    let mut current = pool.write();
    if current.len() < max_working_set {
        current.append(&mut addition);
        sample!(
            SampleRate::Duration(Duration::from_secs(120)),
            info!("Accounts working set increased to {}", current.len())
        );
    } else {
        let start = rng.gen_range(0, current.len() - addition.len());
        current[start..start + addition.len()].swap_with_slice(&mut addition);

        sample!(
            SampleRate::Duration(Duration::from_secs(120)),
            info!(
                "Already at limit {} > {}, so exchanged accounts in working set",
                current.len(),
                max_working_set
            )
        );
    }
}

impl TransactionGenerator for AccountGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        let mut new_accounts = Vec::with_capacity(num_to_create);
        let mut new_account_addresses = Vec::with_capacity(num_to_create);
        for _ in 0..num_to_create {
            let receiver = LocalAccount::generate(&mut self.rng);
            let receiver_address = receiver.address();
            let request = create_account_transaction(
                account,
                receiver_address,
                &self.txn_factory,
                self.creation_balance,
            );
            requests.push(request);
            new_accounts.push(receiver);
            new_account_addresses.push(receiver_address);
        }

        if self.add_created_accounts_to_pool {
            add_to_sized_pool(
                self.accounts_pool.as_ref(),
                new_accounts,
                self.max_working_set,
                &mut self.rng,
            );
            add_to_sized_pool(
                self.addresses_pool.as_ref(),
                new_account_addresses,
                self.max_working_set,
                &mut self.rng,
            );
        }
        requests
    }
}

pub struct AccountGeneratorCreator {
    txn_factory: TransactionFactory,
    addresses_pool: Arc<RwLock<Vec<AccountAddress>>>,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    add_created_accounts_to_pool: bool,
    max_working_set: usize,
    creation_balance: u64,
}

impl AccountGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        addresses_pool: Arc<RwLock<Vec<AccountAddress>>>,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
        add_created_accounts_to_pool: bool,
        max_working_set: usize,
        creation_balance: u64,
    ) -> Self {
        if add_created_accounts_to_pool {
            addresses_pool.write().reserve(max_working_set);
            accounts_pool.write().reserve(max_working_set);
        }

        Self {
            txn_factory,
            addresses_pool,
            accounts_pool,
            add_created_accounts_to_pool,
            max_working_set,
            creation_balance,
        }
    }
}

impl TransactionGeneratorCreator for AccountGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(AccountGenerator::new(
            StdRng::from_entropy(),
            self.txn_factory.clone(),
            self.addresses_pool.clone(),
            self.accounts_pool.clone(),
            self.add_created_accounts_to_pool,
            self.max_working_set,
            self.creation_balance,
        ))
    }
}
