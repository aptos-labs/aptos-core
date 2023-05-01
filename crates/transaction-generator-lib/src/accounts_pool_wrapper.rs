// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{get_account_to_burn_from_pool, TransactionGenerator, TransactionGeneratorCreator};
use aptos_infallible::RwLock;
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use std::sync::Arc;

/// Wrapper that allows inner transaction generator to have unique accounts
/// for all transactions (instead of having 5-20 transactions per account, as default)
/// This is achieved via using accounts from the pool that account creatin can fill,
/// and burning (removing accounts from the pool) them - basically using them only once.
/// (we cannot use more as sequence number is not updated on failure)
pub struct AccountsPoolWrapperGenerator {
    creator: Box<dyn TransactionGenerator>,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
}

impl AccountsPoolWrapperGenerator {
    pub fn new(
        creator: Box<dyn TransactionGenerator>,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    ) -> Self {
        Self {
            creator,
            accounts_pool,
        }
    }
}

impl TransactionGenerator for AccountsPoolWrapperGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let needed = accounts.len() * transactions_per_account;

        let mut accounts_to_burn = get_account_to_burn_from_pool(&self.accounts_pool, needed);
        if accounts_to_burn.is_empty() {
            return Vec::new();
        }
        self.creator
            .generate_transactions(accounts_to_burn.iter_mut().collect(), 1)
    }
}

pub struct AccountsPoolWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
}

impl AccountsPoolWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    ) -> Self {
        Self {
            creator,
            accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for AccountsPoolWrapperCreator {
    fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(AccountsPoolWrapperGenerator::new(
            self.creator.create_transaction_generator(),
            self.accounts_pool.clone(),
        ))
    }
}
