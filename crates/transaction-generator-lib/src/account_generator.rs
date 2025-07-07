// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    create_account_transaction, ObjectPool, ReplayProtectionType, TransactionGenerator,
    TransactionGeneratorCreator,
};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

pub struct AccountGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    addresses_pool: Option<Arc<ObjectPool<AccountAddress>>>,
    accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    max_working_set: usize,
    creation_balance: u64,
    replay_protection_type: ReplayProtectionType,
}

impl AccountGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        addresses_pool: Option<Arc<ObjectPool<AccountAddress>>>,
        accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
        max_working_set: usize,
        creation_balance: u64,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            addresses_pool,
            accounts_pool,
            max_working_set,
            creation_balance,
            replay_protection_type,
        }
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
                self.replay_protection_type,
            );
            requests.push(request);
            new_accounts.push(receiver);
            new_account_addresses.push(receiver_address);
        }

        if let Some(addresses_pool) = &self.addresses_pool {
            addresses_pool.add_to_pool_bounded(
                new_account_addresses,
                self.max_working_set,
                &mut self.rng,
            );
        }
        if let Some(accounts_pool) = &self.accounts_pool {
            accounts_pool.add_to_pool_bounded(new_accounts, self.max_working_set, &mut self.rng);
        }

        requests
    }
}

pub struct AccountGeneratorCreator {
    txn_factory: TransactionFactory,
    addresses_pool: Option<Arc<ObjectPool<AccountAddress>>>,
    accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    max_working_set: usize,
    creation_balance: u64,
    replay_protection_type: ReplayProtectionType,
}

impl AccountGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        addresses_pool: Option<Arc<ObjectPool<AccountAddress>>>,
        accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
        max_working_set: usize,
        creation_balance: u64,
        replay_protection_type: ReplayProtectionType,
    ) -> Self {
        Self {
            txn_factory,
            addresses_pool,
            accounts_pool,
            max_working_set,
            creation_balance,
            replay_protection_type,
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
            self.max_working_set,
            self.creation_balance,
            self.replay_protection_type,
        ))
    }
}
