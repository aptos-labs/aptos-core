// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use rand::prelude::StdRng;
use rand_core::{OsRng, SeedableRng};

use rand::Rng;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, Debug)]
pub struct AccountGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
}

impl AccountGenerator {
    pub fn new(rng: StdRng, txn_factory: TransactionFactory) -> Self {
        Self { rng, txn_factory }
    }

    fn gen_single_txn(
        &self,
        from: &mut LocalAccount,
        to: AccountAddress,
        _num_coins: u64,
        txn_factory: &TransactionFactory,
        gas_price: u64,
    ) -> SignedTransaction {
        from.sign_with_transaction_builder(
            txn_factory
                .payload(aptos_stdlib::account_create_account(to))
                .gas_unit_price(gas_price),
        )
    }
}

impl TransactionGenerator for AccountGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
        _all_addresses: Arc<Vec<AccountAddress>>,
        _invalid_transaction_ratio: usize,
        gas_price: u64,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            for _ in 0..transactions_per_account {
                let receiver = LocalAccount::generate(&mut self.rng).address();
                let request =
                    self.gen_single_txn(account, receiver, 0, &self.txn_factory, gas_price);
                requests.push(request);
            }
        }
        requests
    }
}

#[derive(Debug)]
pub struct AccountGeneratorCreator {
    txn_factory: TransactionFactory,
}

impl AccountGeneratorCreator {
    pub fn new(txn_factory: TransactionFactory) -> Self {
        Self { txn_factory }
    }
}

impl TransactionGeneratorCreator for AccountGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(AccountGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            self.txn_factory.clone(),
        ))
    }
}
