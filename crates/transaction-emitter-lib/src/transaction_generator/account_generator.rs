// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_logger::sample::Sampling;
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::prelude::StdRng;
use rand::Rng;
use rand_core::{OsRng, SeedableRng};
use std::sync::Arc;
use std::time::Duration;

pub struct AccountGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    add_created_accounts_to_pool: bool,
    max_working_set: usize,
    gas_price: u64,
}

impl AccountGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        add_created_accounts_to_pool: bool,
        max_working_set: usize,
        gas_price: u64,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            all_addresses,
            add_created_accounts_to_pool,
            max_working_set,
            gas_price,
        }
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        to: AccountAddress,
        txn_factory: &TransactionFactory,
        gas_price: u64,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            txn_factory
                .payload(aptos_stdlib::aptos_account_create_account(to))
                .gas_unit_price(gas_price),
        )
    }
}

impl TransactionGenerator for AccountGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        let mut new_accounts = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            for _ in 0..transactions_per_account {
                let receiver = LocalAccount::generate(&mut self.rng).address();
                let request =
                    self.gen_single_txn(account, receiver, &self.txn_factory, self.gas_price);
                requests.push(request);
                new_accounts.push(receiver);
            }
        }

        if self.add_created_accounts_to_pool {
            let mut current = self.all_addresses.write();
            if current.len() < self.max_working_set {
                current.append(&mut new_accounts);
                sample!(
                    SampleRate::Duration(Duration::from_secs(120)),
                    info!("Accounts working set increased to {}", current.len())
                );
            } else {
                let start = self.rng.gen_range(0, current.len() - new_accounts.len());
                current[start..start + new_accounts.len()].copy_from_slice(&new_accounts);

                sample!(
                    SampleRate::Duration(Duration::from_secs(120)),
                    info!(
                        "Already at limit {} > {}, so exchanged accounts in working set",
                        current.len(),
                        self.max_working_set
                    )
                );
            }
        }
        requests
    }
}

pub struct AccountGeneratorCreator {
    txn_factory: TransactionFactory,
    all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
    add_created_accounts_to_pool: bool,
    max_working_set: usize,
    gas_price: u64,
}

impl AccountGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        all_addresses: Arc<RwLock<Vec<AccountAddress>>>,
        add_created_accounts_to_pool: bool,
        max_working_set: usize,
        gas_price: u64,
    ) -> Self {
        Self {
            txn_factory,
            all_addresses,
            add_created_accounts_to_pool,
            max_working_set,
            gas_price,
        }
    }
}

impl TransactionGeneratorCreator for AccountGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(AccountGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            self.txn_factory.clone(),
            self.all_addresses.clone(),
            self.add_created_accounts_to_pool,
            self.max_working_set,
            self.gas_price,
        ))
    }
}
